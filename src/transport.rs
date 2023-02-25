//! USB transport of WCH-Link

use std::time::Duration;

use rusb::DeviceHandle;

use crate::Result;

/// Byte transportation of WCH-Link data and commands
pub trait Transport {
    /// Pull some bytes from command channel into the specified buffer, returning how many bytes were read.
    fn read_command_bytes(&mut self, buf: &mut [u8]) -> Result<usize>;
    /// Write a buffer into command transport channel, returning how many bytes were written.
    fn write_command_bytes(&mut self, buf: &[u8]) -> Result<usize>;
    /// Pull some bytes from data channel into the specified buffer, returning how many bytes were read.
    fn read_data_bytes(&mut self, buf: &mut [u8]) -> Result<usize>;
    /// Write a buffer into data transport channel, returning how many bytes were written.
    fn write_data_bytes(&mut self, buf: &[u8]) -> Result<usize>;
}

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

const RAW_ENDPOINT_OUT: u8 = 0x02;
const RAW_ENDPOINT_IN: u8 = 0x82;

//  1a86:8010 1a86 WCH-Link  Serial: 0001A0000000
const USB_TIMEOUT_MS: u64 = 5000;

/// Transport by USB context
impl Transport for DeviceHandle<rusb::Context> {
    fn read_command_bytes(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = self.read_bulk(ENDPOINT_IN, buf, Duration::from_millis(USB_TIMEOUT_MS))?;

        log::trace!("recv {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        Ok(len)
    }

    fn write_command_bytes(&mut self, buf: &[u8]) -> Result<usize> {
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        let len = self.write_bulk(ENDPOINT_OUT, buf, Duration::from_millis(USB_TIMEOUT_MS))?;
        Ok(len)
    }

    // continously reads until buf is full
    fn read_data_bytes(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut bytes_read = 0;
        while bytes_read < buf.len() {
            let mut chunk = &mut buf[bytes_read..];
            let chunk_read = self.read_bulk(
                RAW_ENDPOINT_IN,
                &mut chunk,
                Duration::from_millis(USB_TIMEOUT_MS),
            )?;
            bytes_read += chunk_read;
        }
        if bytes_read != buf.len() {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        log::trace!("read data channel {} bytes", bytes_read);
        Ok(bytes_read)
    }

    fn write_data_bytes(&mut self, buf: &[u8]) -> Result<usize> {
        let mut bytes_written = 0;
        const CHUNK: usize = 64;
        while bytes_written < buf.len() {
            let chunk = &buf[bytes_written..(bytes_written + CHUNK).min(buf.len())];
            self.write_bulk(
                RAW_ENDPOINT_OUT,
                chunk,
                Duration::from_millis(USB_TIMEOUT_MS),
            )?;
            bytes_written += chunk.len();
        }
        log::trace!("write data channel {} bytes", bytes_written);
        Ok(bytes_written)
    }
}
