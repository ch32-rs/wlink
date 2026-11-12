use thiserror::Error;

use crate::{
    RiscvChip,
    probe::{ProbeChoice, ProbeMode},
};

/// Alias for a `Result` with the error type `wlink::Error`.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Custom(String),
    #[error("USB error: {0}")]
    Usb(nusb::Error),
    #[error("WCH-Link not found, please check your connection")]
    ProbeNotFound,
    /// Several probes are attached, but the operation did not name one.
    #[error("{}", ambiguous_probe_message(.0))]
    AmbiguousProbe(Vec<ProbeChoice>),
    #[error("WCH-Link is connected, but is not in RV mode")]
    ProbeModeNotSupported,
    #[error("WCH-Link doesn't support current chip: {0:?}")]
    UnsupportedChip(RiscvChip),
    #[error("Unknown WCH-Link variant: {0}")]
    UnknownLinkVariant(u8),
    #[error("Unknown RISC-V Chip: 0x{0:02x}")]
    UnknownChip(u8),
    #[error(
        "Probe is not attached to an MCU, or debug is not enabled. (hint: use wchisp to enable debug)"
    )]
    NotAttached,
    #[error("Chip mismatch: expected {0:?}, got {1:?}")]
    ChipMismatch(RiscvChip, RiscvChip),
    #[error("WCH-Link underlying protocol error: {0:#04x} {1:#04x?}")]
    Protocol(u8, Vec<u8>),
    #[error("Invalid payload length")]
    InvalidPayloadLength,
    #[error("Invalid payload")]
    InvalidPayload,
    #[error("DM Abstract comand error: {0:?}")]
    AbstractCommandError(AbstractcsCmdErr),
    #[error("DM is busy")]
    Busy,
    #[error("DMI Status Failed")]
    DmiFailed,
    #[error("Operation timeout")]
    Timeout,
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Driver error")]
    Driver,
    #[cfg(windows)]
    #[error("Windows driver error: {0}")]
    Windows(#[from] windows::core::Error),
}

/// Builds the "pick one of these probes" error message. Only RV-mode probes
/// are offered as operation targets; `WchLink::open_nth` rejects DAP-mode
/// probes, so those come with a `mode-switch --rv` hint instead.
fn ambiguous_probe_message(choices: &[ProbeChoice]) -> String {
    use std::fmt::Write as _;

    let rv: Vec<_> = choices.iter().filter(|c| c.mode == ProbeMode::Rv).collect();
    let dap: Vec<_> = choices
        .iter()
        .filter(|c| c.mode == ProbeMode::Dap)
        .collect();

    let mut msg = format!(
        "{} WCH-Link probes attached; refusing to continue without an explicit probe.",
        choices.len()
    );
    if rv.is_empty() {
        let _ = write!(
            msg,
            "\nNone of them is in RV mode, so no probe can be used yet."
        );
    } else {
        let _ = write!(
            msg,
            "\nSelect one of the following with `-d` (a serial stays stable across replugs when available; `wlink list` shows details):"
        );
        for choice in rv {
            // Formatting into a String is infallible.
            let _ = write!(
                msg,
                "\n  -d {:<26} # {}",
                choice.selector, choice.description
            );
        }
    }
    if !dap.is_empty() {
        let _ = write!(
            msg,
            "\nAlso attached in DAP mode (cannot be used; switch back to RV mode first):"
        );
        for choice in dap {
            let _ = write!(
                msg,
                "\n  wlink -d {} mode-switch --rv   # {}",
                choice.selector, choice.description
            );
        }
    }
    msg
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
