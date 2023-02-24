use std::time::Duration;

use rusb::{DeviceHandle, UsbContext};

use crate::{
    commands::{Command, Response},
    error::{Error, Result},
};

const USB_TIMEOUT_MS: u64 = 5000;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

const RAW_ENDPOINT_OUT: u8 = 0x02;
const RAW_ENDPOINT_IN: u8 = 0x82;

//  1a86:8010 1a86 WCH-Link  Serial: 0001A0000000

const VENDOR_ID: u16 = 0x1a86;
const PRODUCT_ID: u16 = 0x8010;

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
}

impl Drop for WchLink {
    fn drop(&mut self) {
        let _ = self.device_handle.release_interface(0);
    }
}

pub trait Transport {
    fn read_bytes(&mut self) -> Result<Vec<u8>>;

    fn write_bytes(&mut self, buf: &[u8]) -> Result<()>;

    fn send_command<C: Command>(&mut self, cmd: C) -> Result<C::Response> {
        let raw = cmd.to_raw();
        self.write_bytes(&raw)?;
        let resp = self.read_bytes()?;

        C::Response::from_raw(&resp)
    }
}

impl Transport for WchLink {
    fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = [0u8; 64];

        let bytes_read = self.device_handle.read_bulk(
            ENDPOINT_IN,
            &mut buf,
            Duration::from_millis(USB_TIMEOUT_MS),
        )?;

        let resp = buf[..bytes_read].to_vec();
        log::debug!(
            "recv {} {}",
            hex::encode(&resp[..3]),
            hex::encode(&resp[3..])
        );
        Ok(resp)
    }

    fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        log::debug!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        self.device_handle
            .write_bulk(ENDPOINT_OUT, buf, Duration::from_millis(USB_TIMEOUT_MS))?;
        Ok(())
    }
}
