use libloading::os::windows::*;
use std::fmt::{self, Display};
use std::sync::OnceLock;

use super::*;
use crate::Error;

static WCH_LINK_DLL: OnceLock<std::result::Result<Library, String>> = OnceLock::new();

fn ensure_library_load() -> Result<&'static Library> {
    let result = WCH_LINK_DLL.get_or_init(|| {
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
