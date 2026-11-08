//! USB Device abstraction - The USB Device of WCH-Link.

use crate::Result;
use std::{
    fmt::{Debug, Display},
    str::FromStr,
    time::Duration,
};

/// Which WCH-Link to operate on when more than one is attached.
///
/// The index form (`-d 1`) is the original one and follows enumeration order,
/// which changes when probes are replugged or when one switches between RV and
/// DAP mode. The serial form (`-d 1a86:8010:D7708F065C7F`) names one probe
/// regardless of enumeration order; it accepts the same `VID:PID:SERIAL`
/// string that `probe-rs` takes in `PROBE_RS_PROBE`, so a serial copied from
/// there works here unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceSelector {
    /// The Nth matching probe, in enumeration order.
    Index(usize),
    /// The probe whose USB serial number is this, in RV or DAP mode.
    Serial(String),
}

impl From<usize> for DeviceSelector {
    fn from(index: usize) -> Self {
        Self::Index(index)
    }
}

impl From<&DeviceSelector> for DeviceSelector {
    fn from(selector: &DeviceSelector) -> Self {
        selector.clone()
    }
}

impl Display for DeviceSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Index(index) => write!(f, "#{index}"),
            Self::Serial(serial) => write!(f, "serial {serial}"),
        }
    }
}

/// USB serials are hexadecimal, so their case is not significant. Comparing
/// case-insensitively lets a serial copied from `probe-rs list` (upper case)
/// work even when typed in lower case.
fn serial_matches(actual: Option<&str>, wanted: &str) -> bool {
    actual.is_some_and(|serial| serial.eq_ignore_ascii_case(wanted))
}

impl FromStr for DeviceSelector {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty probe selector".into());
        }
        if let Ok(index) = s.parse::<usize>() {
            return Ok(Self::Index(index));
        }
        // `VID:PID:SERIAL`, as `probe-rs` prints and accepts it. The IDs are
        // not used for matching: wlink knows the two WCH-Link VID/PID pairs
        // (RV 8010, DAP 8012) and searches both, so one serial selects the same
        // physical probe in either mode. Accepting the whole string keeps it
        // copy-pasteable from `probe-rs list`.
        let serial = match s.split(':').collect::<Vec<_>>().as_slice() {
            [vid, pid, serial] => {
                u16::from_str_radix(vid, 16)
                    .map_err(|_| format!("invalid USB vendor id {vid:?}"))?;
                u16::from_str_radix(pid, 16)
                    .map_err(|_| format!("invalid USB product id {pid:?}"))?;
                *serial
            }
            [serial] => serial,
            _ => return Err(format!("expected INDEX or VID:PID:SERIAL, got {s:?}")),
        };
        if serial.is_empty() {
            return Err("empty serial number".into());
        }
        Ok(Self::Serial(serial.to_string()))
    }
}

pub trait USBDeviceBackend: Debug {
    fn set_timeout(&mut self, _timeout: Duration) {}

    fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize>;

    /// Opens the Nth matching probe. Backends that can read USB serial numbers
    /// should also override [`Self::open_selector`] so a probe can be named
    /// independently of enumeration order.
    fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>>
    where
        Self: Sized;

    /// Opens the probe `selector` names. The default understands indices only.
    fn open_selector(
        vid: u16,
        pid: u16,
        selector: &DeviceSelector,
    ) -> Result<Box<dyn USBDeviceBackend>>
    where
        Self: Sized,
    {
        match selector {
            DeviceSelector::Index(nth) => Self::open_nth(vid, pid, *nth),
            DeviceSelector::Serial(_) => Err(crate::Error::Custom(
                "this USB backend cannot select a probe by serial number".into(),
            )),
        }
    }

    fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()>;
}

/// Opens the Nth matching probe, in enumeration order.
pub fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        ch375_driver::CH375USBDevice::open_nth(vid, pid, nth)
            .or_else(|_| libusb::NusbDevice::open_nth(vid, pid, nth))
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    {
        libusb::NusbDevice::open_nth(vid, pid, nth)
    }
}

/// Opens the probe `selector` names. A plain index behaves like [`open_nth`].
pub fn open(
    vid: u16,
    pid: u16,
    selector: impl Into<DeviceSelector>,
) -> Result<Box<dyn USBDeviceBackend>> {
    let selector = &selector.into();
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        ch375_driver::CH375USBDevice::open_selector(vid, pid, selector)
            .or_else(|_| libusb::NusbDevice::open_selector(vid, pid, selector))
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    {
        libusb::NusbDevice::open_selector(vid, pid, selector)
    }
}

