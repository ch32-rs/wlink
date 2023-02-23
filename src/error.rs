use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error: {0}")]
    Custom(String),
    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),
    #[error("WCH-Link protocol error: {0:02x} {1:02x?}")]
    Protocol(u8, Vec<u8>),
    #[error("Invalid payload length")]
    InvalidPayloadLength,
    #[error("Invalid payload")]
    InvalidPayload,
}
