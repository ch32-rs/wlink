//! Predefined operations for WCH-Link

use std::{thread::sleep, time::Duration};

use crate::{
    commands::{self, control::ProbeInfo, DmiOp, Program, ReadMemory, SetRamAddress},
    device::{ChipInfo, WchLink},
    error::AbstractcsCmdErr,
    regs::{self, Abstractcs, DMReg, Dmcontrol, Dmstatus},
    transport::Transport,
    Error, Result, RiscvChip,
};

impl WchLink {
    pub fn probe_info(&mut self) -> Result<ProbeInfo> {
        let info = self.send_command(commands::control::GetProbeInfo)?;
        log::info!("{}", info);
        Ok(info)
    }
    /// Attach chip and get chip info
    pub fn attach_chip(&mut self, expected_chip: Option<RiscvChip>) -> Result<()> {
        if self.chip.is_some() {
            log::warn!("Chip already attached");
        }

        let probe_info = self.send_command(commands::control::GetProbeInfo)?;

        let mut chip_info = None;
        for _ in 0..3 {
            // self.send_command(commands::control::DetachChip)?;

            self.send_command(commands::SetTwoLineMode {
                riscvchip: expected_chip.unwrap_or(RiscvChip::CH32V30X) as u8,
                speed: 1, // 1 high, 2, medium, 3 low
            })?;

            if let Ok(resp) = self.send_command(commands::control::AttachChip) {
                log::info!("Attached chip: {}", resp);
                chip_info = Some(resp);

                if let Some(expected_chip) = expected_chip {
                    if resp.chip_family != expected_chip {
                        log::error!(
                            "Attached chip type ({:?}) does not match expected chip type ({:?})",
                            resp.chip_family,
                            expected_chip
                        );
                        return Err(Error::ChipMismatch(expected_chip, resp.chip_family));
                    }
                }
                break;
            } else {
                log::debug!("retrying...");
                sleep(Duration::from_millis(100));
            }
        }
        let chip_info = chip_info.ok_or(Error::NotAttached)?;

        let mut uid = None;
        let mut sram_code_mode = 0;
        if chip_info.chip_family.support_flash_protect() {
            let chip_id = if probe_info.version() >= (2, 9) {
                self.send_command(commands::QueryChipInfo::V2)?
            } else {
                self.send_command(commands::QueryChipInfo::V1)?
            };
            log::info!("Chip UID: {chip_id}");
            uid = Some(chip_id);

            let flash_protected = self.send_command(commands::FlashProtect::Query)?;
            let protected = flash_protected == commands::FlashProtect::FLAG_PROTECTED;
            log::info!("Flash protected: {}", protected);
            if protected {
                log::warn!("Flash is protected, debug access is not available");
            }
            sram_code_mode = self.send_command(commands::control::GetChipRomRamSplit)?;
            log::debug!("SRAM CODE mode: {}", sram_code_mode);
        }

        // riscvchip = 7 => 2
        let flash_addr = chip_info.chip_family.code_flash_start();
        let page_size = chip_info.chip_family.page_size();

        let info = ChipInfo {
            uid,
            chip_family: chip_info.chip_family,
            chip_type: chip_info.chip_type,
            march: None,
            flash_size: 0, // TODO: read flash size
            memory_start_addr: flash_addr,
            sram_code_mode,
            page_size,
            //rom_kb: 0, // TODO:
            //ram_kb: 0, // TODO:
        };

        self.chip = Some(info);

        Ok(())
    }

    pub fn dump_info(&mut self) -> Result<()> {
        let misa = self.read_reg(regs::MISA)?;
        log::trace!("Read csr misa: {misa:08x}");
        let misa = parse_misa(misa);
        log::info!("RISC-V ISA: {misa:?}");

        // detect chip's RISC-V core version, QingKe cores
        let marchid = self.read_reg(regs::MARCHID)?;
        log::trace!("Read csr marchid: {marchid:08x}");
        let core_type = parse_marchid(marchid);
        log::info!("RISC-V arch: {core_type:?}");

        Ok(())
    }

