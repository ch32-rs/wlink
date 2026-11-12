//! Backend for WCH's native Windows driver (`WCHLinkDrv`).
//!
//! A probe bound to this driver is not a WinUSB device: nusb/libusb can still
//! *enumerate* it and read its USB serial number, but cannot open its
//! interface. The driver exposes device interfaces and a vendor IOCTL instead;
//! this module enumerates those interfaces and speaks that protocol.
//!
//! There is no public specification for the driver interface. Both constants
//! below were taken from the driver's observed behaviour, originally proposed
//! in <https://github.com/ch32-rs/wlink/pull/116>.

use std::fmt;

use windows::Win32::{
    Devices::DeviceAndDriverInstallation::*, Foundation::*, Storage::FileSystem::*, System::IO::*,
};
use windows::core::{GUID, HRESULT, PCWSTR};

use super::*;
use crate::Error;

/// Device interface class registered by WCH's Windows driver installer.
const GUID_DEVINTERFACE_WCH_LINK: GUID = GUID::from_values(
    0xF8D5EDCA,
    0xB647,
    0x4E9C,
    [0x9B, 0xD3, 0xA5, 0xBD, 0x23, 0x28, 0xD5, 0x5C],
);
/// Vendor IOCTL on `FILE_DEVICE_UNKNOWN` (0x22), `METHOD_BUFFERED`.
/// The request/response layout is described by [`WCHLinkUSBDevice::command`].
const WCH_LINK_DRIVER_IOCTL: u32 = 0x00223CDC;

/// A USB device descriptor is 18 bytes (`bLength` .. `bNumConfigurations`).
const DEVICE_DESCRIPTOR_LEN: usize = 18;
/// `idVendor` / `idProduct` offsets within that descriptor. The driver has to
/// return at least `DEVICE_IDS_END` bytes of descriptor for them to be valid.
const VID_OFFSET: usize = 8;
const PID_OFFSET: usize = 10;
const DEVICE_IDS_END: usize = PID_OFFSET + core::mem::size_of::<u16>();

/// Bytes the driver puts in front of an IOCTL response: a command word
/// (`[0..4]`, with the endpoint and transfer kind packed into its low bytes)
/// followed by a payload length (`[4..8]`), both native-endian u32s.
const RESPONSE_HEADER_LEN: usize = 8;
const HEADER_COMMAND_OFFSET: usize = 0;
/// On bulk transfers byte 0 of that word is the endpoint index instead, so it
/// gets a separate name from the full command word used by control commands.
const HEADER_ENDPOINT_OFFSET: usize = 0;
const HEADER_TRANSFER_OFFSET: usize = 2;
const HEADER_LENGTH_OFFSET: usize = 4;
// The buffer sizes and write offsets are all derived from these constants;
// pin their invariants so changing one cannot silently shift the layout.
const _: () = assert!(RESPONSE_HEADER_LEN == HEADER_LENGTH_OFFSET + core::mem::size_of::<u32>());
const _: () = assert!(DEVICE_DESCRIPTOR_LEN >= DEVICE_IDS_END);

/// Transfer kinds in byte 2 of the command word.
const CMD_READ_ENDPOINT: u8 = 0x01;
const CMD_WRITE_ENDPOINT: u8 = 0x02;

/// GET_DESCRIPTOR(Device): command word, then the 8-byte USB setup packet.
/// The driver replaces that setup packet with the 18-byte descriptor.
const CMD_GET_DEVICE_DESCRIPTOR: u32 = 0x04;
const SETUP_PACKET_LEN: usize = 8;
const GET_DEVICE_DESCRIPTOR_SETUP: [u8; SETUP_PACKET_LEN] = [
    0x80, // bmRequestType: device-to-host, standard, device
    0x06, // bRequest: GET_DESCRIPTOR
    0x00,
    0x01, // wValue: device descriptor
    0x00,
    0x00, // wIndex
    DEVICE_DESCRIPTOR_LEN as u8,
    0x00, // wLength
];

