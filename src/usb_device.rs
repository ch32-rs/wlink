//! USB Device abstraction - The USB Device of WCH-Link.

use crate::Result;
use std::{
    fmt::{Debug, Display},
    time::Duration,
};

pub trait USBDeviceBackend: Debug {
    fn set_timeout(&mut self, _timeout: Duration) {}

    fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize>;

    fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>>
    where
        Self: Sized;

    fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()>;
}

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

pub mod libusb {
    use std::fmt;
    use std::io::{Read, Write};

    use super::*;
    use nusb::transfer::{Bulk, In, Out};
    use nusb::MaybeFuture;

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
            let devices: Vec<_> = nusb::list_devices()
                .wait()
                .map_err(crate::Error::Usb)?
                .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
                .collect();

            if nth >= devices.len() {
                return Err(crate::Error::ProbeNotFound);
            }

            let device_info = &devices[nth];
            log::trace!("Device: {:04x}:{:04x}", device_info.vendor_id(), device_info.product_id());

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

            let interface = device.claim_interface(0).wait().map_err(crate::Error::Usb)?;

            Ok(Box::new(NusbDevice {
                interface,
                timeout: Duration::from_millis(5000),
            }))
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

    struct CH375Libraries {
        wchlink_dll: Option<Library>,
        ch375_dll: Option<Library>,
    }

    static CH375_DRIVERS: OnceLock<std::result::Result<CH375Libraries, String>> = OnceLock::new();

    fn ensure_libraries_load() -> Result<&'static CH375Libraries> {
        let result = CH375_DRIVERS.get_or_init(|| {
            let wchlink_dll = unsafe { Library::new("WCHLinkDLL.dll") }.ok();
            let ch375_dll = unsafe { Library::new("CH375DLL.dll") }.ok();

            if wchlink_dll.is_none() && ch375_dll.is_none() {
                return Err("Neither WCHLinkDLL.dll nor CH375DLL.dll found".to_string());
            }

            // Log version info for loaded libraries
            if let Some(ref lib) = wchlink_dll {
                if let Ok(get_version) = unsafe { lib.get::<Symbol<unsafe extern "stdcall" fn() -> u32>>(b"CH375GetVersion") } {
                    if let Ok(get_driver_version) = unsafe { lib.get::<Symbol<unsafe extern "stdcall" fn() -> u32>>(b"CH375GetDrvVersion") } {
                        log::debug!(
                            "WCHLinkDLL.dll version {}, driver version {}",
                            unsafe { get_version() },
                            unsafe { get_driver_version() }
                        );
                    }
                }
            }

            if let Some(ref lib) = ch375_dll {
                if let Ok(get_version) = unsafe { lib.get::<Symbol<unsafe extern "stdcall" fn() -> u32>>(b"CH375GetVersion") } {
                    if let Ok(get_driver_version) = unsafe { lib.get::<Symbol<unsafe extern "stdcall" fn() -> u32>>(b"CH375GetDrvVersion") } {
                        log::debug!(
                            "CH375DLL.dll version {}, driver version {}",
                            unsafe { get_version() },
                            unsafe { get_driver_version() }
                        );
                    }
                }
            }

            Ok(CH375Libraries {
                wchlink_dll,
                ch375_dll,
            })
        });

        match result {
            Ok(libs) => Ok(libs),
            Err(e) => Err(Error::Custom(e.clone())),
        }
    }

    fn get_library_for_device(vid: u16, pid: u16) -> Result<&'static Library> {
        let libs = ensure_libraries_load()?;

        // For IAP mode devices, prefer CH375DLL.dll if available
        if vid == crate::probe::VENDOR_ID_IAP && pid == crate::probe::PRODUCT_ID_IAP {
            if let Some(ref lib) = libs.ch375_dll {
                return Ok(lib);
            }
        }

        // For normal mode devices, prefer WCHLinkDLL.dll if available
        if let Some(ref lib) = libs.wchlink_dll {
            return Ok(lib);
        }

        Err(Error::Custom("No suitable DLL found".to_string()))
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
        let lib = get_library_for_device(vid, pid)?;
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
        vid: u16,
        pid: u16,
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
        fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
            let lib = get_library_for_device(vid, pid)?;
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
                    // In IAP mode, the device does not have a descriptor, so skip checking vid and pid.
                    let mut descr = unsafe { core::mem::zeroed() };
                    let iap = vid == crate::probe::VENDOR_ID_IAP && pid == crate::probe::PRODUCT_ID_IAP;
                    if !iap {
                        let mut len = core::mem::size_of::<UsbDeviceDescriptor>() as u32;
                        let _ = unsafe { get_device_descriptor(i, &mut descr, &mut len) };
                    }

                    if iap || (descr.idVendor == vid && descr.idProduct == pid) {
                        if idx == nth {
                            log::debug!("Device #{}: {:04x}:{:04x}", i, vid, pid);
                            return Ok(Box::new(CH375USBDevice { index: i, vid, pid }));
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
            let lib = get_library_for_device(self.vid, self.pid)?;
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
            let lib = get_library_for_device(self.vid, self.pid)?;
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
            let lib = get_library_for_device(self.vid, self.pid).unwrap();

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
            if let Ok(lib) = get_library_for_device(self.vid, self.pid) {
                let close_device: Symbol<unsafe extern "stdcall" fn(u32)> =
                    unsafe { lib.get(b"CH375CloseDevice").unwrap() };
                unsafe {
                    close_device(self.index);
                }
            }
        }
    }
}