    pub fn protect_flash(&mut self, protect: bool) -> Result<()> {
        // HACK: requires a fresh attach
        self.send_command(commands::control::DetachChip)?;

        let probe_info = self.send_command(commands::control::GetProbeInfo)?;

        self.send_command(commands::control::AttachChip)?;

        let flash_protected_flag = self.send_command(commands::FlashProtect::Query)?;
        let protected = flash_protected_flag == commands::FlashProtect::FLAG_PROTECTED;
        if protect == protected {
            log::info!(
                "Flash already {}",
                if protected {
                    "protected"
                } else {
                    "unprotected"
                }
            );
            return Ok(());
        }

        let use_v2 = probe_info.version() >= (2, 9);
        let cmd = match (protect, use_v2) {
            (true, true) => commands::FlashProtect::ProtectV2,
            (false, true) => commands::FlashProtect::UnprotectV2,
            (true, false) => commands::FlashProtect::Protect,
            (false, false) => commands::FlashProtect::Unprotect,
        };

        self.send_command(cmd)?;

        self.send_command(commands::Reset::Quit)?; // quit reset
        self.send_command(commands::control::DetachChip)?;

        self.send_command(commands::control::GetProbeInfo)?;
        self.send_command(commands::control::AttachChip)?;

        let flash_protected = self.send_command(commands::FlashProtect::Query)?;
        log::info!(
            "Flash protected: {}",
            flash_protected == commands::FlashProtect::FLAG_PROTECTED
        );

        Ok(())
    }

    // wlink_endprocess
    /// Detach chip and let it resume
    pub fn detach_chip(&mut self) -> Result<()> {
        log::debug!("Detach chip");
        if self.chip.is_some() {
            self.send_command(commands::control::DetachChip)?;
            self.chip = None;
        }
        Ok(())
    }

    pub fn read_flash_size_kb(&mut self) -> Result<u32> {
        // Ref: (DS) Chapter 31 Electronic Signature (ESIG)
        let raw_flash_cap = self.read_memory(0x1FFFF7E0, 4)?;
        println!("=> {raw_flash_cap:02x?}");
        let flash_size = u32::from_le_bytes(raw_flash_cap[0..4].try_into().unwrap());
        log::info!("Flash size {}KiB", flash_size);
        Ok(flash_size)
    }

    /// Read a continuous memory region, require MCU to be halted
    pub fn read_memory(&mut self, address: u32, length: u32) -> Result<Vec<u8>> {
        let mut length = length;
        if length % 4 != 0 {
            length = (length / 4 + 1) * 4;
        }
        self.send_command(ReadMemory {
            start_addr: address,
            len: length,
        })?;
        self.send_command(Program::BeginReadMemory)?;

        let mut mem = self.device_handle.read_data_endpoint(length as usize)?;
        // Fix endian
        for chunk in mem.chunks_exact_mut(4) {
            chunk.reverse();
        }

        println!(
            "{}",
            nu_pretty_hex::config_hex(
                &mem,
                nu_pretty_hex::HexConfig {
                    title: false,
                    ascii: true,
                    address_offset: address as _,
                    ..Default::default()
                },
            )
        );

        Ok(mem)
    }

    pub fn erase_flash(&mut self) -> Result<()> {
        self.send_command(Program::EraseFlash)?;
        Ok(())
    }

