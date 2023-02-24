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
    major_version: u8,
    minor_version: u8,
    link_variant: WchLinkVariant,
}
impl Response for ProbeInfo {
    fn from_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(crate::error::Error::InvalidPayloadLength);
        }
        Ok(Self {
            major_version: bytes[0],
            minor_version: bytes[1],
            link_variant: if bytes.len() == 4 {
                WchLinkVariant::from(bytes[2])
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