/// SET_TIMEOUT: command word, then four native-endian u32 timeout values.
/// The driver's expected payload length (0x10) is exactly those 16 bytes.
const CMD_SET_TIMEOUT: u32 = 0x0F;
const TIMEOUT_PARAM_COUNT: usize = 4;
const TIMEOUT_PARAMS_LEN: usize = TIMEOUT_PARAM_COUNT * core::mem::size_of::<u32>();
const TIMEOUT_BUFFER_LEN: usize = RESPONSE_HEADER_LEN + TIMEOUT_PARAMS_LEN;

// Every caller sizes its buffer from the constants above, so an out-of-range
// offset here would be a programming error rather than a driver condition.

fn write_u32_ne(buffer: &mut [u8], offset: usize, value: u32) {
    let end = offset + core::mem::size_of::<u32>();
    buffer[offset..end].copy_from_slice(&value.to_ne_bytes());
}

fn read_u32_ne(buffer: &[u8], offset: usize) -> Result<u32> {
    let end = offset + core::mem::size_of::<u32>();
    let bytes = buffer.get(offset..end).ok_or(Error::InvalidPayloadLength)?;
    Ok(u32::from_ne_bytes(
        bytes.try_into().map_err(|_| Error::InvalidPayloadLength)?,
    ))
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let end = offset + core::mem::size_of::<u16>();
    let field = bytes.get(offset..end).ok_or(Error::InvalidPayloadLength)?;
    Ok(u16::from_le_bytes([field[0], field[1]]))
}

/// Position of the first NUL, or the slice length when there is none.
fn nul_end(path: &[u16]) -> usize {
    path.iter().position(|c| *c == 0).unwrap_or(path.len())
}

struct DeviceInfoSet {
    hdevinfo: HDEVINFO,
}

impl DeviceInfoSet {
    fn new() -> Result<Self> {
        Ok(Self {
            // SAFETY: the GUID is a static, the optional enumerator and hwnd
            // are None, and the flags are the documented combination for an
            // interface-class query. The returned set is owned by `Drop`.
            hdevinfo: unsafe {
                SetupDiGetClassDevsW(
                    Some(&GUID_DEVINTERFACE_WCH_LINK),
                    None,
                    None,
                    DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
                )
            }?,
        })
    }

    /// Returns `None` once the enumeration is exhausted.
    fn enum_interface(&self, index: u32) -> Result<Option<SP_DEVICE_INTERFACE_DATA>> {
        let mut data = SP_DEVICE_INTERFACE_DATA {
            cbSize: core::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };

        // SAFETY: `hdevinfo` is a live set from `new()`, `data` is initialized
        // with its cbSize and passed by exclusive reference, and the GUID is
        // a static that outlives the call.
        match unsafe {
            SetupDiEnumDeviceInterfaces(
                self.hdevinfo,
                None,
                &GUID_DEVINTERFACE_WCH_LINK,
                index,
                &mut data,
            )
        } {
            Ok(()) => Ok(Some(data)),
            Err(e) if e.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Returns the NUL-terminated device interface path.
    ///
    /// The variable-sized `SP_DEVICE_INTERFACE_DETAIL_DATA_W` is kept in a
    /// `Vec<u64>`: that guarantees enough alignment for the struct header on
    /// every target (x86 uses a packed layout, x86_64/aarch64 do not) and,
    /// unlike a manual `alloc`/`dealloc` pair, cannot leak when a `?` returns
    /// early.
    fn device_path(&self, interface: &SP_DEVICE_INTERFACE_DATA) -> Result<Vec<u16>> {
        let mut required_size = 0u32;
        // SAFETY: a null detail buffer with length 0 is the documented size
        // query; the driver only writes to `required_size`.
        match unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                self.hdevinfo,
                interface,
                None,
                0,
                Some(&mut required_size),
                None,
            )
        } {
            Ok(()) => {}
            Err(e) if e.code() == HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0) => {}
            Err(e) => return Err(e.into()),
        }

