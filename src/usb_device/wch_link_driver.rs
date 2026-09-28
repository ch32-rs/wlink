use std::fmt::{self, Display};
use windows::core::{GUID, HRESULT, PCWSTR};
use windows::Win32::{
    Devices::DeviceAndDriverInstallation::*,
    Devices::Usb::*,
    Foundation::*,
    Storage::FileSystem::*,
    System::IO::*,
};

use super::*;
use crate::Error;

static GUID_DEVINTERFACE_WCH_LINK: GUID = GUID::from_values(0xF8D5EDCA, 0xB647, 0x4E9C, [0x9B, 0xD3, 0xA5, 0xBD, 0x23, 0x28, 0xD5, 0x5C]);
static WCH_LINK_DRIVER_IOCTL: u32 = 0x00223CDC;

struct DeviceInfoSet {
    hdevinfo: HDEVINFO,
}

impl DeviceInfoSet {
    fn new() -> windows::core::Result<Self> {
        Ok(DeviceInfoSet {
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

    fn enum_device_interfaces(&mut self, index: u32) -> windows::core::Result<SP_DEVICE_INTERFACE_DATA> {
        let mut device_interface = SP_DEVICE_INTERFACE_DATA::default();
        device_interface.cbSize = core::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;

        unsafe {
            SetupDiEnumDeviceInterfaces(
                self.hdevinfo,
                None,
                &GUID_DEVINTERFACE_WCH_LINK,
                index,
                &mut device_interface,
            )
        }?;

        Ok(device_interface)
    }

    fn get_device_interface_detail(&mut self, device_interface: &SP_DEVICE_INTERFACE_DATA) -> windows::core::Result<Vec<u16>> {
        let mut required_size = 0u32;

        match unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                self.hdevinfo,
                device_interface,
                None,
                0,
                Some(&mut required_size),
                None,
            )
        } {
            Ok(_) => {}
            Err(e) => {
                if e.code() != HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0) {
                    return Err(e.into());
                }
            }
        }

        let device_path_offset = core::mem::offset_of!(SP_DEVICE_INTERFACE_DETAIL_DATA_W, DevicePath);
        let length = (required_size as usize - device_path_offset) / core::mem::size_of::<u16>();
        let mut device_path = vec![0u16; length];

        unsafe {
            let layout = std::alloc::Layout::from_size_align(
                required_size as usize,
                core::mem::align_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>(),
            ).unwrap();
            let device_interface_detail =
                std::alloc::alloc(layout).cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
            (*device_interface_detail).cbSize = core::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

            SetupDiGetDeviceInterfaceDetailW(
                self.hdevinfo,
                device_interface,
                Some(device_interface_detail),
                required_size,
                None,
                None,
            )?;

            core::ptr::copy_nonoverlapping(
                device_interface_detail.cast::<u8>().add(device_path_offset),
                device_path.as_mut_ptr().cast::<u8>(),
                length * core::mem::size_of::<u16>(),
            );

            std::alloc::dealloc(device_interface_detail.cast::<u8>(), layout);
        }

        Ok(device_path)
    }
}

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.hdevinfo);
        }
    }
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

        Ok(WCHLinkUSBDevice { handle })
    }

    fn command(&mut self, buffer: &mut [u8], in_size: usize, out_size: usize) -> Result<usize> {
        let mut bytes_returned: u32 = 0;
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

    fn get_device_descriptor(&mut self) -> Result<USB_DEVICE_DESCRIPTOR> {
        let mut buffer = vec![0u8; 8 + core::mem::size_of::<USB_DEVICE_DESCRIPTOR>()];
        buffer[0..16].copy_from_slice(&[
            0x04, 0x00, 0x00, 0x00,
            0x08, 0x00, 0x00, 0x00,
            0x80, 0x06, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00,
        ]);
        buffer[14..16].copy_from_slice(&(core::mem::size_of::<USB_DEVICE_DESCRIPTOR>() as u16).to_ne_bytes());

        let len = buffer.len();
        self.command(&mut buffer, len, len)?;

        let descriptor: USB_DEVICE_DESCRIPTOR = unsafe {
            core::ptr::read(buffer[8..].as_ptr() as *const _)
        };
        Ok(descriptor)
    }
}

impl USBDeviceBackend for WCHLinkUSBDevice {
    fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
        let mut device_info_set = DeviceInfoSet::new()?;

        let device_interface = device_info_set.enum_device_interfaces(nth as u32)?;

        let device_path = device_info_set.get_device_interface_detail(&device_interface)?;

        let mut device = WCHLinkUSBDevice::open_device_path(&device_path)?;

        let device_descriptor = device.get_device_descriptor()?;
        if (device_descriptor.idVendor != vid) || (device_descriptor.idProduct != pid) {
            return Err(Error::ProbeNotFound);
        }

        Ok(Box::new(device))
    }

    fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize> {
        let mut buffer = vec![0u8; 8 + buf.len()];
        buffer[0] = (ep & 0x7F) - 1;
        buffer[2] = 0x01;
        buffer[4..8].copy_from_slice(&(buf.len() as u32).to_ne_bytes());

        let len = buffer.len();
        self.command(&mut buffer, 8, len)?;

        let length = u32::from_ne_bytes(buffer[4..8].try_into().unwrap()) as usize;
        if length > buf.len() {
            return Err(Error::InvalidPayloadLength);
        }

        buf[..length].copy_from_slice(&buffer[8..8 + length]);
        Ok(length)
    }

    fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()> {
        let mut buffer = vec![0u8; 8 + buf.len()];
        buffer[0] = ep - 1;
        buffer[2] = 0x02;
        buffer[4..8].copy_from_slice(&(buf.len() as u32).to_ne_bytes());
        buffer[8..8 + buf.len()].copy_from_slice(buf);

        let len = buffer.len();
        self.command(&mut buffer,len, 8)?;

        Ok(())
    }

    fn set_timeout(&mut self, timeout: Duration) {
        let mut buffer = [
            0x0F, 0x00, 0x00, 0x00,
            0x10, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let ds = timeout.as_millis() as u32;
        buffer[8..12].copy_from_slice(&ds.to_ne_bytes());
        buffer[12..16].copy_from_slice(&ds.to_ne_bytes());
        buffer[16..20].copy_from_slice(&ds.to_ne_bytes());
        buffer[20..24].copy_from_slice(&ds.to_ne_bytes());

        let len = buffer.len();
        self.command(&mut buffer, len, len).unwrap();
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

pub fn list_devices(vid: u16, pid: u16) -> Result<Vec<impl Display>> {
    let mut result: Vec<String> = vec![];

    let mut device_info_set = DeviceInfoSet::new()?;

    let mut index = 0u32;

    loop {
        match device_info_set.enum_device_interfaces(index) {
            Ok(device_interface) => {
                let device_path = device_info_set.get_device_interface_detail(&device_interface)?;

                let mut device = WCHLinkUSBDevice::open_device_path(&device_path)?;

                let device_descriptor = device.get_device_descriptor()?;
                if (device_descriptor.idVendor != vid) || (device_descriptor.idProduct != pid) {
                    index += 1;
                    continue;
                }

                result.push(format!(
                    "<WCH-Link#{} WCHLinkDriver device> WCHLinkDriver Device {:04x}:{:04x} ({})",
                    index,
                    vid,
                    pid,
                    String::from_utf16_lossy(device_path.as_slice().split_last().unwrap().1),
                ));

                index += 1;
            }
            Err(e) => {
                if e.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0) {
                    break;
                } else {
                    return Err(e.into());
                }
            }
        }
    }

    Ok(result)
}
