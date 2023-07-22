use std::{fmt, str::FromStr};

pub mod commands;
pub mod device;
pub mod error;
pub mod flash_op;
pub mod format;
mod operations;
pub mod regs;
pub mod transport;

pub use crate::error::{Error, Result};

/// All WCH-Link probe variants, see-also: <http://www.wch-ic.com/products/WCH-Link.html>
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum WchLinkVariant {
    /// WCH-Link-CH549, does not support CH32V00X
    Ch549 = 1,
    /// WCH-LinkE-CH32V305
    ECh32v305 = 2,
    /// WCH-LinkS-CH32V203
    SCh32v203 = 3,
    /// WCH-LinkW-CH32V208
    WCh32v208 = 5,
}

impl WchLinkVariant {
    pub fn try_from_u8(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Ch549),
            2 => Ok(Self::ECh32v305),
            3 => Ok(Self::SCh32v203),
            5 => Ok(Self::WCh32v208),
            0x12 => Ok(Self::ECh32v305), // ??
            _ => Err(Error::UnknownLinkVariant(value)),
        }
    }

    pub fn can_switch_mode(&self) -> bool {
        !matches!(self, WchLinkVariant::Ch549)
    }
}

impl fmt::Display for WchLinkVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WchLinkVariant::Ch549 => write!(f, "WCH-Link-CH549"),
            WchLinkVariant::ECh32v305 => write!(f, "WCH-LinkE-CH32V305"),
            WchLinkVariant::SCh32v203 => write!(f, "WCH-LinkS-CH32V203"),
            WchLinkVariant::WCh32v208 => write!(f, "WCH-LinkW-CH32V208"),
        }
    }
}

/// Currently supported RISC-V chip series/family
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RiscvChip {
    /// CH32V103 RISC-V3A series
    CH32V103 = 0x01,
    /// CH571/CH573 RISC-V3A BLE 4.2 series
    CH57X = 0x02,
    /// CH565/CH569 RISC-V3A series
    CH56X = 0x03,
    /// CH32V20X RISC-V4B/V4C series
    CH32V20X = 0x05,
    /// CH32V30X RISC-V4C/V4F series, The same as type 5
    CH32V30X = 0x06,
    /// CH58x RISC-V4A BLE 5.3 series, fallback as CH57X
    CH58X = 0x07,
    /// CH32V003 RISC-V2A series
    CH32V003 = 0x09,
    _Unkown0A = 0x0A, // 10
    /// CH59x RISC-V4C BLE 5.4 series, fallback as CH57X
    CH59X = 0x0B, // 11
    _Unkown0C = 0x0C, // 12
    /// CH32X035 RISC-V4C series, fallbak as 0x0C
    CH32X035 = 0x0D, // 13
}

impl RiscvChip {
    /// Support flash protect commands, and info query commands
    pub(crate) fn support_flash_protect(&self) -> bool {
        // type 3, 2, 7, 0x0A, 0x0b do not support write protect
        matches!(
            self,
            RiscvChip::CH32V103
                | RiscvChip::CH32V003
                | RiscvChip::CH32V20X
                | RiscvChip::CH32V30X
                | RiscvChip::CH32X035
        )
    }

    // CH32V208xB, CH32V307, CH32V303RCT6/VCT6
    pub(crate) fn support_ram_rom_mode(&self) -> bool {
        matches!(self, RiscvChip::CH32V20X | RiscvChip::CH32V30X)
    }

    /// Very unsafe. This disables the debug interface of the chip.
    /// Command sequence is 810e0101
    pub fn can_disable_debug(&self) -> bool {
        matches!(
            self,
            RiscvChip::CH57X | RiscvChip::CH56X | RiscvChip::CH58X | RiscvChip::CH59X
        )
    }

    pub fn reset_command(&self) -> crate::commands::Reset {
        match self {
            RiscvChip::CH57X | RiscvChip::CH58X | RiscvChip::CH59X => {
                crate::commands::Reset::Normal2
            }
            _ => crate::commands::Reset::Normal,
        }
    }

    fn flash_op(&self) -> &[u8] {
        match self {
            RiscvChip::CH32V103 => &flash_op::CH32V103,
            RiscvChip::CH32V003 => &flash_op::CH32V003,
            RiscvChip::CH57X | RiscvChip::CH58X | RiscvChip::CH59X => &flash_op::CH573,
            RiscvChip::CH56X => &flash_op::CH569,
            RiscvChip::CH32V20X | RiscvChip::CH32V30X => &flash_op::CH32V307,
            RiscvChip::_Unkown0A => &flash_op::UNKNOWN_10,
            RiscvChip::_Unkown0C => &flash_op::UNKNOWN_12,
            RiscvChip::CH32X035 => &flash_op::UNKNOWN_12,
        }
    }
    fn try_from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(RiscvChip::CH32V103),
            0x02 => Ok(RiscvChip::CH57X),
            0x03 => Ok(RiscvChip::CH56X),
            0x05 => Ok(RiscvChip::CH32V20X),
            0x06 => Ok(RiscvChip::CH32V30X),
            0x07 => Ok(RiscvChip::CH58X),
            0x09 => Ok(RiscvChip::CH32V003),
            0x0B => Ok(RiscvChip::CH59X),
            0x0D => Ok(RiscvChip::CH32X035),
            _ => Err(Error::UnknownChip(value)),
        }
    }

    pub fn page_size(&self) -> u32 {
        match self {
            RiscvChip::CH32V103 => 128,
            RiscvChip::CH32V003 => 64,
            _ => 256,
        }
    }

    pub fn code_flash_start(&self) -> u32 {
        match self {
            RiscvChip::CH56X | RiscvChip::CH57X | RiscvChip::CH58X | RiscvChip::CH59X => {
                0x0000_0000
            }
            _ => 0x0800_0000,
        }
    }

    pub fn write_pack_size(&self) -> u32 {
        match self {
            RiscvChip::CH32V003 => 1024,
            _ => 4096,
        }
    }
}

// for clap parser
impl FromStr for RiscvChip {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match &*s.to_ascii_uppercase() {
            "CH32V103" => Ok(RiscvChip::CH32V103),
            "CH32V20X" => Ok(RiscvChip::CH32V20X),
            "CH32V30X" => Ok(RiscvChip::CH32V30X),
            "CH32V003" => Ok(RiscvChip::CH32V003),
            "CH56X" => Ok(RiscvChip::CH56X),
            "CH57X" => Ok(RiscvChip::CH57X),
            "CH58X" => Ok(RiscvChip::CH58X),
            "CH59X" => Ok(RiscvChip::CH59X),
            "CH32X035" => Ok(RiscvChip::CH32X035),
            _ => Err(Error::UnknownChip(0)),
        }
    }
}
