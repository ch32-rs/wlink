//! USB Device abstraction - The USB Device of WCH-Link.

use crate::Result;
use std::{
    fmt::{Debug, Display},
    str::FromStr,
    time::Duration,
};

#[cfg(target_os = "windows")]
pub mod wch_link_driver;

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

/// Number of matching probes visible through Windows backends that `open`
/// tries before nusb. Neither of them can match a USB serial number.
#[cfg(target_os = "windows")]
fn index_only_backend_count(vid: u16, pid: u16) -> Result<usize> {
    let mut count = 0;

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        // CH375's DLL is optional; when it is missing it simply sees nothing.
        count = count.max(
            ch375_driver::list_devices(vid, pid)
                .map(|devs| devs.len())
                .unwrap_or(0),
        );
    }

    // Unlike that optional DLL, an enumeration failure here means we cannot
    // tell how many probes are reachable. Propagate it so the ambiguous probe
    // guard fails closed instead of silently undercounting.
    count = count.max(wch_link_driver::matching_device_paths(vid, pid)?.len());
    Ok(count)
}

#[cfg(not(target_os = "windows"))]
fn index_only_backend_count(_vid: u16, _pid: u16) -> Result<usize> {
    Ok(0)
}

/// How [`open`] resolves a selector. Kept separate from the platform-specific
/// backend chain so the "a serial never falls back to an index" rule is
/// testable without hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenPlan {
    BySerial,
    ByIndex(usize),
}

fn open_plan(selector: &DeviceSelector) -> OpenPlan {
    match selector {
        DeviceSelector::Serial(_) => OpenPlan::BySerial,
        DeviceSelector::Index(nth) => OpenPlan::ByIndex(*nth),
    }
}

fn open_by_index(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        if let Ok(device) = ch375_driver::CH375USBDevice::open_nth(vid, pid, nth) {
            return Ok(device);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(device) = wch_link_driver::WCHLinkUSBDevice::open_nth(vid, pid, nth) {
            return Ok(device);
        }
    }

    libusb::NusbDevice::open_nth(vid, pid, nth)
}

/// Opens the Nth matching probe, in enumeration order.
pub fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
    open_by_index(vid, pid, nth)
}

/// Opens the probe `selector` names. A plain index behaves like [`open_nth`].
///
/// A serial is resolved by nusb only: no other backend can match a serial, and
/// deliberately *not* falling back to an index keeps a mistyped serial from
/// silently opening a different probe (the ambiguous probe guard, #134).
pub fn open(
    vid: u16,
    pid: u16,
    selector: impl Into<DeviceSelector>,
) -> Result<Box<dyn USBDeviceBackend>> {
    let selector = &selector.into();
    match open_plan(selector) {
        OpenPlan::BySerial => libusb::NusbDevice::open_selector(vid, pid, selector)
            .map_err(|err| serial_open_error(selector, err)),
        OpenPlan::ByIndex(nth) => open_by_index(vid, pid, nth),
    }
}

/// On Windows in particular, nusb can enumerate a probe bound to WCH's native
/// driver but cannot open it, and the raw `incompatible driver` error gives no
/// hint about what to change. Keep the original cause and name the options.
fn serial_open_error(selector: &DeviceSelector, err: crate::Error) -> crate::Error {
    crate::Error::Custom(format!(
        "cannot open WCH-Link by USB {selector}: {err}. Check `wlink list` for \
         the available probes. A serial selector requires a WinUSB/libusb driver \
         (see https://zadig.akeo.ie); with WCH's native driver or the 32-bit \
         CH375 driver, use `-d <INDEX>` instead."
    ))
}

pub fn list_devices(vid: u16, pid: u16) -> Result<Vec<String>> {
    // Report the same probe set `open` would pick from. On Windows more than
    // one backend can enumerate the same physical probe (CH375/native/nusb),
    // so concatenating every list would print it twice and make `wlink list`
    // disagree with the ambiguous probe guard.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        // The CH375 DLL is optional. `open_by_index` simply skips that backend
        // when it cannot be loaded, so listing must not fail either.
        let ch375 = match ch375_driver::list_devices(vid, pid) {
            Ok(devs) => devs.into_iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            Err(err) => {
                log::debug!("CH375 driver unavailable, skipping: {err}");
                Vec::new()
            }
        };
        if !ch375.is_empty() {
            return Ok(ch375);
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Listing is informational: unlike the ambiguous probe guard (which
        // must fail closed), it can fall through to nusb when the native
        // driver cannot be queried, e.g. a probe is busy in WCH's own tooling.
        match wch_link_driver::list_devices(vid, pid) {
            Ok(native) if !native.is_empty() => return Ok(native),
            Ok(_) => {}
            Err(err) => log::warn!("could not enumerate the WCH native driver: {err}"),
        }
    }

    Ok(libusb::list_libusb_devices(vid, pid)?
        .into_iter()
        .map(|s| s.to_string())
        .collect())
}

/// Minimal description of a matching USB device, used to offer the user an
/// explicit `-d` selector when several probes are attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedDevice {
    /// Position among devices with the same VID/PID, in enumeration order.
    pub index: usize,
    /// USB serial number, when the backend can read it.
    pub serial: Option<String>,
    /// Whether `DeviceSelector::Serial` can open this device.
    ///
    /// False when an index-only Windows backend (CH375 on 32-bit, or WCH's
    /// native driver) can see matching probes: `open` tries those first for
    /// index selectors and their enumeration order need not match nusb's, so
    /// an index may not name the probe carrying this serial.
    pub serial_selectable: bool,
}

