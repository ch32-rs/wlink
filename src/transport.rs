//! USB transport layer of WCH-Link

use std::time::Duration;

use rusb::DeviceHandle;

use crate::{Error, Result};

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

const DATA_ENDPOINT_OUT: u8 = 0x02;
const DATA_ENDPOINT_IN: u8 = 0x82;

// 1a86:8010 1a86 WCH-Link  Serial: 0001A0000000
const USB_TIMEOUT_MS: u64 = 5000;

/// A Transport type represents the USB connection to the WCH-Link.
/// With ep 0x01 and 0x81 for commands and 0x02 and 0x82 for raw data.
pub(crate) trait Transport {
    fn read_command_endpoint(&mut self) -> Result<Vec<u8>>;

    fn write_command_endpoint(&mut self, buf: &[u8]) -> Result<()>;

    fn read_data_endpoint(&mut self, n: usize) -> Result<Vec<u8>>;

    fn write_data_endpoint(&mut self, buf: &[u8], packet_len: usize) -> Result<()>;
}

impl Transport for DeviceHandle<rusb::Context> {
    fn read_command_endpoint(&mut self) -> Result<Vec<u8>> {
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

    fn write_command_endpoint(&mut self, buf: &[u8]) -> Result<()> {
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        self.write_bulk(ENDPOINT_OUT, buf, Duration::from_millis(USB_TIMEOUT_MS))?;
        Ok(())
    }

    fn read_data_endpoint(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(n);
        let mut bytes_read = 0;
        while bytes_read < n {
            let mut chunk = vec![0u8; 64];
            let chunk_read = self.read_bulk(
                DATA_ENDPOINT_IN,
                &mut chunk,
                Duration::from_millis(USB_TIMEOUT_MS),
            )?;
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

    // pWriteData
    fn write_data_endpoint(&mut self, buf: &[u8], packet_len: usize) -> Result<()> {
        for chunk in buf.chunks(packet_len) {
            let mut chunk = chunk.to_vec();
            if chunk.len() < packet_len {
                chunk.resize(packet_len, 0xff);
            }
            log::trace!("write data ep {} bytes", chunk.len());
            self.write_bulk(
                DATA_ENDPOINT_OUT,
                &chunk,
                Duration::from_millis(USB_TIMEOUT_MS),
            )?;
        }
        log::trace!("write data ep total {} bytes", buf.len());
        Ok(())
    }
}