pub fn list_devices(vid: u16, pid: u16) -> Result<Vec<String>> {
    let mut ret = vec![];
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        ret.extend(
            ch375_driver::list_devices(vid, pid)?
                .into_iter()
                .map(|s| s.to_string()),
        );
    }

    ret.extend(
        libusb::list_libusb_devices(vid, pid)?
            .into_iter()
            .map(|s| s.to_string()),
    );

    Ok(ret)
}

/// Minimal description of a matching USB device, used to offer the user an
/// explicit `-d` selector when several probes are attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedDevice {
    /// Position among devices with the same VID/PID, in enumeration order.
    pub index: usize,
    /// USB serial number, when the backend can read it.
    pub serial: Option<String>,
}

/// Lists matching probes using the same enumeration that [`open`] selects
/// from, so the offered selectors agree with what a selector-less open would
/// actually pick.
///
/// This is separate from [`list_devices`]: on 32-bit Windows the CH375 driver
/// and nusb both see the same physical probes, so merging their printed lists
/// would double count them. nusb is preferred because it exposes serials.
pub fn list_attached_devices(vid: u16, pid: u16) -> Result<Vec<AttachedDevice>> {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        let nusb_devices = libusb::list_attached_devices(vid, pid)?;
        if !nusb_devices.is_empty() {
            return Ok(nusb_devices);
        }
        // No WinUSB-compatible devices: fall back to the CH375 driver, which
        // cannot report serial numbers. The DLL is optional.
        let count = ch375_driver::list_devices(vid, pid)
            .map(|devs| devs.len())
            .unwrap_or(0);
        Ok((0..count)
            .map(|index| AttachedDevice {
                index,
                serial: None,
            })
            .collect())
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    {
        libusb::list_attached_devices(vid, pid)
    }
}

pub mod libusb {
    use std::fmt;
    use std::io::{Read, Write};

    use super::*;
    use nusb::MaybeFuture;
    use nusb::transfer::{Bulk, In, Out};

