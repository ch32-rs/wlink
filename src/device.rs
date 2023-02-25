use rusb::{DeviceHandle, UsbContext};

use crate::{commands::Response, error::Error, transport::Transport, Result};

const VENDOR_ID: u16 = 0x1a86;
const PRODUCT_ID: u16 = 0x8010;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

#[derive(Debug)]
pub struct WchLink {
    pub(crate) device_handle: DeviceHandle<rusb::Context>,
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
            .map_or(Err(Error::UsbNoSuchDevice), Ok)?;

        let mut device_handle = device.open()?;

        let config = device.active_config_descriptor()?;

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
            return Err(Error::UsbNoSuchEndpoints);
        }

        Ok(Self { device_handle })
    }

    pub fn send_command<C: crate::commands::Command>(&mut self, cmd: C) -> Result<C::Response> {
        let raw = cmd.to_raw();
        self.device_handle.write_command_bytes(&raw)?;

        let mut buf = vec![0u8; 64];
        let len = self.device_handle.read_command_bytes(&mut buf)?;

        C::Response::from_raw(&buf[..len])
    }
}

impl Drop for WchLink {
    fn drop(&mut self) {
        let _ = self.device_handle.release_interface(0);
    }
}
