//! WchLink device type

use std::time::Duration;

use log::info;
use rusb::{DeviceHandle, UsbContext};

use crate::{
    commands::{control::ProbeInfo, ChipUID, RawCommand, Response},
    usb_device::{self, USBDeviceBackend},
    Error, Result, RiscvChip,
};

pub const VENDOR_ID: u16 = 0x1a86;
pub const PRODUCT_ID: u16 = 0x8010;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

const DATA_ENDPOINT_OUT: u8 = 0x02;
const DATA_ENDPOINT_IN: u8 = 0x82;

const VENDOR_ID_DAP: u16 = 0x1a86;
const PRODUCT_ID_DAP: u16 = 0x8012;

const ENDPOINT_OUT_DAP: u8 = 0x02;
// const ENDPOINT_IN_DAP: u8 = 0x83;

const USB_TIMEOUT_MS: u64 = 5000;

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

pub struct WchLink {
    pub(crate) device_handle: Box<dyn usb_device::USBDeviceBackend>,
    pub chip: Option<ChipInfo>,
    pub probe: Option<ProbeInfo>,
    pub(crate) speed: crate::commands::Speed,
}

impl WchLink {
    pub fn open_nth(nth: usize) -> Result<Self> {
        let mut dev = usb_device::USBDevice::open_nth(VENDOR_ID, PRODUCT_ID, nth)?;

        dev.set_timeout(Duration::from_millis(USB_TIMEOUT_MS));

        Ok(Self {
            device_handle: dev,
            chip: None,
            probe: None,
            speed: Default::default(),
        })
    }

    fn write_command_ep(&mut self, buf: &[u8]) -> Result<()> {
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        self.device_handle.write_endpoint(ENDPOINT_OUT, buf)?;
        Ok(())
    }

    fn read_command_ep(&mut self) -> Result<Vec<u8>> {
        let mut buf = [0u8; 64];

        let bytes_read = self.device_handle.read_endpoint(ENDPOINT_IN, &mut buf)?;
        let resp = buf[..bytes_read].to_vec();
        log::trace!(
            "recv {} {}",
            hex::encode(&resp[..3]),
            hex::encode(&resp[3..])
        );
        Ok(resp)
    }

    pub(crate) fn read_data_ep(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(n);
        let mut bytes_read = 0;
        while bytes_read < n {
            let mut chunk = vec![0u8; 64];
            let chunk_read = self
                .device_handle
                .read_endpoint(DATA_ENDPOINT_IN, &mut chunk)?;
            buf.extend_from_slice(&chunk[..chunk_read]);
            bytes_read += chunk_read;
        }
        if bytes_read != n {
            return Err(crate::Error::InvalidPayloadLength);
        }
        log::trace!("read data ep {} bytes", bytes_read);
        if bytes_read <= 10 {
            log::trace!("recv data {}", hex::encode(&buf[..bytes_read]));
        }
        if bytes_read != n {
            log::warn!("read data ep {} bytes", bytes_read);
            return Err(Error::InvalidPayloadLength);
        }
        Ok(buf[..n].to_vec())
    }

    pub(crate) fn write_data_ep(&mut self, buf: &[u8], packet_len: usize) -> Result<()> {
        self.write_data_ep_with_progress(buf, packet_len, &|_| {})
    }

    pub(crate) fn write_data_ep_with_progress(
        &mut self,
        buf: &[u8],
        packet_len: usize,
        progress_callback: &dyn Fn(usize),
    ) -> Result<()> {
        for chunk in buf.chunks(packet_len) {
            let mut chunk = chunk.to_vec();
            progress_callback(chunk.len());
            if chunk.len() < packet_len {
                chunk.resize(packet_len, 0xff);
            }
            log::trace!("write data ep {} bytes", chunk.len());
            self.device_handle
                .write_endpoint(DATA_ENDPOINT_OUT, &chunk)?;
        }
        log::trace!("write data ep total {} bytes", buf.len());
        Ok(())
    }

    pub fn send_command<C: crate::commands::Command>(&mut self, cmd: C) -> Result<C::Response> {
        log::trace!("send command: {:?}", cmd);
        let raw = cmd.to_raw();
        self.write_command_ep(&raw)?;
        let resp = self.read_command_ep()?;

        C::Response::from_raw(&resp)
    }

    pub fn set_speed(&mut self, speed: crate::commands::Speed) {
        self.speed = speed;
    }
}

/// Switch from DAP mode to RV mode
// ref: https://github.com/cjacker/wchlinke-mode-switch/blob/main/main.c
pub fn try_switch_from_rv_to_dap<USB: USBDeviceBackend>(nth: usize) -> Result<()> {
    let mut dev = USB::open_nth(VENDOR_ID, PRODUCT_ID, nth)?;
    info!("Switch mode for WCH-LinkRV");

    let mut dev = WchLink {
        device_handle: dev,
        // fake info
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
pub fn try_switch_from_dap_to_rv<USB: USBDeviceBackend>(nth: usize) -> Result<()> {
    let mut dev = USB::open_nth(VENDOR_ID_DAP, PRODUCT_ID_DAP, nth)?;
    info!("Switch mode for WCH-LinkDAP");

    let buf = [0x81, 0xff, 0x01, 0x52];
    log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
    let _ = dev.write_endpoint(ENDPOINT_OUT_DAP, &buf);

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