    pub fn list_libusb_devices(vid: u16, pid: u16) -> Result<Vec<impl Display>> {
        let devices = nusb::list_devices().wait().map_err(crate::Error::Usb)?;
        let mut result = vec![];
        let mut idx = 0;

        for device in devices {
            if device.vendor_id() == vid && device.product_id() == pid {
                let serial = device
                    .serial_number()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "N/A".to_string());

                result.push(format!(
                    "<WCH-Link#{} nusb device> ID {:04x}:{:04x} Serial {} ({})",
                    idx,
                    device.vendor_id(),
                    device.product_id(),
                    serial,
                    get_speed(device.speed())
                ));
                idx += 1;
            }
        }
        Ok(result)
    }

    pub struct NusbDevice {
        interface: nusb::Interface,
        #[allow(dead_code)]
        timeout: Duration,
    }

    impl fmt::Debug for NusbDevice {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("USBDevice")
                .field("provider", &"nusb")
                .finish()
        }
    }

    impl USBDeviceBackend for NusbDevice {
        fn set_timeout(&mut self, timeout: Duration) {
            self.timeout = timeout;
        }

        fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
            let devices = matching_devices(vid, pid)?;
            let device_info = devices.get(nth).ok_or(crate::Error::ProbeNotFound)?;
            open_info(device_info)
        }

        fn open_selector(
            vid: u16,
            pid: u16,
            selector: &DeviceSelector,
        ) -> Result<Box<dyn USBDeviceBackend>> {
            match selector {
                DeviceSelector::Index(nth) => Self::open_nth(vid, pid, *nth),
                DeviceSelector::Serial(serial) => {
                    let devices = matching_devices(vid, pid)?;
                    let device_info = devices
                        .iter()
                        .find(|d| serial_matches(d.serial_number(), serial))
                        .ok_or(crate::Error::ProbeNotFound)?;
                    open_info(device_info)
                }
            }
        }

        fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize> {
            let endpoint = self
                .interface
                .endpoint::<Bulk, In>(ep)
                .map_err(|e| crate::Error::Custom(format!("Failed to get endpoint: {}", e)))?;
            let mut reader = endpoint.reader(64);
            let n = reader.read(buf)?;
            Ok(n)
        }

        fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()> {
            let endpoint = self
                .interface
                .endpoint::<Bulk, Out>(ep)
                .map_err(|e| crate::Error::Custom(format!("Failed to get endpoint: {}", e)))?;
            let mut writer = endpoint.writer(64);
            writer.write_all(buf)?;
            writer.flush()?;
            Ok(())
        }
    }

    pub(super) fn list_attached_devices(vid: u16, pid: u16) -> Result<Vec<AttachedDevice>> {
        Ok(matching_devices(vid, pid)?
            .iter()
            .enumerate()
            .map(|(index, device)| AttachedDevice {
                index,
                serial: device.serial_number().map(|s| s.to_string()),
            })
            .collect())
    }

    pub(super) fn matching_devices(vid: u16, pid: u16) -> Result<Vec<nusb::DeviceInfo>> {
        let devices = nusb::list_devices()
            .wait()
            .map_err(crate::Error::Usb)?
            .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
            .collect();
        Ok(devices)
    }

    fn open_info(device_info: &nusb::DeviceInfo) -> Result<Box<dyn USBDeviceBackend>> {
        log::trace!(
            "Device: {:04x}:{:04x}",
            device_info.vendor_id(),
            device_info.product_id()
        );

        if let Some(serial) = device_info.serial_number() {
            log::debug!("Serial number: {:?}", serial);
        }

        let device = device_info.open().wait().map_err(|e| {
            log::error!("Failed to open USB device: {}", e);
            #[cfg(target_os = "windows")]
            log::warn!("It's likely no WinUSB driver installed. Please install it from Zadig. See also: https://zadig.akeo.ie");
            #[cfg(target_os = "linux")]
            log::warn!("It's likely the udev rules are not installed properly. Please refer to README.md for more details.");
            crate::Error::Usb(e)
        })?;

        let interface = device
            .claim_interface(0)
            .wait()
            .map_err(crate::Error::Usb)?;

        Ok(Box::new(NusbDevice {
            interface,
            timeout: Duration::from_millis(5000),
        }))
    }

    fn get_speed(speed: Option<nusb::Speed>) -> &'static str {
        match speed {
            Some(nusb::Speed::SuperPlus) => "USB-SS+ 10000 Mbps",
            Some(nusb::Speed::Super) => "USB-SS 5000 Mbps",
            Some(nusb::Speed::High) => "USB-HS 480 Mbps",
            Some(nusb::Speed::Full) => "USB-FS 12 Mbps",
            Some(nusb::Speed::Low) => "USB-LS 1.5 Mbps",
            _ => "(unknown)",
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub mod ch375_driver {
    use libloading::os::windows::*;
    use std::fmt;
    use std::sync::OnceLock;

    use super::*;
    use crate::Error;

    static CH375_DRIVER: OnceLock<std::result::Result<Library, String>> = OnceLock::new();

    fn ensure_library_load() -> Result<&'static Library> {
        let result = CH375_DRIVER.get_or_init(|| {
            let lib = match unsafe { Library::new("WCHLinkDLL.dll") } {
                Ok(lib) => lib,
                Err(_) => return Err("WCHLinkDLL.dll not found".to_string()),
            };

            let get_version: Symbol<unsafe extern "stdcall" fn() -> u32> =
                unsafe { lib.get(b"CH375GetVersion").unwrap() };
            let get_driver_version: Symbol<unsafe extern "stdcall" fn() -> u32> =
                unsafe { lib.get(b"CH375GetDrvVersion").unwrap() };

            log::debug!(
                "DLL version {}, driver version {}",
                unsafe { get_version() },
                unsafe { get_driver_version() }
            );
            Ok(lib)
        });

        match result {
            Ok(lib) => Ok(lib),
            Err(e) => Err(Error::Custom(e.clone())),
        }
    }

    #[allow(non_snake_case, unused)]
    #[derive(Debug)]
    #[repr(packed)]
    pub struct UsbDeviceDescriptor {
        bLength: u8,
        bDescriptorType: u8,
        bcdUSB: u16,
        bDeviceClass: u8,
        bDeviceSubClass: u8,
        bDeviceProtocol: u8,
        bMaxPacketSize0: u8,
        idVendor: u16,
        idProduct: u16,
        bcdDevice: u16,
        iManufacturer: u8,
        iProduct: u8,
        iSerialNumber: u8,
        bNumConfigurations: u8,
    }

    pub fn list_devices(vid: u16, pid: u16) -> Result<Vec<impl Display>> {
        let lib = ensure_library_load()?;
        let mut ret: Vec<String> = vec![];

        let open_device: Symbol<unsafe extern "stdcall" fn(u32) -> u32> =
            unsafe { lib.get(b"CH375OpenDevice").unwrap() };
        let close_device: Symbol<unsafe extern "stdcall" fn(u32)> =
            unsafe { lib.get(b"CH375CloseDevice").unwrap() };
        let get_device_descriptor: Symbol<
            unsafe extern "stdcall" fn(u32, *mut UsbDeviceDescriptor, *mut u32) -> bool,
        > = unsafe { lib.get(b"CH375GetDeviceDescr").unwrap() };

        const INVALID_HANDLE: u32 = 0xffffffff;

        for i in 0..8 {
            let h = unsafe { open_device(i) };
            if h != INVALID_HANDLE {
                let mut descr = unsafe { core::mem::zeroed() };
                let mut len = core::mem::size_of::<UsbDeviceDescriptor>() as u32;
                let _ = unsafe { get_device_descriptor(i, &mut descr, &mut len) };

                if descr.idVendor == vid && descr.idProduct == pid {
                    ret.push(format!(
                        "<WCH-Link#{} WCHLinkDLL device> CH375Driver Device {:04x}:{:04x}",
                        i, vid, pid
                    ));

                    log::debug!("Device #{}: {:04x}:{:04x}", i, vid, pid);
                }
                unsafe { close_device(i) };
            }
        }

        Ok(ret)
    }

    /// USB Device implementation provided by CH375 Windows driver
    pub struct CH375USBDevice {
        index: u32,
    }

    impl fmt::Debug for CH375USBDevice {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("USBDevice")
                .field("provider", &"ch375")
                .field("device", &self.index)
                .finish()
        }
    }

    impl USBDeviceBackend for CH375USBDevice {
        // The CH375 driver opens devices by handle index and never exposes
        // their serial numbers. `open_selector` is left at its default, which
        // rejects serial selectors so `usb_device::open` falls through to the
        // nusb backend.
        fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
            let lib = ensure_library_load()?;
            /*HANDLE WINAPI CH375OpenDevice( // Open CH375 device, return the handle, invalid if error
            ULONG	iIndex );  */
            let open_device: Symbol<unsafe extern "stdcall" fn(u32) -> u32> =
                unsafe { lib.get(b"CH375OpenDevice").unwrap() };
            /*VOID WINAPI CH375CloseDevice( // Close the CH375 device
            ULONG	iIndex );         // Specify the serial number of the CH375 device */
            let close_device: Symbol<unsafe extern "stdcall" fn(u32)> =
                unsafe { lib.get(b"CH375CloseDevice").unwrap() };
            let get_device_descriptor: Symbol<
                unsafe extern "stdcall" fn(u32, *mut UsbDeviceDescriptor, *mut u32) -> bool,
            > = unsafe { lib.get(b"CH375GetDeviceDescr").unwrap() };

            const INVALID_HANDLE: u32 = 0xffffffff;

            let mut idx = 0;
            for i in 0..8 {
                let h = unsafe { open_device(i) };
                if h != INVALID_HANDLE {
                    let mut descr = unsafe { core::mem::zeroed() };
                    let mut len = core::mem::size_of::<UsbDeviceDescriptor>() as u32;
                    let _ = unsafe { get_device_descriptor(i, &mut descr, &mut len) };

                    if descr.idVendor == vid && descr.idProduct == pid {
                        if idx == nth {
                            log::debug!("Device #{}: {:04x}:{:04x}", i, vid, pid);
                            return Ok(Box::new(CH375USBDevice { index: i }));
                        } else {
                            idx += 1;
                        }
                    }
                    unsafe { close_device(i) };
                }
            }

            return Err(crate::Error::ProbeNotFound);
        }

        fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize> {
            let lib = ensure_library_load()?;
            /*
            BOOL WINAPI CH375ReadEndP( // read data block
            ULONG	iIndex,        // Specify the serial number of the CH375 device
            ULONG	iPipeNum,      // Endpoint number, valid values are 1 to 8.
            PVOID	oBuffer,       // Point to a buffer large enough to hold the read data
            PULONG	ioLength);     // Point to the length unit, the length to be read when input, and the actual read length after return
             */
            let read_end_point: Symbol<
                unsafe extern "stdcall" fn(u32, u32, *mut u8, *mut u32) -> bool,
            > = unsafe { lib.get(b"CH375ReadEndP").unwrap() };

            let mut len = buf.len() as u32;
            let ep = (ep & 0x7f) as u32;

            let ret = unsafe { read_end_point(self.index, ep, buf.as_mut_ptr(), &mut len) };

            if ret {
                Ok(len as usize)
            } else {
                Err(Error::Driver)
            }
        }

        fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()> {
            let lib = ensure_library_load()?;
            /*
                BOOL WINAPI CH375WriteEndP( // write out data block
            ULONG	iIndex,         // Specify the serial number of the CH375 device
            ULONG	iPipeNum,       // Endpoint number, valid values are 1 to 8.
            PVOID	iBuffer,        // Point to a buffer where the data to be written is placed
            PULONG	ioLength);      // Point to the length unit, the length to be written out when input, and the length actually written out after returnF */
            let write_end_point: Symbol<
                unsafe extern "stdcall" fn(u32, u32, *mut u8, *mut u32) -> bool,
            > = unsafe { lib.get(b"CH375WriteEndP").unwrap() };

            let mut len = buf.len() as u32;
            let ret = unsafe {
                write_end_point(self.index, ep as u32, buf.as_ptr() as *mut u8, &mut len)
            };
            if ret { Ok(()) } else { Err(Error::Driver) }
        }

        fn set_timeout(&mut self, timeout: Duration) {
            let lib = ensure_library_load().unwrap();

            let set_timeout_ex: Symbol<
                unsafe extern "stdcall" fn(u32, u32, u32, u32, u32) -> bool,
            > = unsafe { lib.get(b"CH375SetTimeoutEx").unwrap() };

            let ds = timeout.as_millis() as u32;

            unsafe {
                set_timeout_ex(self.index, ds, ds, ds, ds);
            }
        }
    }

    impl Drop for CH375USBDevice {
        fn drop(&mut self) {
            if let Ok(lib) = ensure_library_load() {
                let close_device: Symbol<unsafe extern "stdcall" fn(u32)> =
                    unsafe { lib.get(b"CH375CloseDevice").unwrap() };
                unsafe {
                    close_device(self.index);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_selector_keeps_the_original_behaviour() {
        assert_eq!("0".parse(), Ok(DeviceSelector::Index(0)));
        assert_eq!("2".parse(), Ok(DeviceSelector::Index(2)));
        assert_eq!(" 1 ".parse(), Ok(DeviceSelector::Index(1)));
    }

    #[test]
    fn probe_rs_style_selector_uses_the_serial() {
        // Regression: this used to be parsed as a usize and failed with
        // "invalid digit found in string".
        assert_eq!(
            "1a86:8010:D7708F065C7F".parse(),
            Ok(DeviceSelector::Serial("D7708F065C7F".into()))
        );
        // Upper case IDs and the DAP-mode PID are accepted as well; the IDs
        // are validated but not used for matching.
        assert_eq!(
            "1A86:8012:d7708f065c7f".parse(),
            Ok(DeviceSelector::Serial("d7708f065c7f".into()))
        );
    }

    #[test]
    fn bare_serial_is_a_selector() {
        assert_eq!(
            "D7708F065C7F".parse(),
            Ok(DeviceSelector::Serial("D7708F065C7F".into()))
        );
    }

    #[test]
    fn malformed_selectors_are_rejected() {
        for s in [
            "",
            "  ",
            "zzzz:8010:SERIAL",
            "1a86:8010",
            "1a86:8010:",
            "1:2:3:4",
        ] {
            assert!(
                s.parse::<DeviceSelector>().is_err(),
                "{s:?} should be rejected"
            );
        }
    }

    #[test]
    fn serial_matching_ignores_case_but_not_content() {
        assert!(serial_matches(Some("D7708F065C7F"), "d7708f065c7f"));
        assert!(!serial_matches(Some("D7708F065C7F"), "D7708F065C7E"));
        assert!(!serial_matches(None, "D7708F065C7F"));
    }

    #[test]
    fn display_describes_the_selector() {
        assert_eq!(DeviceSelector::Index(1).to_string(), "#1");
        assert_eq!(
            DeviceSelector::Serial("ABC".into()).to_string(),
            "serial ABC"
        );
    }
}
