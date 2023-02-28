use thiserror::Error;

/// Alias for a `Result` with the error type `wlink::Error`.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error: {0}")]
    Custom(String),
    #[error("USB error: {0}")]
    Rusb(#[from] rusb::Error),
    #[error("Unknown WCH-Link variant: {0}")]
    UnknownLinkVariant(u8),
    #[error("Unknown RISC-V Chip: 0x{0:02x}")]
    UnknownChip(u8),
    #[error("WCH-Link underlying protocol error: {0:02x} {1:02x?}")]
    Protocol(u8, Vec<u8>),
    #[error("Invalid payload length")]
    InvalidPayloadLength,
    #[error("Invalid payload")]
    InvalidPayload,
    #[error("DM Abstract comand error: {0:?}")]
    AbstractCommandError(AbstractcsCmdErr),
}

#[derive(Debug, Clone, Copy)]
pub enum AbstractcsCmdErr {
    /// Write to the command, abstractcs and abstractauto registers, or read/write to the data
    /// and progbuf registers when the abstract command is executed.
    Busy = 1,
    /// The current abstract command is not supported
    NotSupported = 2,
    /// error occurs when the abstract command is executed.
    Exception = 3,
    /// the hart wasn’t in the required state (running/halted), or unavailable
    HaltOrResume = 4,
    /// bus error (e.g. alignment, access size, or timeout)
    Bus = 5,
    /// Parity bit error during communication (WCH's extension)
    Parity = 6,
    /// The command failed for another reason.
    Other = 7,
}

impl AbstractcsCmdErr {
    pub(crate) fn try_from_cmderr(value: u8) -> Result<()> {
        match value {
            0 => Ok(()),
            1 => Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)),
            2 => Err(Error::AbstractCommandError(AbstractcsCmdErr::NotSupported)),
            3 => Err(Error::AbstractCommandError(AbstractcsCmdErr::Exception)),
            4 => Err(Error::AbstractCommandError(AbstractcsCmdErr::HaltOrResume)),
            5 => Err(Error::AbstractCommandError(AbstractcsCmdErr::Bus)),
            6 => Err(Error::AbstractCommandError(AbstractcsCmdErr::Parity)),
            7 => Err(Error::AbstractCommandError(AbstractcsCmdErr::Other)),

            _ => unreachable!(),
        }
    }
}
