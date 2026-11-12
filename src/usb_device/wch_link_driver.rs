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
/// Bytes the driver puts in front of an IOCTL response.
const RESPONSE_HEADER_LEN: usize = 8;

struct DeviceInfoSet {
    hdevinfo: HDEVINFO,
}

impl DeviceInfoSet {
    fn new() -> Result<Self> {
        Ok(Self {
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
        unsafe {
            // cbSize is architecture-dependent (6 on x86, 8 on x86_64), which is
            // exactly what size_of gives us for this struct.
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
        unsafe {
            core::ptr::copy_nonoverlapping(
                storage.as_ptr().cast::<u8>().add(path_offset),
                path.as_mut_ptr().cast::<u8>(),
                path.len() * core::mem::size_of::<u16>(),
            );
        }
        // CreateFileW needs a NUL terminator; keep one even if the reported
        // size did not include it.
        match path.iter().position(|c| *c == 0) {
            Some(end) => path.truncate(end + 1),
            None => path.push(0),
        }
        Ok(path)
    }
}

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
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
    let bytes = descriptor.get(..12).ok_or(Error::InvalidPayloadLength)?;
    Ok((
        u16::from_le_bytes([bytes[8], bytes[9]]),
        u16::from_le_bytes([bytes[10], bytes[11]]),
    ))
}

/// The driver normally returns the 8-byte header plus the whole 18-byte
/// descriptor. A short response leaves zeros in the buffer, which would parse
/// as VID/PID 0000:0000 and silently hide the probe instead of reporting a
/// driver problem.
fn check_descriptor_response(bytes_returned: usize) -> Result<()> {
    const MIN_RESPONSE: usize = RESPONSE_HEADER_LEN + 12; // through idProduct
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

fn display_path(path: &[u16]) -> String {
    let end = path.iter().position(|c| *c == 0).unwrap_or(path.len());
    String::from_utf16_lossy(&path[..end])
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

    fn read_device_ids(&mut self) -> Result<(u16, u16)> {
        let mut buffer = vec![0u8; RESPONSE_HEADER_LEN + DEVICE_DESCRIPTOR_LEN];
        // GET_DESCRIPTOR(Device), wLength = 18.
        let setup: [u8; 8] = [
            0x80,
            0x06,
            0x00,
            0x01,
            0x00,
            0x00,
            DEVICE_DESCRIPTOR_LEN as u8,
            0x00,
        ];
        buffer[..4].copy_from_slice(&4u32.to_ne_bytes());
        buffer[4..8].copy_from_slice(&8u32.to_ne_bytes());
        buffer[8..16].copy_from_slice(&setup);

        let len = buffer.len();
        let returned = self.command(&mut buffer, len, len)?;
        check_descriptor_response(returned)?;
        parse_device_ids(&buffer[RESPONSE_HEADER_LEN..])
    }
}

impl USBDeviceBackend for WCHLinkUSBDevice {
    fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
        let path = matching_device_paths(vid, pid)?
            .get(nth)
            .ok_or(Error::ProbeNotFound)?
            .clone();
        Ok(Box::new(Self::open_device_path(&path)?))
    }

    // `open_selector` is deliberately left at the trait default. `open()`
    // routes serial selectors straight to nusb, so this backend is only ever
    // reached through `open_nth`; duplicating the "cannot select by serial"
    // message here would suggest a code path that cannot happen.

    fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize> {
        let mut buffer = vec![0u8; RESPONSE_HEADER_LEN + buf.len()];
        buffer[0] = driver_endpoint(ep)?;
        buffer[2] = 0x01;
        buffer[4..8].copy_from_slice(&(buf.len() as u32).to_ne_bytes());

        let len = buffer.len();
        self.command(&mut buffer, RESPONSE_HEADER_LEN, len)?;

        let length = u32::from_ne_bytes(buffer[4..8].try_into().map_err(|_| Error::InvalidPayload)?)
            as usize;
        let payload = response_payload(&buffer, length)?;
        // The response buffer is RESPONSE_HEADER_LEN + buf.len() bytes, so any
        // payload that fits in it also fits in the caller's buffer.
        buf[..payload.len()].copy_from_slice(payload);
        Ok(payload.len())
    }

    fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()> {
        let mut buffer = vec![0u8; RESPONSE_HEADER_LEN + buf.len()];
        buffer[0] = driver_endpoint(ep)?;
        buffer[2] = 0x02;
        buffer[4..8].copy_from_slice(&(buf.len() as u32).to_ne_bytes());
        buffer[RESPONSE_HEADER_LEN..].copy_from_slice(buf);

        let len = buffer.len();
        self.command(&mut buffer, len, RESPONSE_HEADER_LEN)?;
        Ok(())
    }

    fn set_timeout(&mut self, timeout: Duration) {
        let ds = timeout.as_millis().min(u32::MAX as u128) as u32;
        let mut buffer = [0u8; 24];
        buffer[..4].copy_from_slice(&0x0Fu32.to_ne_bytes());
        buffer[4..8].copy_from_slice(&0x10u32.to_ne_bytes());
        for offset in (8..buffer.len()).step_by(4) {
            buffer[offset..offset + 4].copy_from_slice(&ds.to_ne_bytes());
        }

        // The trait cannot return an error here. A driver that rejects the
        // timeout must not take the whole process down.
        let len = buffer.len();
        if let Err(e) = self.command(&mut buffer, len, len) {
            log::warn!("WCH native driver rejected the timeout command: {e}");
        }
    }
}

impl Drop for WCHLinkUSBDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl fmt::Debug for WCHLinkUSBDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("USBDevice")
            .field("provider", &"wch_link")
            .field("device", &self.handle)
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
        descriptor[8..10].copy_from_slice(&0x1a86u16.to_le_bytes());
        descriptor[10..12].copy_from_slice(&0x8010u16.to_le_bytes());
        assert_eq!(parse_device_ids(&descriptor).unwrap(), (0x1a86, 0x8010));
    }

    #[test]
    fn rejects_a_truncated_descriptor() {
        assert!(matches!(
            parse_device_ids(&[0u8; 10]),
            Err(Error::InvalidPayloadLength)
        ));
    }

    #[test]
    fn a_short_descriptor_response_is_an_error() {
        let full = RESPONSE_HEADER_LEN + DEVICE_DESCRIPTOR_LEN;
        assert!(check_descriptor_response(full).is_ok());
        assert!(check_descriptor_response(RESPONSE_HEADER_LEN + 12).is_ok());
        assert!(check_descriptor_response(RESPONSE_HEADER_LEN + 11).is_err());
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
}