        let path_offset = core::mem::offset_of!(SP_DEVICE_INTERFACE_DETAIL_DATA_W, DevicePath);
        let path_bytes = (required_size as usize)
            .checked_sub(path_offset)
            .filter(|len| *len >= core::mem::size_of::<u16>())
            .ok_or_else(|| Error::Custom("invalid WCH device interface detail size".into()))?;

        let mut storage = vec![0u64; required_size as usize / 8 + 1];
        let detail = storage
            .as_mut_ptr()
            .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        // SAFETY: `storage` is `Vec<u64>`, so the cast is 8-byte aligned —
        // enough for every target's layout of this struct (x86 uses a packed
        // variant) — and holds at least `required_size` bytes from the size
        // query above. `size_of` gives the architecture-dependent cbSize (6
        // on x86, 8 elsewhere), and the same `interface` is passed back with
        // a buffer of exactly the reported size.
        unsafe {
            (*detail).cbSize = core::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            SetupDiGetDeviceInterfaceDetailW(
                self.hdevinfo,
                interface,
                Some(detail),
                required_size,
                None,
                None,
            )?;
        }

        let mut path = vec![0u16; path_bytes / core::mem::size_of::<u16>()];
        // SAFETY: `path_offset` was subtracted from the driver-reported size,
        // so `storage` has at least `path_bytes` readable bytes from there;
        // `path` is exactly that many bytes long. Both sides are cast to u8,
        // and two distinct Vec allocations cannot overlap.
        unsafe {
            core::ptr::copy_nonoverlapping(
                storage.as_ptr().cast::<u8>().add(path_offset),
                path.as_mut_ptr().cast::<u8>(),
                path.len() * core::mem::size_of::<u16>(),
            );
        }
        // CreateFileW needs a NUL terminator; keep one even if the reported
        // size did not include it.
        let end = nul_end(&path);
        if end == path.len() {
            path.push(0);
        } else {
            path.truncate(end + 1);
        }
        Ok(path)
    }
}

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        // SAFETY: `hdevinfo` was returned by `SetupDiGetClassDevsW` in `new()`
        // and is neither cloned nor closed elsewhere, so this is its only
        // and final use.
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.hdevinfo);
        }
    }
}

/// Extracts `idVendor` / `idProduct` from a USB device descriptor.
///
/// Reading fields by offset avoids relying on the alignment and layout of a
/// packed FFI struct when parsing bytes coming back from the driver.
fn parse_device_ids(descriptor: &[u8]) -> Result<(u16, u16)> {
    Ok((
        read_le_u16(descriptor, VID_OFFSET)?,
        read_le_u16(descriptor, PID_OFFSET)?,
    ))
}

/// The driver normally returns the 8-byte header plus the whole 18-byte
/// descriptor. A short response leaves zeros in the buffer, which would parse
/// as VID/PID 0000:0000 and silently hide the probe instead of reporting a
/// driver problem.
fn check_descriptor_response(bytes_returned: usize) -> Result<()> {
    const MIN_RESPONSE: usize = RESPONSE_HEADER_LEN + DEVICE_IDS_END; // through idProduct
    if bytes_returned < MIN_RESPONSE {
        return Err(Error::Custom(format!(
            "short WCH device descriptor response: {bytes_returned} bytes, \
             expected at least {MIN_RESPONSE}"
        )));
    }
    Ok(())
}

/// Slices the payload out of an IOCTL response. `length` is reported by the
/// driver, so it is bounds-checked instead of being used in arithmetic that
/// could overflow on 32-bit targets.
fn response_payload(buffer: &[u8], length: usize) -> Result<&[u8]> {
    let end = RESPONSE_HEADER_LEN
        .checked_add(length)
        .ok_or(Error::InvalidPayloadLength)?;
    buffer
        .get(RESPONSE_HEADER_LEN..end)
        .ok_or(Error::InvalidPayloadLength)
}

