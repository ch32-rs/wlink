//! The wlink library.

pub mod chips;
pub mod commands;
pub mod dmi;
pub mod error;
pub mod firmware;
pub mod flash_op;
pub mod operations;
pub mod probe;
pub mod regs;
pub mod usb_device;

use clap::{builder::PossibleValue, ValueEnum};
use probe::WchLink;

pub use crate::error::{Error, Result};

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
    /// CH583/CH582/CH581 RISC-V4A BLE 5.3 series
    CH582 = 0x07,
    /// CH32V003 RISC-V2A series
    CH32V003 = 0x09,
    /// RISC-V EC controller, undocumented.
    /// The only reference I can find is <https://www.wch.cn/news/606.html>.
    CH8571 = 0x0A, // 10,
    /// CH59x RISC-V4C BLE 5.4 series, fallback as CH58X
    CH59X = 0x0B, // 11
    /// CH643 RISC-V4C series, RGB Display Driver MCU
    CH643 = 0x0C, // 12
    /// CH32X035 RISC-V4C USB-PD series, fallback as CH643
    CH32X035 = 0x0D, // 13
    /// CH32L103 RISC-V4C low power series, USB-PD
    CH32L103 = 0x0E, // 14
    /// CH641 RISC-V2A series, USB-PD, fallback as CH32V003
    CH641 = 0x49,
    /// CH585/CH584 RISC-V3C series, BLE 5.4, NFC, USB HS, fallback as CH582
    CH585 = 0x4B,

    // The following are possible riscvchip code of:
    // - CH32M007
    // - CH32V002/4/5/6/7
    // - CH645
    // - CH653
    // - CH654
    // - CH32V317?
    Unknown0x0F = 0x0F,
    // 1024, 256
    Unknown0x4E = 0x4E,
    // V002/4/5/6/7, or M007
    Unknown0x46 = 0x46,
    // V002/4/5/6/7, or M007
    Unknown0x86 = 0x86,
}

impl ValueEnum for RiscvChip {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            RiscvChip::CH32V103,
            RiscvChip::CH57X,
            RiscvChip::CH56X,
            RiscvChip::CH32V20X,
            RiscvChip::CH32V30X,
            RiscvChip::CH582,
            RiscvChip::CH585,
            RiscvChip::CH32V003,
            RiscvChip::CH8571,
            RiscvChip::CH59X,
            RiscvChip::CH643,
            RiscvChip::CH32X035,
            RiscvChip::CH32L103,
            RiscvChip::CH641,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            RiscvChip::CH32V103 => Some(PossibleValue::new("CH32V103")),
            RiscvChip::CH57X => Some(PossibleValue::new("CH57X")),
            RiscvChip::CH56X => Some(PossibleValue::new("CH56X")),
            RiscvChip::CH32V20X => Some(PossibleValue::new("CH32V20X")),
            RiscvChip::CH32V30X => Some(PossibleValue::new("CH32V30X")),
            RiscvChip::CH582 => Some(PossibleValue::new("CH582")),
            RiscvChip::CH585 => Some(PossibleValue::new("CH585")),
            RiscvChip::CH32V003 => Some(PossibleValue::new("CH32V003")),
            RiscvChip::CH8571 => Some(PossibleValue::new("CH8571")),
            RiscvChip::CH59X => Some(PossibleValue::new("CH59X")),
            RiscvChip::CH643 => Some(PossibleValue::new("CH643")),
            RiscvChip::CH32X035 => Some(PossibleValue::new("CH32X035")),
            RiscvChip::CH32L103 => Some(PossibleValue::new("CH32L103")),
            RiscvChip::CH641 => Some(PossibleValue::new("CH641")),
            RiscvChip::Unknown0x0F => todo!(),
            RiscvChip::Unknown0x4E => todo!(),
            RiscvChip::Unknown0x46 => todo!(),
            RiscvChip::Unknown0x86 => todo!(),
        }
    }

    fn from_str(input: &str, ignore_case: bool) -> std::result::Result<Self, String> {
        let s = if ignore_case {
            input.to_ascii_uppercase()
        } else {
            input.to_string()
        };
        match &*s {
            "CH32V103" => Ok(RiscvChip::CH32V103),
            "CH32V20X" | "CH32V203" | "CH32V208" => Ok(RiscvChip::CH32V20X),
            "CH32V30X" | "CH32V303" | "CH32V305" | "CH32V307" => Ok(RiscvChip::CH32V30X),
            "CH32V003" => Ok(RiscvChip::CH32V003),
            "CH56X" | "CH565" | "CH569" => Ok(RiscvChip::CH56X),
            "CH57X" | "CH571" | "CH573" => Ok(RiscvChip::CH57X),
            "CH581" | "CH582" | "CH583" => Ok(RiscvChip::CH582),
            "CH584" | "CH585" => Ok(RiscvChip::CH585),
            "CH59X" | "CH591" | "CH592" => Ok(RiscvChip::CH59X),
            "CH32X0" | "CH32X03X" | "CH32X033" | "CH32X034" | "CH32X035" => Ok(RiscvChip::CH32X035),
            "CH643" => Ok(RiscvChip::CH643),
            "CH32L103" => Ok(RiscvChip::CH32L103),
            "CH8571" => Ok(RiscvChip::CH8571),
            "CH641" => Ok(RiscvChip::CH641),
            "CH58X" => Err("Ambiguous chip family, use either CH582 or CH585".to_string()),
            _ => Err(format!("Unknown chip: {}", s)),
        }
    }
}

