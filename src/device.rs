use rusb::{DeviceHandle, UsbContext};

use crate::{
    error::{Result},
    transport::Transport,
};

const VENDOR_ID: u16 = 0x1a86;
const PRODUCT_ID: u16 = 0x8010;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

#[derive(Debug)]
pub struct WchLink {
    device_handle: DeviceHandle<rusb::Context>,
}

impl WchLink {
    fn new(device_handle: DeviceHandle<rusb::Context>) -> Self {
        Self { device_handle }
    }

    pub fn open_nth(nth: usize) -> Result<Self> {
        let context = rusb::Context::new()?;

        log::debug!("Acquired libusb context.");
        let device = context
            .devices()?
            .iter()
            .filter(|device| {
                device
                    .device_descriptor()
                    .map(|desc| desc.vendor_id() == VENDOR_ID && desc.product_id() == PRODUCT_ID)
                    .unwrap_or(false)
            })
            .nth(nth)
            .map_or(
                Err(crate::error::Error::Custom("No such device".to_string())),
                Ok,
            )?;

        let mut device_handle = device.open()?;

        log::debug!("Aquired handle for probe");

        let config = device.active_config_descriptor()?;

        log::debug!("Active config descriptor: {:?}", &config);

        let descriptor = device.device_descriptor()?;

        log::debug!("Device descriptor: {:?}", &descriptor);

        device_handle.claim_interface(0)?;

        log::debug!("Claimed interface 0 of USB device.");

        let mut endpoint_out = false;
        let mut endpoint_in = false;

        if let Some(interface) = config.interfaces().next() {
            if let Some(descriptor) = interface.descriptors().next() {
                for endpoint in descriptor.endpoint_descriptors() {
                    if endpoint.address() == ENDPOINT_OUT {
                        endpoint_out = true;
                    }

                    if endpoint.address() == ENDPOINT_IN {
                        endpoint_in = true;
                    }
                }
            }
        }

        if !endpoint_out || !endpoint_in {
            return Err(crate::error::Error::Custom(
                "Could not find endpoints".to_string(),
            ));
        }

        Ok(Self { device_handle })
    }

    pub fn send_command<C: crate::commands::Command>(&mut self, cmd: C) -> Result<C::Response> {
        self.device_handle.send_command(cmd)
    }
}

impl Drop for WchLink {
    fn drop(&mut self) {
        let _ = self.device_handle.release_interface(0);
    }
}
