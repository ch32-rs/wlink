//! The probe - WCH-Link

use crate::commands::{self, Response};
use crate::Result;
use crate::{
    commands::control::ProbeInfo,
    usb_device::{USBDevice, USBDeviceBackend},
};

pub const VENDOR_ID: u16 = 0x1a86;
pub const PRODUCT_ID: u16 = 0x8010;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

const VENDOR_ID_DAP: u16 = 0x1a86;
const PRODUCT_ID_DAP: u16 = 0x8012;

const ENDPOINT_OUT_DAP: u8 = 0x02;

pub struct WchLink {
    pub(crate) device: Box<dyn USBDeviceBackend>,
    pub probe: Option<ProbeInfo>,
}

impl WchLink {
    pub fn open_nth(nth: usize) -> Result<Self> {
        let device = USBDevice::open_nth(VENDOR_ID, PRODUCT_ID, nth)?;
        Ok(Self {
            device,
            probe: None,
        })
    }

    pub fn probe_info(&mut self) -> Result<ProbeInfo> {
        let info = self.send_command(commands::control::GetProbeInfo)?;
        log::info!("{}", info);
        self.probe = Some(info);
        Ok(info)
    }

    /// Switch from DAP mode to RV mode
    // ref: https://github.com/cjacker/wchlinke-mode-switch/blob/main/main.c
    pub fn switch_from_rv_to_dap(nth: usize) -> Result<()> {
        let dev = USBDevice::open_nth(VENDOR_ID, PRODUCT_ID, nth)?;
        log::info!(
            "Switch mode WCH-LinkRV {:04x}:{:04x} #{}",
            VENDOR_ID,
            PRODUCT_ID,
            nth
        );

        todo!()
    }

    fn write_raw_cmd(&mut self, buf: &[u8]) -> Result<()> {
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        self.device.write_endpoint(ENDPOINT_OUT, buf)?;
        Ok(())
    }

    fn read_raw_cmd_resp(&mut self) -> Result<Vec<u8>> {
        let mut buf = [0u8; 64];
        let bytes_read = self.device.read_endpoint(ENDPOINT_IN, &mut buf)?;

        let resp = buf[..bytes_read].to_vec();
        log::trace!(
            "recv {} {}",
            hex::encode(&resp[..3]),
            hex::encode(&resp[3..])
        );
        Ok(resp)
    }

    pub(crate) fn send_command<C: crate::commands::Command>(
        &mut self,
        cmd: C,
    ) -> Result<C::Response> {
        log::trace!("send command: {:?}", cmd);
        let raw = cmd.to_raw();
        self.write_raw_cmd(&raw)?;
        let resp = self.read_raw_cmd_resp()?;

        C::Response::from_raw(&resp)
    }
}