/// Lists matching probes using the same enumeration that [`open`] selects
/// from, so the offered selectors agree with what a selector-less open would
/// actually pick.
///
/// This is separate from [`list_devices`]: on Windows the index-only backends
/// and nusb can enumerate the same physical probe (nusb just cannot open one
/// bound to WCH's driver), so merging their printed lists would double count
/// it. nusb is preferred because it exposes serials; taking the larger count
/// also keeps a probe that only a native driver reports in the guard.
pub fn list_attached_devices(vid: u16, pid: u16) -> Result<Vec<AttachedDevice>> {
    let devices = libusb::list_attached_devices(vid, pid)?;
    let index_only_count = index_only_backend_count(vid, pid)?;
    Ok(merge_attached_views(devices, index_only_count))
}

fn merge_attached_views(
    mut devices: Vec<AttachedDevice>,
    index_only_count: usize,
) -> Vec<AttachedDevice> {
    if index_only_count > 0 {
        // `open` tries those backends first for index selectors, and their
        // enumeration order need not match nusb's. Offering a serial next to
        // an index that may resolve through a different view would name the
        // wrong probe, so offer indices for every entry.
        for device in &mut devices {
            device.serial_selectable = false;
        }
    }
    pad_index_only_devices(devices, index_only_count)
}

/// Pads a device list with index-only placeholders when another backend sees
/// more probes than nusb. Without this, an ambiguous setup would look like a
/// single probe and the flash would go ahead. Index selectors are opened by
/// the native drivers first on Windows, so these remain selectable.
fn pad_index_only_devices(
    mut devices: Vec<AttachedDevice>,
    min_count: usize,
) -> Vec<AttachedDevice> {
    while devices.len() < min_count {
        let index = devices.len();
        devices.push(AttachedDevice {
            index,
            serial: None,
            serial_selectable: false,
        });
    }
    devices
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
                .field("device", &"WinUSB interface 0")
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
                serial_selectable: true,
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

    #[test]
    fn a_backend_that_sees_more_probes_does_not_undercount() {
        // Regression (32-bit Windows): CH375 can see probes nusb cannot, and
        // `open` prefers CH375, so returning only the nusb list made two
        // probes look like one and let an ambiguous flash through.
        let devices = vec![AttachedDevice {
            index: 0,
            serial: Some("AAA".into()),
            serial_selectable: true,
        }];

        let padded = pad_index_only_devices(devices.clone(), 3);
        assert_eq!(padded.len(), 3);
        assert_eq!(padded[0].serial.as_deref(), Some("AAA"));
        assert_eq!(
            padded[1],
            AttachedDevice {
                index: 1,
                serial: None,
                serial_selectable: false,
            }
        );
        assert_eq!(
            padded[2],
            AttachedDevice {
                index: 2,
                serial: None,
                serial_selectable: false,
            }
        );

        // nusb seeing more (or the same) keeps its richer, serial-bearing list.
        assert_eq!(pad_index_only_devices(devices.clone(), 1), devices);
        assert!(pad_index_only_devices(vec![], 0).is_empty());
    }

    #[test]
    fn a_serial_never_falls_back_to_an_index() {
        // Regression (Windows native driver): a backend that cannot match a
        // serial must not be allowed to answer with "the Nth probe instead".
        // That would silently open the wrong board when a serial is mistyped.
        assert_eq!(
            open_plan(&DeviceSelector::Serial("D7708F065C7F".into())),
            OpenPlan::BySerial
        );
        assert_eq!(open_plan(&DeviceSelector::Index(3)), OpenPlan::ByIndex(3));
    }

    #[test]
    fn a_failed_serial_names_the_winusb_requirement() {
        let selector = DeviceSelector::Serial("2FC48F06B4A8".into());
        let err = serial_open_error(
            &selector,
            crate::Error::Custom("incompatible driver is installed".into()),
        )
        .to_string();
        assert!(err.contains("2FC48F06B4A8"), "{err}");
        assert!(err.contains("incompatible driver"), "{err}");
        assert!(err.contains("`wlink list`"), "{err}");
        assert!(err.contains("WinUSB"), "{err}");
        assert!(err.contains("-d <INDEX>"), "{err}");
    }

    #[test]
    fn probes_only_visible_to_a_native_driver_are_not_undercounted() {
        // If the only available view comes from an index-only backend, its
        // count must still make the ambiguous probe guard refuse a flash.
        let views = merge_attached_views(vec![], 2);
        assert_eq!(views.len(), 2);
        assert!(
            views
                .iter()
                .all(|d| d.serial.is_none() && !d.serial_selectable)
        );
        assert_eq!(views[0].index, 0);
        assert_eq!(views[1].index, 1);
    }

    #[test]
    fn an_index_only_backend_disables_serial_selectors() {
        let devices = vec![
            AttachedDevice {
                index: 0,
                serial: Some("AAA".into()),
                serial_selectable: true,
            },
            AttachedDevice {
                index: 1,
                serial: Some("BBB".into()),
                serial_selectable: true,
            },
        ];

        let merged = merge_attached_views(devices, 1);
        assert_eq!(
            merged,
            vec![
                AttachedDevice {
                    index: 0,
                    serial: Some("AAA".into()),
                    serial_selectable: false,
                },
                AttachedDevice {
                    index: 1,
                    serial: Some("BBB".into()),
                    serial_selectable: false,
                },
            ]
        );

        // With no native backend in play, serials stay usable.
        let unchanged = merge_attached_views(
            vec![AttachedDevice {
                index: 0,
                serial: Some("AAA".into()),
                serial_selectable: true,
            }],
            0,
        );
        assert!(unchanged[0].serial_selectable);
    }
}