/// The driver numbers endpoints from zero, while USB endpoint addresses start
/// at one. Endpoint 0 is the control endpoint and is never used for bulk I/O.
fn driver_endpoint(ep: u8) -> Result<u8> {
    (ep & 0x7f)
        .checked_sub(1)
        .ok_or_else(|| Error::Custom(format!("invalid bulk endpoint {ep:#04x}")))
}

/// Fills the shared request/response header for a bulk transfer.
fn prepare_endpoint_command(
    buffer: &mut [u8],
    ep: u8,
    transfer: u8,
    payload_len: usize,
) -> Result<()> {
    buffer[HEADER_ENDPOINT_OFFSET] = driver_endpoint(ep)?;
    buffer[HEADER_TRANSFER_OFFSET] = transfer;
    // The header field is a u32; refuse a buffer that cannot be described by
    // it rather than silently truncating the length seen by the driver.
    let length = u32::try_from(payload_len).map_err(|_| Error::InvalidPayloadLength)?;
    write_u32_ne(buffer, HEADER_LENGTH_OFFSET, length);
    Ok(())
}

/// SET_TIMEOUT command: header plus four identical timeout values.
fn timeout_command(timeout: Duration) -> [u8; TIMEOUT_BUFFER_LEN] {
    let ds = timeout.as_millis().min(u32::MAX as u128) as u32;
    let mut buffer = [0u8; TIMEOUT_BUFFER_LEN];
    write_u32_ne(&mut buffer, HEADER_COMMAND_OFFSET, CMD_SET_TIMEOUT);
    write_u32_ne(&mut buffer, HEADER_LENGTH_OFFSET, TIMEOUT_PARAMS_LEN as u32);
    for offset in (RESPONSE_HEADER_LEN..TIMEOUT_BUFFER_LEN).step_by(core::mem::size_of::<u32>()) {
        write_u32_ne(&mut buffer, offset, ds);
    }
    buffer
}

fn display_path(path: &[u16]) -> String {
    String::from_utf16_lossy(&path[..nul_end(path)])
}

