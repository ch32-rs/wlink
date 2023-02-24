//! Probe control commands.
//  COMMAND_ID = 0x0d

use std::fmt;

use crate::{RiscvChip, WchLinkVariant};

use super::*;

pub struct GetProbeInfo;
impl Command for GetProbeInfo {
    type Response = ProbeInfo;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x01]
    }
}
#[derive(Debug)]
pub struct ProbeInfo {
    pub major_version: u8,
    pub minor_version: u8,
    pub variant: WchLinkVariant,
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

pub struct AttachChip;
impl Command for AttachChip {
    type Response = ChipInfo;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x02]
    }
}
pub struct ChipInfo {
    pub chip_family: RiscvChip,
    riscvchip: u8,
    pub chip_type: u32,
}
impl ChipInfo {}
impl Response for ChipInfo {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 5 {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        Ok(Self {
            chip_family: RiscvChip::try_from_u8(bytes[0])?,
            riscvchip: bytes[0],
            chip_type: u32::from_be_bytes(bytes[1..5].try_into().unwrap()),
        })
    }
}
impl fmt::Debug for ChipInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChipInfo")
            .field("chip_family", &self.chip_family)
            .field("chip_type", &format!("{:#010x}", self.chip_type))
            .finish()
    }
}

// ?? close out
pub struct DetachChip;
impl Command for DetachChip {
    type Response = ();
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0xff]
    }
}

/// Only avaliable for CH32V2, CH32V3, CH56x
/// 0, 1, 2, 3
pub struct GetChipRomRamSplit;
impl Command for GetChipRomRamSplit {
    type Response = u8;
    const COMMAND_ID: u8 = 0x0d;
    fn payload(&self) -> Vec<u8> {
        vec![0x04]
    }
}