impl RiscvChip {
    /// Support flash protect commands, and info query commands
    pub fn support_flash_protect(&self) -> bool {
        matches!(
            self,
            RiscvChip::CH32V103
                | RiscvChip::CH32V20X
                | RiscvChip::CH32V30X
                | RiscvChip::CH32V003
                | RiscvChip::CH643
                | RiscvChip::CH32L103
                | RiscvChip::CH32X035
                | RiscvChip::CH641
        )
    }

    // CH32V208xB, CH32V307, CH32V303RCT6/VCT6
    pub(crate) fn support_ram_rom_mode(&self) -> bool {
        matches!(self, RiscvChip::CH32V20X | RiscvChip::CH32V30X)
    }

    /// Support config registers, query info(UID, etc.)
    pub fn support_query_info(&self) -> bool {
        !matches!(
            self,
            RiscvChip::CH57X
                | RiscvChip::CH56X
                | RiscvChip::CH582
                | RiscvChip::CH585
                | RiscvChip::CH59X
        )
    }

    /// Very unsafe.
    /// This disables the debug interface of the chip.
    /// Command sequence is 810e0101
    pub fn support_disable_debug(&self) -> bool {
        matches!(
            self,
            RiscvChip::CH57X
                | RiscvChip::CH56X
                | RiscvChip::CH582
                | RiscvChip::CH585
                | RiscvChip::CH59X
        )
    }

    /// Erase code flash by RST pin or power-off
    pub fn support_special_erase(&self) -> bool {
        !matches!(
            self,
            RiscvChip::CH57X
                | RiscvChip::CH56X
                | RiscvChip::CH582
                | RiscvChip::CH585
                | RiscvChip::CH59X
        )
    }

    pub fn support_sdi_print(&self) -> bool {
        // CH641, CH643, CH32V003, CH32V103, CH32V20x, CH32V30x, CH32X035, CH32L103
        matches!(
            self,
            RiscvChip::CH32V003
                | RiscvChip::CH32V103
                | RiscvChip::CH32V20X
                | RiscvChip::CH32V30X
                | RiscvChip::CH32X035
                | RiscvChip::CH32L103
                | RiscvChip::CH643
                | RiscvChip::CH641
        )
    }

    pub fn is_rv32ec(&self) -> bool {
        matches!(self, RiscvChip::CH32V003 | RiscvChip::CH641)
    }

    pub fn reset_command(&self) -> crate::commands::Reset {
        match self {
            RiscvChip::CH57X | RiscvChip::CH582 | RiscvChip::CH59X => crate::commands::Reset::Chip,
            _ => crate::commands::Reset::Normal,
        }
    }

