//! WchLink device type

use log::info;
use rusb::{DeviceHandle, UsbContext};

use crate::{
    commands::{control::ProbeInfo, ChipUID, RawCommand, Response},
    transport::Transport,
    Result, RiscvChip,
};

const VENDOR_ID: u16 = 0x1a86;
const PRODUCT_ID: u16 = 0x8010;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

const VENDOR_ID_DAP: u16 = 0x1a86;
const PRODUCT_ID_DAP: u16 = 0x8012;

const ENDPOINT_OUT_DAP: u8 = 0x02;
// const ENDPOINT_IN_DAP: u8 = 0x83;

/// Attached chip information
#[derive(Debug, Clone)]
pub struct ChipInfo {
    /// UID
    pub uid: Option<ChipUID>,
    pub chip_family: RiscvChip,
    /// 0x303305x4 like chip_id, In SDK, `DBGMCU_GetCHIPID` is used to get this value
    pub chip_id: u32,
    /// parsed marchid: WCH-V4B, WCH-V4F...
    pub march: Option<String>,
}

#[derive(Debug)]
pub struct WchLink {
    pub(crate) device_handle: DeviceHandle<rusb::Context>,
    pub chip: Option<ChipInfo>,
    pub probe: Option<ProbeInfo>,
    pub(crate) speed: crate::commands::Speed,
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
            .map_or(Err(crate::error::Error::ProbeNotFound), Ok);

        // check if there's a device with the DAP VID/PID
        let device = match device {
            Ok(device) => device,
            Err(e) => {
                if open_usb_device(VENDOR_ID_DAP, PRODUCT_ID_DAP, nth).is_ok() {
                    return Err(crate::error::Error::ProbeModeNotSupported);
                }
                return Err(e);
            }
        };

        let mut device_handle = device.open()?;

        let config = device.active_config_descriptor()?;

        // let descriptor = device.device_descriptor()?;

        log::trace!("Device: {:?}", &device);

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
            probe: None,
            speed: Default::default(),
        })
    }

    pub fn send_command<C: crate::commands::Command>(&mut self, cmd: C) -> Result<C::Response> {
        log::debug!("send command: {:?}", cmd);
        let raw = cmd.to_raw();
        self.device_handle.write_command_endpoint(&raw)?;
        let resp = self.device_handle.read_command_endpoint()?;

        C::Response::from_raw(&resp)
    }

    pub fn set_speed(&mut self, speed: crate::commands::Speed) {
        self.speed = speed;
    }
}

impl Drop for WchLink {
    fn drop(&mut self) {
        let _ = self.device_handle.release_interface(0);
    }
}

/// Switch from DAP mode to RV mode
// ref: https://github.com/cjacker/wchlinke-mode-switch/blob/main/main.c
pub fn try_switch_from_rv_to_dap(nth: usize) -> Result<()> {
    let dev = open_usb_device(VENDOR_ID, PRODUCT_ID, nth)?;
    info!("Switch mode WCH-LinkRV {:?}", dev.device());

    let mut dev = WchLink {
        device_handle: dev,
        chip: None,
        probe: None,
        speed: Default::default(),
    };
    let info = dev.probe_info()?;
    info!("probe info: {:?}", info);
    if info.variant.support_switch_mode() {
        let _ = dev.send_command(RawCommand::<0xff>(vec![0x41]));
        Ok(())
    } else {
        log::error!("Cannot switch mode for WCH-LinkRV: not supported");
        Err(crate::Error::Custom(
            "WCH-Link-CH549 does not support mode switch".into(),
        ))
    }
}

/// Switch from RV mode to DAP mode
pub fn try_switch_from_dap_to_rv(nth: usize) -> Result<()> {
    let dev = open_usb_device(VENDOR_ID_DAP, PRODUCT_ID_DAP, nth)?;
    info!("Switch mode for WCH-LinkDAP {:?}", dev.device());

    let buf = [0x81, 0xff, 0x01, 0x52];
    log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
    let _ = dev.write_bulk(
        ENDPOINT_OUT_DAP,
        &buf,
        std::time::Duration::from_millis(5000),
    );

    Ok(())
}

/// Check connected USB device
pub fn check_usb_device() -> Result<()> {
    let context = rusb::Context::new()?;
    log::trace!("Acquired libusb context.");

    for device in context.devices()?.iter() {
        let desc = device.device_descriptor()?;
        if desc.vendor_id() == VENDOR_ID && desc.product_id() == PRODUCT_ID {
            log::info!("Found WCH-LinkRV, {:?}", device);
        } else if desc.vendor_id() == VENDOR_ID_DAP && desc.product_id() == PRODUCT_ID_DAP {
            log::info!("Found WCH-LinkDAP, {:?}", device);
        }
    }

    Ok(())
}

fn open_usb_device(
    vendor_id: u16,
    produce_id: u16,
    nth: usize,
) -> Result<DeviceHandle<rusb::Context>> {
    let context = rusb::Context::new()?;
    log::trace!("Acquired libusb context.");

    let device = context
        .devices()?
        .iter()
        .filter(|device| {
            device
                .device_descriptor()
                .map(|desc| desc.vendor_id() == vendor_id && desc.product_id() == produce_id)
                .unwrap_or(false)
        })
        .nth(nth)
        .map_or(Err(crate::error::Error::ProbeNotFound), Ok)?;

    let mut device_handle = device.open()?;

    device_handle.claim_interface(0)?;

    log::trace!("Claimed interface 0 of USB device.");

    // TODO: endpoint check
    // let config = device.active_config_descriptor()?;

    Ok(device_handle)
}
