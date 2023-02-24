use std::fmt;

use self::error::{Error, Result};

pub mod commands;
pub mod device;
pub mod error;
pub mod transport;

/// All WCH-Link probe variants, see-also: http://www.wch-ic.com/products/WCH-Link.html
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum WchLinkVariant {
    /// WCH-Link-CH549, does not support CH32V00X
    Ch549 = 1,
    /// WCH-LinkE-CH32V305
    ECh32v305 = 2,
    /// WCH-LinkS-CH32V203
    SCh32v203 = 3,
    /// WCH-LinkB,
    B = 4,
}

impl WchLinkVariant {
    pub fn try_from_u8(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Ch549),
            2 => Ok(Self::ECh32v305),
            3 => Ok(Self::SCh32v203),
            4 => Ok(Self::B),
            _ => Err(Error::Custom(format!("Unknown WCH-Link variant {}", value))),
        }
    }
}

impl fmt::Display for WchLinkVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WchLinkVariant::Ch549 => write!(f, "WCH-Link-CH549"),
            WchLinkVariant::ECh32v305 => write!(f, "WCH-LinkE-CH32V305"),
            WchLinkVariant::SCh32v203 => write!(f, "WCH-LinkS-CH32V203"),
            WchLinkVariant::B => write!(f, "WCH-LinkB"),
        }
    }
}

/// Currently supported RISC-V chip series/family
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum RiscvChip {
    /// CH32V103 RISC-V3A series
    CH32V103 = 0x01,
    /// CH571/CH573 RISC-V3A BLE 4.2 series
    CH57x = 0x02,
    /// CH565/CH569 RISC-V3A series
    CH56x = 0x03,
    /// CH32V20x RISC-V4B/V4C series
    CH32V20x = 0x05,
    /// CH32V30x RISC-V4C/V4F series
    CH32V30x = 0x06,
    /// CH581/CH582/CH583 RISC-V4A BLE 5.3 series
    CH58x = 0x07,
    /// CH32V003 RISC-V2A series
    CH32V003 = 0x09,
}

impl RiscvChip {
    fn try_from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(RiscvChip::CH32V103),
            0x02 => Ok(RiscvChip::CH57x),
            0x03 => Ok(RiscvChip::CH56x),
            0x05 => Ok(RiscvChip::CH32V20x),
            0x06 => Ok(RiscvChip::CH32V30x),
            0x07 => Ok(RiscvChip::CH58x),
            0x09 => Ok(RiscvChip::CH32V003),
            _ => Err(Error::Custom(format!(
                "Unknown riscvchip type 0x{:02x}",
                value
            ))),
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
            RiscvChip::CH58x => 0x0000_0000,
            RiscvChip::CH56x => 0x0000_0000,
            RiscvChip::CH57x => 0x0000_0000,
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
