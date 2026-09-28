use std::fmt::{self, Display};
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
        let devices: Vec<_> = nusb::list_devices()
            .wait()
            .map_err(crate::Error::Usb)?
            .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
            .collect();

        if nth >= devices.len() {
            return Err(crate::Error::ProbeNotFound);
        }

        let device_info = &devices[nth];
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
