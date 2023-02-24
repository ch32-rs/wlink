//! USB transport of WCH-Link

use std::time::Duration;

use rusb::DeviceHandle;

use crate::{
    commands::{Command, Response},
    error::Result,
};

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

const RAW_ENDPOINT_OUT: u8 = 0x02;
const RAW_ENDPOINT_IN: u8 = 0x82;

//  1a86:8010 1a86 WCH-Link  Serial: 0001A0000000
const USB_TIMEOUT_MS: u64 = 5000;

pub trait Transport {
    fn read_bytes(&mut self) -> Result<Vec<u8>>;

    fn write_bytes(&mut self, buf: &[u8]) -> Result<()>;

    fn send_command<C: Command>(&mut self, cmd: C) -> Result<C::Response> {
        let raw = cmd.to_raw();
        self.write_bytes(&raw)?;
        let resp = self.read_bytes()?;

        C::Response::from_raw(&resp)
    }

    fn read_from_data_channel(&mut self, n: usize) -> Result<Vec<u8>>;
    fn write_to_data_channel(&mut self, buf: &[u8]) -> Result<()>;
}

impl Transport for DeviceHandle<rusb::Context> {
    fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = [0u8; 64];

        let bytes_read =
            self.read_bulk(ENDPOINT_IN, &mut buf, Duration::from_millis(USB_TIMEOUT_MS))?;

        let resp = buf[..bytes_read].to_vec();
        log::trace!(
            "recv {} {}",
            hex::encode(&resp[..3]),
            hex::encode(&resp[3..])
        );
        Ok(resp)
    }

    fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        self.write_bulk(ENDPOINT_OUT, buf, Duration::from_millis(USB_TIMEOUT_MS))?;
        Ok(())
    }

    fn read_from_data_channel(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(n);
        let mut bytes_read = 0;
        while bytes_read < n {
            let mut chunk = vec![0u8; 64];
            let chunk_read = self.read_bulk(
                RAW_ENDPOINT_IN,
                &mut chunk,
                Duration::from_millis(USB_TIMEOUT_MS),
            )?;
            buf.extend_from_slice(&chunk[..chunk_read]);
            bytes_read += chunk_read;
        }
        if bytes_read != n {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        log::trace!("read data channel {} bytes", bytes_read);
        Ok(buf)
    }

    fn write_to_data_channel(&mut self, buf: &[u8]) -> Result<()> {
        todo!()
    }
}