/// Enumerates the device paths that match `vid`/`pid`.
///
/// Interface indices are global to the driver, but `-d N` and the ambiguous
/// probe guard both count *matching* probes, so filtering happens here and the
/// result is used for opening, listing and counting alike.
pub(super) fn matching_device_paths(vid: u16, pid: u16) -> Result<Vec<Vec<u16>>> {
    let info_set = DeviceInfoSet::new()?;
    let mut paths = Vec::new();
    let mut index = 0u32;

    while let Some(interface) = info_set.enum_interface(index)? {
        index += 1;
        let path = info_set.device_path(&interface)?;
        let mut device = WCHLinkUSBDevice::open_device_path(&path)?;
        if device.read_device_ids()? == (vid, pid) {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub struct WCHLinkUSBDevice {
    handle: HANDLE,
}

impl WCHLinkUSBDevice {
    fn open_device_path(device_path: &[u16]) -> Result<Self> {
        // SAFETY: `device_path()` guarantees a NUL terminator, and the slice
        // outlives this call, so `PCWSTR::from_raw` is a valid string pointer.
        let handle = unsafe {
            CreateFileW(
                PCWSTR::from_raw(device_path.as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }?;
        Ok(Self { handle })
    }

    /// Sends a vendor IOCTL. Requests and responses share one buffer: the
    /// driver prepends an 8-byte header to its response, so a USB setup packet
    /// written at offset 8 is replaced by the descriptor on return.
    fn command(&mut self, buffer: &mut [u8], in_size: usize, out_size: usize) -> Result<usize> {
        let mut bytes_returned = 0u32;
        // SAFETY: `handle` is a live device handle from `CreateFileW`; both
        // buffer pointers and the in/out sizes describe the same `buffer`
        // (callers never pass a size beyond `buffer.len()`), and
        // `bytes_returned` is a valid exclusive out-parameter.
        unsafe {
            DeviceIoControl(
                self.handle,
                WCH_LINK_DRIVER_IOCTL,
                Some(buffer.as_ptr().cast()),
                in_size as u32,
                Some(buffer.as_mut_ptr().cast()),
                out_size as u32,
                Some(&mut bytes_returned),
                None,
            )
        }?;
        Ok(bytes_returned as usize)
    }

    /// The request and response share one buffer of the same size.
    fn command_in_place(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let len = buffer.len();
        self.command(buffer, len, len)
    }

    /// Only the header is sent; the driver fills the rest with the response.
    fn command_read_payload(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let len = buffer.len();
        self.command(buffer, RESPONSE_HEADER_LEN, len)
    }

    /// The whole buffer is sent; the driver only writes back a header.
    fn command_write_payload(&mut self, buffer: &mut [u8]) -> Result<()> {
        let len = buffer.len();
        self.command(buffer, len, RESPONSE_HEADER_LEN)?;
        Ok(())
    }

    /// GET_DESCRIPTOR(Device) request: an 8-byte header, the 8-byte USB setup
    /// packet and room for the 18-byte descriptor the driver writes back.
    fn descriptor_request() -> Vec<u8> {
        let mut buffer = vec![0u8; RESPONSE_HEADER_LEN + DEVICE_DESCRIPTOR_LEN];
        write_u32_ne(
            &mut buffer,
            HEADER_COMMAND_OFFSET,
            CMD_GET_DEVICE_DESCRIPTOR,
        );
        write_u32_ne(&mut buffer, HEADER_LENGTH_OFFSET, SETUP_PACKET_LEN as u32);
        let setup_at = RESPONSE_HEADER_LEN;
        buffer[setup_at..setup_at + GET_DEVICE_DESCRIPTOR_SETUP.len()]
            .copy_from_slice(&GET_DEVICE_DESCRIPTOR_SETUP);
        buffer
    }

    fn read_device_ids(&mut self) -> Result<(u16, u16)> {
        let mut buffer = Self::descriptor_request();
        let returned = self.command_in_place(&mut buffer)?;
        check_descriptor_response(returned)?;
        parse_device_ids(&buffer[RESPONSE_HEADER_LEN..])
    }
}

impl USBDeviceBackend for WCHLinkUSBDevice {
    fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
        let paths = matching_device_paths(vid, pid)?;
        let path = paths.get(nth).ok_or(Error::ProbeNotFound)?;
        Ok(Box::new(Self::open_device_path(path)?))
    }

    // `open_selector` is deliberately left at the trait default. `open()`
    // routes serial selectors straight to nusb, so this backend is only ever
    // reached through `open_nth`; duplicating the "cannot select by serial"
    // message here would suggest a code path that cannot happen.

    fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize> {
        let mut buffer = vec![0u8; RESPONSE_HEADER_LEN + buf.len()];
        prepare_endpoint_command(&mut buffer, ep, CMD_READ_ENDPOINT, buf.len())?;

        self.command_read_payload(&mut buffer)?;

        let length = read_u32_ne(&buffer, HEADER_LENGTH_OFFSET)? as usize;
        let payload = response_payload(&buffer, length)?;
        // The response buffer is RESPONSE_HEADER_LEN + buf.len() bytes, so any
        // payload that fits in it also fits in the caller's buffer.
        buf[..payload.len()].copy_from_slice(payload);
        Ok(payload.len())
    }

    fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()> {
        let mut buffer = vec![0u8; RESPONSE_HEADER_LEN + buf.len()];
        prepare_endpoint_command(&mut buffer, ep, CMD_WRITE_ENDPOINT, buf.len())?;
        buffer[RESPONSE_HEADER_LEN..].copy_from_slice(buf);

        self.command_write_payload(&mut buffer)
    }

    fn set_timeout(&mut self, timeout: Duration) {
        let mut buffer = timeout_command(timeout);
        // The trait cannot return an error here. A driver that rejects the
        // timeout must not take the whole process down.
        if let Err(e) = self.command_in_place(&mut buffer) {
            log::warn!("WCH native driver rejected the timeout command: {e}");
        }
    }
}

impl Drop for WCHLinkUSBDevice {
    fn drop(&mut self) {
        // SAFETY: `handle` came from `CreateFileW` in `open_device_path` and
        // the device is never cloned, so this closes it exactly once.
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl fmt::Debug for WCHLinkUSBDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A raw HANDLE is unstable and useless in a log; name the driver
        // instead, like CH375USBDevice reports its device index.
        f.debug_struct("USBDevice")
            .field("provider", &"wch_link")
            .field("driver", &"WCHLinkDrv")
            .finish()
    }
}

/// Human-readable entries for `wlink list`.
pub(super) fn list_devices(vid: u16, pid: u16) -> Result<Vec<String>> {
    Ok(matching_device_paths(vid, pid)?
        .iter()
        .enumerate()
        .map(|(index, path)| {
            format!(
                "<WCH-Link#{index} WCHLinkDriver device> ID {vid:04x}:{pid:04x} ({})",
                display_path(path)
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vendor_and_product_from_a_device_descriptor() {
        let mut descriptor = [0u8; DEVICE_DESCRIPTOR_LEN];
        descriptor[VID_OFFSET..PID_OFFSET].copy_from_slice(&0x1a86u16.to_le_bytes());
        descriptor[PID_OFFSET..DEVICE_IDS_END].copy_from_slice(&0x8010u16.to_le_bytes());
        assert_eq!(parse_device_ids(&descriptor).unwrap(), (0x1a86, 0x8010));
    }

    #[test]
    fn rejects_a_truncated_descriptor() {
        assert!(matches!(
            parse_device_ids(&[0u8; PID_OFFSET]),
            Err(Error::InvalidPayloadLength)
        ));
    }

    #[test]
    fn a_short_descriptor_response_is_an_error() {
        let full = RESPONSE_HEADER_LEN + DEVICE_DESCRIPTOR_LEN;
        assert!(check_descriptor_response(full).is_ok());
        assert!(check_descriptor_response(RESPONSE_HEADER_LEN + DEVICE_IDS_END).is_ok());
        assert!(check_descriptor_response(RESPONSE_HEADER_LEN + DEVICE_IDS_END - 1).is_err());
        assert!(check_descriptor_response(0).is_err());
    }

    #[test]
    fn response_payload_rejects_a_bogus_length() {
        let buffer = vec![0u8; RESPONSE_HEADER_LEN + 4];
        assert_eq!(response_payload(&buffer, 4).unwrap().len(), 4);
        assert_eq!(response_payload(&buffer, 0).unwrap().len(), 0);
        assert!(response_payload(&buffer, 5).is_err());
        // Would overflow usize on 32-bit Windows without the checked add.
        assert!(response_payload(&buffer, usize::MAX).is_err());
        assert!(response_payload(&[], 0).is_err());
    }

    #[test]
    fn endpoint_zero_is_not_a_bulk_endpoint() {
        assert!(driver_endpoint(0).is_err());
        assert_eq!(driver_endpoint(0x01).unwrap(), 0);
        assert_eq!(driver_endpoint(0x81).unwrap(), 0);
        assert_eq!(driver_endpoint(0x02).unwrap(), 1);
        assert_eq!(driver_endpoint(0x82).unwrap(), 1);
    }

    #[test]
    fn display_path_stops_at_the_nul_terminator() {
        let path = [
            '\\' as u16,
            '\\' as u16,
            '.' as u16,
            '\\' as u16,
            'w' as u16,
            0,
            'x' as u16,
        ];
        assert_eq!(display_path(&path), "\\\\.\\w");
    }

    // The driver protocol is undocumented, so the byte layout below is pinned
    // to what a real WCH-Link answers to. These tests keep the constant and
    // helper refactor from changing a single byte on the wire.

    #[test]
    fn header_helpers_round_trip_and_bounds_check() {
        let mut buffer = [0u8; RESPONSE_HEADER_LEN];
        write_u32_ne(&mut buffer, HEADER_COMMAND_OFFSET, CMD_SET_TIMEOUT);
        write_u32_ne(&mut buffer, HEADER_LENGTH_OFFSET, 0x10);
        assert_eq!(
            read_u32_ne(&buffer, HEADER_COMMAND_OFFSET).unwrap(),
            CMD_SET_TIMEOUT
        );
        assert_eq!(read_u32_ne(&buffer, HEADER_LENGTH_OFFSET).unwrap(), 0x10);
        assert!(read_u32_ne(&buffer, HEADER_LENGTH_OFFSET + 1).is_err());
        assert_eq!(nul_end(&['a' as u16, 0, 'b' as u16]), 1);
        assert_eq!(nul_end(&['a' as u16, 'b' as u16]), 2);
    }

    #[test]
    fn descriptor_request_keeps_the_observed_layout() {
        let request = WCHLinkUSBDevice::descriptor_request();
        let expected: [u8; RESPONSE_HEADER_LEN + SETUP_PACKET_LEN] = [
            0x04,
            0x00,
            0x00,
            0x00, // command
            0x08,
            0x00,
            0x00,
            0x00, // setup packet length
            0x80,
            0x06,
            0x00,
            0x01,
            0x00,
            0x00,
            DEVICE_DESCRIPTOR_LEN as u8,
            0x00,
        ];
        assert_eq!(request.len(), RESPONSE_HEADER_LEN + DEVICE_DESCRIPTOR_LEN);
        assert_eq!(&request[..expected.len()], &expected[..]);
        assert!(request[expected.len()..].iter().all(|b| *b == 0));
    }

    #[test]
    fn timeout_command_keeps_the_observed_layout() {
        let buffer = timeout_command(Duration::from_millis(5000));
        let expected_header: [u8; RESPONSE_HEADER_LEN] = [
            0x0f, 0x00, 0x00, 0x00, // command
            0x10, 0x00, 0x00, 0x00, // parameter length
        ];
        assert_eq!(buffer.len(), TIMEOUT_BUFFER_LEN);
        assert_eq!(&buffer[..RESPONSE_HEADER_LEN], &expected_header[..]);
        assert_eq!(
            buffer[RESPONSE_HEADER_LEN..].len(),
            TIMEOUT_PARAM_COUNT * core::mem::size_of::<u32>()
        );
        for i in 0..TIMEOUT_PARAM_COUNT {
            let start = RESPONSE_HEADER_LEN + i * core::mem::size_of::<u32>();
            let end = start + core::mem::size_of::<u32>();
            assert_eq!(&buffer[start..end], &5000u32.to_ne_bytes()[..]);
        }

        // Durations beyond u32::MAX milliseconds saturate instead of wrapping.
        let saturated = timeout_command(Duration::from_secs(u64::MAX));
        for i in 0..TIMEOUT_PARAM_COUNT {
            let start = RESPONSE_HEADER_LEN + i * core::mem::size_of::<u32>();
            let end = start + core::mem::size_of::<u32>();
            assert_eq!(&saturated[start..end], &u32::MAX.to_ne_bytes()[..]);
        }
    }

    #[test]
    fn endpoint_command_header_keeps_the_observed_layout() {
        let mut buffer = [0u8; RESPONSE_HEADER_LEN + 4];
        prepare_endpoint_command(&mut buffer, 0x81, CMD_READ_ENDPOINT, 4).unwrap();
        assert_eq!(buffer, [0u8, 0, 0x01, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
        prepare_endpoint_command(&mut buffer, 0x02, CMD_WRITE_ENDPOINT, 3).unwrap();
        assert_eq!(buffer, [1u8, 0, 0x02, 0, 3, 0, 0, 0, 0, 0, 0, 0]);
        // Endpoint 0 is the control endpoint, not a bulk endpoint.
        assert!(prepare_endpoint_command(&mut buffer, 0, CMD_READ_ENDPOINT, 4).is_err());
    }
}
