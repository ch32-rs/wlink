//! Probe control commands.
//  COMMAND_ID = 0x0d

use crate::{probe::WchLinkVariant, RiscvChip};

use super::*;

/// GetDeviceVersion (0x0d, 0x01)
#[derive(Debug)]
pub struct GetProbeInfo;
impl Command for GetProbeInfo {
    type Response = ProbeInfo;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x01]
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProbeInfo {
    pub major_version: u8,
    pub minor_version: u8,
    pub variant: WchLinkVariant,
}
impl ProbeInfo {
    pub fn version(&self) -> (u8, u8) {
        (self.major_version, self.minor_version)
    }
}
impl Response for ProbeInfo {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        Ok(Self {
            major_version: bytes[0],
            minor_version: bytes[1],
            // Only avaliable in newer version of firmware
            variant: if bytes.len() == 4 {
                WchLinkVariant::try_from_u8(bytes[2])?
            } else {
                WchLinkVariant::Ch549
            },
        })
    }
}
impl fmt::Display for ProbeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WCH-Link v{}.{}(v{}) ({})",
            self.major_version,
            self.minor_version,
            self.major_version * 10 + self.minor_version,
            self.variant
        )
    }
}

/// ?SetChipType (0x0d, 0x02)
#[derive(Debug)]
pub struct AttachChip;
impl Command for AttachChip {
    type Response = AttachChipResponse;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x02]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachChipResponse {
    pub chip_family: RiscvChip,
    pub riscvchip: u8,
    pub chip_id: u32,
}
impl Response for AttachChipResponse {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 5 {
            return Err(Error::InvalidPayloadLength);
        }
        Ok(Self {
            chip_family: RiscvChip::try_from_u8(bytes[0])?,
            riscvchip: bytes[0],
            chip_id: u32::from_be_bytes(bytes[1..5].try_into().unwrap()),
        })
    }
}
// For logging
impl fmt::Display for AttachChipResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.chip_id == 0 {
            write!(f, "{:?}", self.chip_family)
        } else if let Some(chip_name) = crate::chips::chip_id_to_chip_name(self.chip_id) {
            write!(
                f,
                "{:?} [{}] (ChipID: 0x{:08x})",
                self.chip_family, chip_name, self.chip_id
            )
        } else {
            write!(f, "{:?} (ChipID: 0x{:08x})", self.chip_family, self.chip_id)
        }
    }
}

/// Erase code flash, only supported by WCH-LinkE.
#[derive(Debug)]
pub enum EraseCodeFlash {
    ByPinRST(RiscvChip),
    ByPowerOff(RiscvChip),
}
impl Command for EraseCodeFlash {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;

    fn payload(&self) -> Vec<u8> {
        match self {
            // This is more complex, require RST pin to be connected.
            EraseCodeFlash::ByPinRST(c) => vec![0x08, *c as u8],
            // NOTE: From the protocol, this command's bytes is wrongly seted
            // 81 0d 01 0f 09, note here, the "length" bytes is wrong.
            // I guess it is not verified. So here we use `02`.
            EraseCodeFlash::ByPowerOff(c) => vec![0x0f, *c as u8],
        }
    }
}

/// GetROMRAM, Only avaliable for CH32V2, CH32V3, CH56X
/// 0, 1, 2, 3
#[derive(Debug)]
pub struct GetChipRomRamSplit;
impl Command for GetChipRomRamSplit {
    type Response = u8;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x04]
    }
}

/// 0, 1, 2, 3
#[derive(Debug)]
pub struct SetChipRomRamSplit(u8);
impl Command for SetChipRomRamSplit {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x05, self.0]
    }
}

// ?? close out
/// Detach Chip, (0x0d, 0xff)
#[derive(Debug)]
pub struct OptEnd;
impl Command for OptEnd {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0xff]
    }
}

/// Set Power, from pow3v3, pow5v fn
#[derive(clap::Subcommand, PartialEq, Clone, Copy, Debug)]
pub enum SetPower {
    /// Enable 3.3V output
    Enable3v3,
    /// Disable 3.3V output
    Disable3v3,
    /// Enable 5V output
    Enable5v,
    /// Disable 5V output
    Disable5v,
}
impl Command for SetPower {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        match self {
            SetPower::Enable3v3 => vec![0x09],
            SetPower::Disable3v3 => vec![0x0A],
            SetPower::Enable5v => vec![0x0B],
            SetPower::Disable5v => vec![0x0C],
        }
    }
}

/// SDI print support, only available for WCH-LinkE
/// Firmware version >= 2.10
#[derive(Debug)]
pub struct SetSdiPrintEnabled(pub bool);

impl Command for SetSdiPrintEnabled {
    // 0x00 success, 0xff not support
    type Response = u8;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        if self.0 {
            vec![0xee, 0x00]
        } else {
            vec![0xee, 0x01]
        }
    }
}

/// Set RST pin
#[derive(Debug)]
pub enum SetRSTPin {
    Low,
    High,
    Floating,
}
impl Command for SetRSTPin {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        let subcmd = match *self {
            SetRSTPin::Low => 0x13,
            SetRSTPin::High => 0x14,
            SetRSTPin::Floating => 0x15,
        };
        vec![0x0e, subcmd]
    }
}
