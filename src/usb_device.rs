//! USB Device abstraction - The USB Device of WCH-Link.

use crate::Result;
use std::{fmt::Display, time::Duration};

pub trait USBDeviceBackend: Sized {
    fn list_devices(vid: u16, pid: u16) -> Result<Vec<impl Display>>;

    fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Self>;

    fn set_timeout(&mut self, _timeout: Duration) {}

    fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize>;

    fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()>;
}

pub use libusb::USBDevice;

mod libusb {
    use super::*;
    use rusb::{DeviceHandle, Speed, UsbContext};

    #[derive(Debug)]
    pub struct USBDevice {
        handle: DeviceHandle<rusb::Context>,
        timeout: Duration,
    }

    impl USBDeviceBackend for USBDevice {
        fn list_devices(vid: u16, pid: u16) -> Result<Vec<impl Display>> {
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

        fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Self> {
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

            Ok(Self {
                handle,
                timeout: Duration::from_millis(5000),
            })
        }

        fn set_timeout(&mut self, timeout: Duration) {
            self.timeout = timeout;
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

mod wchlink_driver {
    // TODO
}
