//! WchLink device type

use rusb::{DeviceHandle, UsbContext};

use crate::{
    commands::{ChipId, Response},
    transport::Transport,
    Result, RiscvChip,
};

const VENDOR_ID: u16 = 0x1a86;
const PRODUCT_ID: u16 = 0x8010;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

/// Attached chip information
#[derive(Debug)]
pub struct ChipInfo {
    pub uid: ChipId,
    pub flash_protected: bool,
    pub chip_family: RiscvChip,
    pub chip_type: u32,
    /// parsed marchid: WCH-V4B, WCH-V4F...
    pub march: Option<String>,

    pub flash_size: u32,
    pub page_size: u32,
    pub memory_start_addr: u32,
    // Fields for ROM/RAM split
    pub sram_code_mode: u8,
    pub(crate) rom_kb: u32,
    pub(crate) ram_kb: u32,
}

#[derive(Debug)]
pub struct WchLink {
    pub(crate) device_handle: DeviceHandle<rusb::Context>,
    pub chip: Option<ChipInfo>,
}

impl WchLink {
    pub fn open_nth(nth: usize) -> Result<Self> {
        let context = rusb::Context::new()?;
        log::trace!("Acquired libusb context.");

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

        let config = device.active_config_descriptor()?;

        let descriptor = device.device_descriptor()?;

        log::trace!("Device descriptor: {:?}", &descriptor);

        device_handle.claim_interface(0)?;

        log::trace!("Claimed interface 0 of USB device.");

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

        Ok(Self {
            device_handle,
            chip: None,
        })
    }

    pub fn send_command<C: crate::commands::Command>(&mut self, cmd: C) -> Result<C::Response> {
        let raw = cmd.to_raw();
        self.device_handle.write_command_endpoint(&raw)?;
        let resp = self.device_handle.read_command_endpoint()?;

        C::Response::from_raw(&resp)
    }
}

impl Drop for WchLink {
    fn drop(&mut self) {
        let _ = self.device_handle.release_interface(0);
    }
}