    /// Device-specific post init logic
    pub fn do_post_init(&self, probe: &mut WchLink) -> Result<()> {
        match self {
            RiscvChip::CH32V103 => {
                // 81 0d 01 03
                // 81 0d 01 10
                let _ = probe.send_command(commands::RawCommand::<0x0d>(vec![0x03]))?;
                // let _ = probe.send_command(commands::RawCommand::<0x0d>(vec![0x10]))?;
            }
            RiscvChip::CH32V30X | RiscvChip::CH8571 | RiscvChip::CH32V003 => {
                // 81 0d 01 03
                // let _ = probe.send_command(commands::RawCommand::<0x0d>(vec![0x03]))?;
            }
            RiscvChip::CH57X | RiscvChip::CH582 => {
                log::warn!("The debug interface has been opened, there is a risk of code leakage.");
                log::warn!("Please ensure that the debug interface has been closed before leaving factory!");
            }
            RiscvChip::CH56X => {
                log::warn!("The debug interface has been opened, there is a risk of code leakage.");
                log::warn!("Please ensure that the debug interface has been closed before leaving factory!");
                // 81 0d 01 04
                // should test return value
                let resp = probe.send_command(commands::RawCommand::<0x0d>(vec![0x04]))?;
                log::debug!("TODO, handle CH56X resp {:?}", resp);
            }
            _ => (),
        }
        Ok(())
    }

    // TODO: CH32V003 has two flash_op for different flash start address
    fn get_flash_op(&self) -> &'static [u8] {
        match self {
            RiscvChip::CH32V003 | RiscvChip::CH641 => &flash_op::CH32V003,
            RiscvChip::CH32V103 => &flash_op::CH32V103,
            RiscvChip::CH32V20X | RiscvChip::CH32V30X => &flash_op::CH32V307,
            RiscvChip::CH56X => &flash_op::CH569,
            RiscvChip::CH57X => &flash_op::CH573,
            RiscvChip::CH582 | RiscvChip::CH59X | RiscvChip::CH585 => &flash_op::CH583,
            RiscvChip::CH8571 => &flash_op::OP8571,
            RiscvChip::CH32X035 | RiscvChip::CH643 => &flash_op::CH643,
            RiscvChip::CH32L103 => &flash_op::CH32L103,
            RiscvChip::Unknown0x0F => todo!(),
            RiscvChip::Unknown0x4E => todo!(),
            RiscvChip::Unknown0x46 => todo!(),
            RiscvChip::Unknown0x86 => todo!(),
        }
    }
    fn try_from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(RiscvChip::CH32V103),
            0x02 => Ok(RiscvChip::CH57X),
            0x03 => Ok(RiscvChip::CH56X),
            0x05 => Ok(RiscvChip::CH32V20X),
            0x06 => Ok(RiscvChip::CH32V30X),
            0x07 => Ok(RiscvChip::CH582),
            0x09 => Ok(RiscvChip::CH32V003),
            0x0A => Ok(RiscvChip::CH8571),
            0x0B => Ok(RiscvChip::CH59X),
            0x0C => Ok(RiscvChip::CH643),
            0x0D => Ok(RiscvChip::CH32X035),
            0x0E => Ok(RiscvChip::CH32L103),
            0x49 => Ok(RiscvChip::CH641),
            0x4B => Ok(RiscvChip::CH585),
            _ => Err(Error::UnknownChip(value)),
        }
    }

    /// Packet data length of data endpoint
    pub fn data_packet_size(&self) -> usize {
        match self {
            RiscvChip::CH32V103 => 128,
            RiscvChip::CH32V003 | RiscvChip::CH641 => 64,
            _ => 256,
        }
    }

    pub fn code_flash_start(&self) -> u32 {
        match self {
            RiscvChip::CH56X
            | RiscvChip::CH57X
            | RiscvChip::CH582
            | RiscvChip::CH585
            | RiscvChip::CH59X
            | RiscvChip::CH8571 => 0x0000_0000,
            _ => 0x0800_0000,
        }
    }

    // The same as wch-openocd-riscv
    pub fn fix_code_flash_start(&self, start_address: u32) -> u32 {
        let addr = self.code_flash_start() + start_address;
        if addr >= 0x10000000 {
            addr - 0x08000000
        } else {
            addr
        }
    }

    /// pack size for fastprogram
    pub fn write_pack_size(&self) -> u32 {
        match self {
            RiscvChip::CH32V003 | RiscvChip::CH641 => 1024,
            RiscvChip::Unknown0x4E => 1024,
            _ => 4096,
        }
    }
}
