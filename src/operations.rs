//! Predefined operations for WCH-Link

use std::{thread::sleep, time::Duration};

use crate::{
    commands::{self, DmiOp, Program, ReadMemory, SetRamAddress},
    device::{ChipInfo, WchLink},
    error::AbstractcsCmdErr,
    regs::{self, Abstractcs, DMReg, Dmcontrol, Dmstatus},
    transport::Transport,
    Error, Result,
};

impl WchLink {
    pub fn probe_info(&mut self) -> Result<()> {
        let info = self.send_command(commands::control::GetProbeInfo)?;
        log::info!("{}", info);
        Ok(())
    }
    /// Attach chip and get chip info
    pub fn attach_chip(&mut self) -> Result<()> {
        if self.chip.is_some() {
            log::warn!("chip already attached");
        }
        let chip_info = self.send_command(commands::control::AttachChip)?;
        log::info!("attached chip: {}", chip_info);

        let uid = self.send_command(commands::GetChipId)?;
        log::debug!("Chip UID: {uid}");

        self.send_command(commands::GetFlashProtected)?;
        let flash_protected = self.send_command(commands::GetFlashProtected)?;
        log::debug!("flash protected: {}", flash_protected);

        let sram_code_mode = self.send_command(commands::control::GetChipRomRamSplit)?;
        log::debug!("SRAM CODE mode: {}", sram_code_mode);

        // detect chip's RISC-V core version, QingKe cores
        let marchid = self.read_reg(regs::MARCHID)?;
        log::trace!("read csr marchid: {marchid:08x}");
        let core_type = parse_marchid(marchid);
        log::debug!("RISC-V core version: {core_type:?}");

        // riscvchip = 7 => 2
        let flash_addr = chip_info.chip_family.code_flash_start();
        let page_size = chip_info.chip_family.page_size();

        let info = ChipInfo {
            uid,
            flash_protected,
            chip_family: chip_info.chip_family,
            chip_type: chip_info.chip_type,
            march: core_type,
            flash_size: 0, // TODO: read flash size
            memory_start_addr: flash_addr,
            sram_code_mode,
            page_size,
            rom_kb: 0, // TODO:
            ram_kb: 0, // TODO:
        };

        self.chip = Some(info);

        Ok(())
    }

    // wlink_endprocess
    /// Detach chip and let it resume
    pub fn detach_chip(&mut self) -> Result<()> {
        log::debug!("detach chip");
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

    pub fn write_flash(&mut self, data: &[u8]) -> Result<()> {
        let pack_size = self.chip.as_ref().unwrap().chip_family.write_pack_size();
        let code_start_addr = self.chip.as_ref().unwrap().memory_start_addr;
        log::debug!("Code start address 0x{:08x}", code_start_addr);

        let mut data = data.to_vec();
        if data.len() % 256 != 0 {
            data.resize((data.len() / 256 + 1) * 256, 0);
        }
        self.send_command(SetRamAddress {
            start_addr: code_start_addr,
            len: data.len() as u32,
        })?;
        self.send_command(Program::BeginWriteMemory)?;
        self.device_handle
            .write_data_endpoint(self.chip.as_ref().unwrap().chip_family.flash_op())?;

        log::debug!("flash op written");

        for i in 0.. {
            // check written
            if let Ok(n) = self.send_command(Program::ExecMemory) {
                if n == 0x07 {
                    break;
                }
            }
            if i > 10 {
                return Err(Error::Custom("timeout while write flash".into()));
            }
            sleep(Duration::from_millis(10));
        }
        // wlink_fastprogram
        self.send_command(Program::BeginWriteFlash)?;

        for chunk in data.chunks(pack_size as usize) {
            self.device_handle.write_data_endpoint(chunk)?;
            let rxbuf = self.device_handle.read_data_endpoint(4)?;
            if rxbuf[3] != 0x02 && rxbuf[3] != 0x04 {
                return Err(Error::Custom("error while fastprogram".into()));
            }
        }
        log::debug!("fastprogram done");

        Ok(())
    }

    pub fn ensure_mcu_halt(&mut self) -> Result<()> {
        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if dmstatus.allhalted() && dmstatus.anyhalted() {
            log::trace!("already halted, nop");
        } else {
            loop {
                self.send_command(DmiOp::write(0x10, 0x80000001))?;
                let dmstatus = self.dmi_read::<Dmstatus>()?;
                if dmstatus.anyhalted() && dmstatus.allhalted() {
                    break;
                } else {
                    log::warn!("not halt, try send");
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
            log::debug!("already running, nop");
            return Ok(());
        }

        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x10, 0x40000001))?;

        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if dmstatus.allresumeack() && dmstatus.anyresumeack() {
            log::debug!("resumed");
            Ok(())
        } else {
            log::warn!("resume fails");
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
            log::debug!("reseted");
        } else {
            log::warn!("reset failed");
        }

        // Clear the reset status signal
        self.send_command(DmiOp::write(0x10, 0x10000001))?; // ackhavereset
        let dmstatus = self.dmi_read::<Dmstatus>()?;
        if !dmstatus.allhavereset() && !dmstatus.anyhavereset() {
            log::debug!("reset status cleared");
        } else {
            log::warn!("reset status clear failed");
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
            log::debug!("reseted");
        } else {
            log::debug!("reset failed")
        }
        // Clear the reset status signal and hold the halt request
        loop {
            self.send_command(DmiOp::write(0x10, 0x90000001))?;
            let dmstatus = self.dmi_read::<Dmstatus>()?;
            if !dmstatus.allhavereset() && !dmstatus.anyhavereset() {
                log::debug!("reset status cleared");
                break;
            } else {
                log::warn!("reset status clear failed")
            }

        }
        // Clear the halt request when the processor is reset and haltedd again
        self.send_command(DmiOp::write(0x10, 0x00000001))?;

        Ok(())
    }

    pub fn dump_regs(&mut self) -> Result<()> {
        let dpc = self.read_reg(regs::DPC)?;
        println!("dpc(pc):   0x{dpc:08x}");
        for (reg, name, regno) in regs::GPRS {
            let val = self.read_reg(regno)?;
            println!("{reg:<4}{name:>5}: 0x{val:08x}");
        }
        Ok(())
    }

    /// Read register value
    /// CSR: 0x0000 - 0x0fff
    /// GPR: 0x1000 - 0x101f
    /// FPR: 0x1020 - 0x103f
    pub fn read_reg(&mut self, regno: u16) -> Result<u32> {
        self.ensure_mcu_halt()?;

        let reg = regno as u32;
        self.send_command(DmiOp::write(0x04, 0x00000000))?;
        self.send_command(DmiOp::write(0x17, 0x00220000 | reg))?;

        let abstractcs = self.dmi_read::<Abstractcs>()?;
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        let resp = self.send_command(DmiOp::read(0x04))?;

        Ok(resp.data)
    }

    pub fn write_reg(&mut self, regno: u16, value: u32) -> Result<()> {
        self.ensure_mcu_halt()?;

        let reg = regno as u32;
        self.send_command(DmiOp::write(0x04, value))?;
        self.send_command(DmiOp::write(0x17, 0x00230000 | reg))?;

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
            log::warn!("reset is not successful");
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
