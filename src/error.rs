//! Wlink error type and associated utilities
use thiserror::Error;

/// Possible error types when using wlink
#[derive(Error, Debug)]
pub enum Error {
    #[error("WCH-Link protocol error: {0:02x} {1:02x?}")]
    Protocol(u8, Vec<u8>),
    #[error("Invalid payload length")]
    InvalidPayloadLength,
    #[error("Invalid payload")]
    InvalidPayload,
    #[error("Unknown link type")]
    UnknownLinkType(u8),
    #[error("Unknown RISC-V chip type")]
    UnknownRiscvChipType(u8),
    #[error("Cannot reset debug module")]
    DebugModuleResetFailed,
    #[error("Failed to execute fast program procedure")]
    FastProgram,
    #[error("Timeout when writing flash")]
    WriteFlashTimeout,
    #[error("`rusb` library error: {0}")]
    Rusb(#[from] rusb::Error),
    #[error("No such USB device")]
    UsbNoSuchDevice,
    #[error("No such endpoints in USB device")]
    UsbNoSuchEndpoints,
}