    pub fn write_flash(&mut self, data: &[u8], address: u32) -> Result<()> {
        let pack_size = self.chip.as_ref().unwrap().chip_family.write_pack_size();
        let code_start_addr = self.chip.as_ref().unwrap().memory_start_addr;
        log::debug!("Default flash address 0x{:08x}", code_start_addr);

        let mut data = data.to_vec();
        if data.len() % 256 != 0 {
            data.resize((data.len() / 256 + 1) * 256, 0);
        }
        self.send_command(SetRamAddress {
            start_addr: address,
            len: data.len() as u32,
        })?;
        self.send_command(Program::BeginWriteMemory)?;
        self.device_handle
            .write_data_endpoint(self.chip.as_ref().unwrap().chip_family.flash_op())?;

        log::debug!("Flash op written");

        for i in 0.. {
            // check written
            if let Ok(n) = self.send_command(Program::ExecMemory) {
                if n == 0x07 {
                    break;
                }
            }
            if i > 10 {
                return Err(Error::Custom("Timeout while write flash".into()));
            }
            sleep(Duration::from_millis(10));
        }
        // wlink_fastprogram
        self.send_command(Program::BeginWriteFlash)?;

        for chunk in data.chunks(pack_size as usize) {
            self.device_handle.write_data_endpoint(chunk)?;
            let rxbuf = self.device_handle.read_data_endpoint(4)?;
            if rxbuf[3] != 0x02 && rxbuf[3] != 0x04 {
                return Err(Error::Custom("Error while fastprogram".into()));
            }
        }
        log::debug!("Fastprogram done");

        Ok(())
    }

    pub fn ensure_mcu_halt(&mut self) -> Result<()> {
        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if dmstatus.allhalted() && dmstatus.anyhalted() {
            log::trace!("Already halted, nop");
        } else {
            loop {
                // Initiate a halt request
                self.send_command(DmiOp::write(0x10, 0x80000001))?;
                let dmstatus = self.dmi_read::<Dmstatus>()?;
                if dmstatus.anyhalted() && dmstatus.allhalted() {
                    break;
                } else {
                    log::warn!("Not halt, try send");
                    sleep(Duration::from_millis(10));
                }
            }
        }

        // Clear the halt request bit.
        self.send_command(DmiOp::write(0x10, 0x00000001))?;

        Ok(())
    }

