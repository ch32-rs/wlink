//! USB Device abstraction - The USB Device of WCH-Link.

use crate::Result;
use std::{fmt::Display, time::Duration};

pub trait USBDeviceBackend {
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
        ch375_driver::USBDevice::open_nth(vid, pid, nth)
            .or_else(|_| libusb::USBDevice::open_nth(vid, pid, nth))
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    {
        libusb::USBDevice::open_nth(vid, pid, nth)
    }
}

pub fn list_devices(vid: u16, pid: u16) -> Result<Vec<String>> {
    let mut ret = vec![];
    ret.extend(
        libusb::list_libusb_devices(vid, pid)?
            .into_iter()
            .map(|s| s.to_string()),
    );

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        ret.extend(
            ch375_driver::list_devices(vid, pid)?
                .into_iter()
                .map(|s| s.to_string()),
        );
    }

    Ok(ret)
}

pub use libusb::USBDevice;
// pub use ch375_driver::USBDevice;

mod libusb {
    use super::*;
    use rusb::{DeviceHandle, Speed, UsbContext};

    pub fn list_libusb_devices(vid: u16, pid: u16) -> Result<Vec<impl Display>> {
        let context = rusb::Context::new()?;
        let devices = context.devices()?;
        let mut result = vec![];
        for (i, device) in devices.iter().enumerate() {
            let device_desc = device.device_descriptor()?;
            if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
                result.push(format!(
                    "<WCH-Link#{}> Bus {:03} Device {:03} ID {:04x}:{:04x} {}",
                    i,
                    device.bus_number(),
                    device.address(),
                    device_desc.vendor_id(),
                    device_desc.product_id(),
                    get_speed(device.speed())
                ));
            }
        }
        Ok(result)
    }

    #[derive(Debug)]
    pub struct USBDevice {
        handle: DeviceHandle<rusb::Context>,
        timeout: Duration,
    }

    impl USBDeviceBackend for USBDevice {
        fn set_timeout(&mut self, timeout: Duration) {
            self.timeout = timeout;
        }

        fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
            println!("fuck");
            let context = rusb::Context::new()?;
            let devices = context.devices()?;
            let mut result = vec![];
            for device in devices.iter() {
                let device_desc = device.device_descriptor()?;
                if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
                    result.push(device);
                }
            }
            if nth >= result.len() {
                return Err(crate::Error::ProbeNotFound);
            }
            let device = result.remove(nth);
            let mut handle = device.open()?;

            log::trace!("Device: {:?}", &device);

            let desc = device.device_descriptor()?;
            let serial_number = handle.read_serial_number_string_ascii(&desc)?;
            log::debug!("Serial number: {:?}", serial_number);

            handle.claim_interface(0)?;

            Ok(Box::new(USBDevice {
                handle,
                timeout: Duration::from_millis(5000),
            }))
        }

        fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize> {
            let bytes_read = self.handle.read_bulk(ep, buf, self.timeout)?;
            Ok(bytes_read)
        }

        fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()> {
            self.handle.write_bulk(ep, buf, self.timeout)?;
            Ok(())
        }
    }

    impl Drop for USBDevice {
        fn drop(&mut self) {
            let _ = self.handle.release_interface(0);
        }
    }

    fn get_speed(speed: Speed) -> &'static str {
        match speed {
            Speed::SuperPlus => "USB-SS+ 10000 Mbps",
            Speed::Super => "USB-SS 5000 Mbps",
            Speed::High => "USB-HS 480 Mbps",
            Speed::Full => "USB-FS 12 Mbps",
            Speed::Low => "USB-LS 1.5 Mbps",
            _ => "(unknown)",
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod ch375_driver {
    use libloading::os::windows::*;

    use super::*;
    use crate::Error;

    static mut CH375_DRIVER: Option<Library> = None;

    fn ensure_library_load() -> Result<&'static Library> {
        unsafe {
            if CH375_DRIVER.is_none() {
                CH375_DRIVER = Some(
                    Library::new("WCHLinkDLL.dll")
                        .map_err(|_| Error::Custom("WCHLinkDLL.dll not found".to_string()))?,
                );
                let lib = CH375_DRIVER.as_ref().unwrap();
                let get_version: Symbol<unsafe extern "stdcall" fn() -> u32> =
                    { lib.get(b"CH375GetVersion").unwrap() };
                let get_driver_version: Symbol<unsafe extern "stdcall" fn() -> u32> =
                    { lib.get(b"CH375GetDrvVersion").unwrap() };

                log::debug!(
                    "DLL version {}, driver version {}",
                    get_version(),
                    get_driver_version()
                );
                Ok(lib)
            } else {
                Ok(CH375_DRIVER.as_ref().unwrap())
            }
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
                let vid = descr.idVendor;
                let pid = descr.idProduct;

                log::debug!("Device #{}: {:04x}:{:04x}", i, vid, pid);
                if vid == vid && pid == pid {
                    ret.push(format!("<WCH-Link#{}> {:04x}:{:04x}", i, vid, pid));
                }
                unsafe { close_device(i) };
            }
        }
    }

    pub struct USBDevice {
        index: u32,
    }

    impl USBDeviceBackend for USBDevice {
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
                    let vid = descr.idVendor;
                    let pid = descr.idProduct;

                    log::debug!("Device #{}: {:04x}:{:04x}", i, vid, pid);
                    if vid == vid && pid == pid {
                        if idx == nth {
                            return Ok(Box::new(USBDevice { index: i }));
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
            if ret {
                Ok(())
            } else {
                Err(Error::Driver)
            }
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

    impl Drop for USBDevice {
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