    // SingleLineExitPauseMode
    pub fn ensure_mcu_resume(&mut self) -> Result<()> {
        self.clear_dmstatus_havereset()?;
        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if dmstatus.allrunning() && dmstatus.anyrunning() {
            log::debug!("Already running, nop");
            return Ok(());
        }

        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x10, 0x40000001))?;

        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if dmstatus.allresumeack() && dmstatus.anyresumeack() {
            log::debug!("Resumed");
            Ok(())
        } else {
            log::warn!("Resume fails");
            Ok(())
        }
    }

    /// Write a memory word, require MCU to be halted.
    ///
    /// V2 microprocessor debug module abstract command only supports the register access mode,
    /// So this function will use the register access mode to write a memory word,
    /// instead of using the memory access mode.
    pub fn write_memory_word(&mut self, address: u32, data: u32) -> Result<()> {
        self.ensure_mcu_halt()?;

        self.send_command(DmiOp::write(0x20, 0x0072a023))?; // sw x7,0(x5)
        self.send_command(DmiOp::write(0x21, 0x00100073))?; // ebreak
        self.send_command(DmiOp::write(0x04, address))?; // data0 <- address
        self.clear_abstractcs_cmderr()?;
        self.send_command(DmiOp::write(0x17, 0x00231005))?; // x5 <- data0

        let abstractcs = self.dmi_read::<Abstractcs>()?;
        log::trace!("{:?}", abstractcs);
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        self.send_command(DmiOp::write(0x04, data))?; // data0 <- data
        self.clear_abstractcs_cmderr()?;
        self.send_command(DmiOp::write(0x17, 0x00271007))?; // data0 <- x7
        let abstractcs = self.dmi_read::<Abstractcs>()?;
        log::trace!("{:?}", abstractcs);
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        Ok(())
    }

    fn clear_dmstatus_havereset(&mut self) -> Result<()> {
        let mut dmcontrol = self.dmi_read::<Dmcontrol>()?;
        dmcontrol.set_ackhavereset(true);
        self.dmi_write(dmcontrol)?;
        Ok(())
    }

    /// Clear cmderror field of abstractcs register.
    /// write 1 to clean the corresponding bit.
    fn clear_abstractcs_cmderr(&mut self) -> Result<()> {
        let mut abstractcs = self.dmi_read::<Abstractcs>()?;
        abstractcs.set_cmderr(0b111);
        self.dmi_write(abstractcs)?;
        Ok(())
    }

    /// Soft reset MCU, using PFIC.CFGR.SYSRST
    pub fn soft_reset(&mut self) -> Result<()> {
        const PFIC_CFGR: u32 = 0xE000E048;
        const KEY3: u32 = 0xBEEF;
        const KEY_OFFSET: u8 = 16;
        const RESETSYS_OFFSET: u8 = 7;

        const RESET_VAL: u32 = KEY3 << KEY_OFFSET | 1 << RESETSYS_OFFSET;

        self.write_memory_word(PFIC_CFGR, RESET_VAL)?;

        Ok(())
    }

    // SingleLineCoreReset
    pub fn reset_mcu_and_run(&mut self) -> Result<()> {
        self.ensure_mcu_halt()?;
        self.clear_dmstatus_havereset()?;

        // Clear the reset signal.
        self.send_command(DmiOp::write(0x10, 0x00000001))?; // clear haltreq

        self.send_command(DmiOp::write(0x10, 0x00000003))?; // initiate ndmreset
        let dmstatus = self.dmi_read::<Dmstatus>()?;
        println!("{:?}", dmstatus);
        if dmstatus.allhavereset() && dmstatus.anyhavereset() {
            // reseted
            log::debug!("Reseted");
        } else {
            log::warn!("Reset failed");
        }

        // Clear the reset status signal
        self.send_command(DmiOp::write(0x10, 0x10000001))?; // ackhavereset
        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if !dmstatus.allhavereset() && !dmstatus.anyhavereset() {
            log::debug!("Reset status cleared");
        } else {
            log::warn!("Reset status clear failed");
        }
        Ok(())
    }

    /// Microprocessor halted immediately after reset
    pub fn reset_mcu_and_halt(&mut self) -> Result<()> {
        self.ensure_mcu_halt()?;

        // Initiate a core reset request and hold the halt request.
        self.send_command(DmiOp::write(0x10, 0x80000003))?;
        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if dmstatus.allhavereset() && dmstatus.anyhavereset() {
            log::debug!("Reseted");
        } else {
            log::debug!("Reset failed")
        }
        // Clear the reset status signal and hold the halt request
        loop {
            self.send_command(DmiOp::write(0x10, 0x90000001))?;
            let dmstatus = self.dmi_read::<Dmstatus>()?;
            if !dmstatus.allhavereset() && !dmstatus.anyhavereset() {
                log::debug!("Reset status cleared");
                break;
            } else {
                log::warn!("Reset status clear failed")
            }
        }
        // Clear the halt request when the processor is reset and haltedd again
        self.send_command(DmiOp::write(0x10, 0x00000001))?;

        Ok(())
    }

    pub fn dump_regs(&mut self) -> Result<()> {
        let dpc = self.read_reg(regs::DPC)?;
        println!("dpc(pc):   0x{dpc:08x}");

        let gprs = if self
            .chip
            .as_ref()
            .map(|chip| chip.chip_family == RiscvChip::CH32V003)
            .unwrap_or(false)
        {
            &regs::GPRS_RV32EC[..]
        } else {
            &regs::GPRS[..]
        };

        for (reg, name, regno) in gprs {
            let val = self.read_reg(*regno)?;
            println!("{reg:<4}{name:>5}: 0x{val:08x}");
        }
        Ok(())
    }

    /// Read register value
    /// CSR: 0x0000 - 0x0fff
    /// GPR: 0x1000 - 0x101f
    /// FPR: 0x1020 - 0x103f
    // ref: QingKeV2 Microprocessor Debug Manual
    pub fn read_reg(&mut self, regno: u16) -> Result<u32> {
        // no need to halt when read register
        // self.ensure_mcu_halt()?;

        self.clear_abstractcs_cmderr()?;

        let reg = regno as u32;
        self.send_command(DmiOp::write(0x04, 0x00000000))?; // Clear the Data0 register
        self.send_command(DmiOp::write(0x17, 0x00220000 | (reg & 0xFFFF)))?;

        let abstractcs = self.dmi_read::<Abstractcs>()?;
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); // resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        let resp = self.send_command(DmiOp::read(0x04))?;

        Ok(resp.data)
    }

    pub fn write_reg(&mut self, regno: u16, value: u32) -> Result<()> {
        // self.ensure_mcu_halt()?;

        let reg = regno as u32;
        self.send_command(DmiOp::write(0x04, value))?;
        self.send_command(DmiOp::write(0x17, 0x00230000 | (reg & 0xFFFF)))?;

        let abstractcs = self.dmi_read::<Abstractcs>()?;
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        Ok(())
    }

    // via: SingleLineDebugMReset
    pub fn reset_debug_module(&mut self) -> Result<()> {
        self.ensure_mcu_halt()?;

        // Write command
        self.send_command(DmiOp::write(0x10, 0x00000003))?;

        let dmcontrol = self.dmi_read::<Dmcontrol>()?;
        if !(dmcontrol.dmactive() && dmcontrol.ndmreset()) {
            return Err(Error::Custom(
                "Value not written, DM reset might be not supported".into(),
            ));
        }

        // Write the debug module reset command
        self.send_command(DmiOp::write(0x10, 0x00000002))?;

        let dmcontrol = self.dmi_read::<Dmcontrol>()?;

        if !dmcontrol.ndmreset() {
            Ok(())
        } else {
            log::warn!("Reset is not successful");
            Ok(())
        }
    }

    pub fn dmi_read<R>(&mut self) -> Result<R>
    where
        R: DMReg,
    {
        let mut n = 0;
        loop {
            let resp = self.send_command(DmiOp::read(R::ADDR))?;
            if resp.op == 0x03 && resp.data == 0xffffffff && resp.addr == 0x7d {
                // special code for NotAttached
                return Err(Error::NotAttached);
            }
            if resp.is_success() {
                return Ok(R::from(resp.data));
            } else if n > 100 {
                return Err(Error::Timeout);
            } else if resp.is_busy() {
                sleep(Duration::from_millis(10));
                n += 1;
            } else {
                return Err(Error::DmiFailed);
            }
        }
    }

    pub fn dmi_write<R>(&mut self, reg: R) -> Result<()>
    where
        R: DMReg,
    {
        self.send_command(DmiOp::write(R::ADDR, reg.into()))?;
        Ok(())
    }
}

// marchid => dc68d882
// Parsed marchid: WCH-V4B
// Ref: QingKe V4 Manual
fn parse_marchid(marchid: u32) -> Option<String> {
    if marchid == 0 {
        None
    } else {
        Some(format!(
            "{}{}{}-{}{}{}",
            (((marchid >> 26) & 0x1F) + 64) as u8 as char,
            (((marchid >> 21) & 0x1F) + 64) as u8 as char,
            (((marchid >> 16) & 0x1F) + 64) as u8 as char,
            (((marchid >> 10) & 0x1F) + 64) as u8 as char,
            ((((marchid >> 5) & 0x1F) as u8) + b'0') as char,
            ((marchid & 0x1F) + 64) as u8 as char,
        ))
    }
}

fn parse_misa(misa: u32) -> Option<String> {
    let mut s = String::new();
    let mxl = (misa >> 30) & 0x3;
    s.push_str(match mxl {
        1 => "RV32",
        2 => "RV64",
        3 => "RV128",
        _ => return None,
    });
    for i in 0..26 {
        if (misa >> i) & 1 == 1 {
            s.push((b'A' + i as u8) as char);
        }
    }
    Some(s)
}
