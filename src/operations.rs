//! Predefined operations for WCH-Link

use indicatif::ProgressBar;
use std::{thread::sleep, time::Duration};

use crate::{
    commands::{self, Speed},
    probe::WchLink,
    Error, Result, RiscvChip,
};

/// A running probe session, flash, erase, inspect, etc.
pub struct ProbeSession {
    pub probe: WchLink,
    pub chip_family: RiscvChip,
    pub speed: Speed,
}

impl ProbeSession {
    /// Attach probe to target chip, start a probe session
    pub fn attach(probe: WchLink, expected_chip: Option<RiscvChip>, speed: Speed) -> Result<Self> {
        let mut probe = probe;

        let chip = expected_chip.unwrap_or(RiscvChip::CH32V103);

        if !probe.info.variant.support_chip(chip) {
            log::error!(
                "Current WCH-Link variant doesn't support the choosen MCU, please use WCH-LinkE!"
            );
            return Err(Error::UnsupportedChip(chip));
        }

        let mut chip_info = None;

        for _ in 0..3 {
            probe.send_command(commands::SetSpeed {
                riscvchip: chip as u8,
                speed,
            })?;

            if let Ok(resp) = probe.send_command(commands::control::AttachChip) {
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
                // set speed again
                if expected_chip.is_none() {
                    probe.send_command(commands::SetSpeed {
                        riscvchip: resp.chip_family as u8,
                        speed,
                    })?;
                }

                break;
            } else {
                log::debug!("retrying...");
                sleep(Duration::from_millis(100));
            }
        }

        let chip_info = chip_info.ok_or(Error::NotAttached)?;
        chip_info.chip_family.do_post_init(&mut probe)?;

        //let ret = self.send_command(control::CheckQE)?;
        //log::info!("Check QE: {:?}", ret);
        // riscvchip = 7 => 2
        //let flash_addr = chip_info.chip_family.code_flash_start();
        //let page_size = chip_info.chip_family.data_packet_size();

        Ok(ProbeSession {
            probe,
            chip_family: chip_info.chip_family,
            speed,
        })
    }

    pub fn detach_chip(&mut self) -> Result<()> {
        log::trace!("Detach chip");
        self.probe.send_command(commands::control::OptEnd)?;
        Ok(())
    }

    fn reattach_chip(&mut self) -> Result<()> {
        log::debug!("Reattach chip");
        self.detach_chip()?;
        let _ = self.probe.send_command(commands::control::AttachChip)?;
        Ok(())
    }

    // NOTE: this halts the MCU
    pub fn dump_info(&mut self) -> Result<()> {
        if self.chip_family.support_query_info() {
            let esig = if self.probe.info.version() >= (2, 9) {
                self.probe.send_command(commands::GetChipInfo::V2)?
            } else {
                self.probe.send_command(commands::GetChipInfo::V1)?
            };
            log::info!("Chip ESIG: {esig}");

            let flash_protected = self
                .probe
                .send_command(commands::ConfigChip::CheckReadProtect)?;
            let protected = flash_protected == commands::ConfigChip::FLAG_READ_PROTECTED;
            log::info!("Flash protected: {}", protected);
            if protected {
                log::warn!("Flash is protected, debug access is not available");
            }
        }
        if self.chip_family.support_ram_rom_mode() {
            let sram_code_mode = self
                .probe
                .send_command(commands::control::GetChipRomRamSplit)?;
            log::debug!("SRAM CODE split mode: {}", sram_code_mode);
        }
        /*
        if detailed {

        }
        */
        Ok(())
    }

    pub fn unprotect_flash(&mut self) -> Result<()> {
        // HACK: requires a fresh attach
        self.reattach_chip()?;

        let read_protected = self
            .probe
            .send_command(commands::ConfigChip::CheckReadProtect)?;
        if read_protected == commands::ConfigChip::FLAG_READ_PROTECTED {
            log::info!("Flash already unprotected");
        }

        self.probe.send_command(commands::ConfigChip::Unprotect)?;

        self.reattach_chip()?;

        let read_protected = self
            .probe
            .send_command(commands::ConfigChip::CheckReadProtect)?;
        log::info!(
            "Read protected: {}",
            read_protected == commands::ConfigChip::FLAG_READ_PROTECTED
        );

        let write_protected = self
            .probe
            .send_command(commands::ConfigChip::CheckReadProtectEx)?;
        if write_protected == commands::ConfigChip::FLAG_WRITE_PROTECTED {
            log::warn!("Flash is write protected!");
            log::warn!("try to unprotect...");
            self.probe
                .send_command(commands::ConfigChip::UnprotectEx(0xff))?; // FIXME: 0xff or 0xbf

            self.reattach_chip()?;

            let write_protected = self
                .probe
                .send_command(commands::ConfigChip::CheckReadProtectEx)?;
            println!(
                "Write protected: {}",
                write_protected == commands::ConfigChip::FLAG_WRITE_PROTECTED
            );
        }

        Ok(())
    }

    pub fn protect_flash(&mut self) -> Result<()> {
        // HACK: requires a fresh attach
        self.reattach_chip()?;

        let read_protected = self
            .probe
            .send_command(commands::ConfigChip::CheckReadProtect)?;
        if read_protected == commands::ConfigChip::FLAG_READ_PROTECTED {
            log::warn!("Flash already protected");
        }

        self.probe.send_command(commands::ConfigChip::Protect)?;

        self.reattach_chip()?;

        let read_protected = self
            .probe
            .send_command(commands::ConfigChip::CheckReadProtect)?;
        log::info!(
            "Read protected: {}",
            read_protected == commands::ConfigChip::FLAG_READ_PROTECTED
        );

        Ok(())
    }

    /// Clear cmderror

    /// Erases flash and re-attach
    pub fn erase_flash(&mut self) -> Result<()> {
        if self.chip_family.support_flash_protect() {
            let ret = self
                .probe
                .send_command(commands::ConfigChip::CheckReadProtect)?;
            if ret == commands::ConfigChip::FLAG_READ_PROTECTED {
                log::warn!("Flash is protected, unprotecting...");
                self.unprotect_flash()?;
            } else if ret == 2 {
                self.unprotect_flash()?; // FIXME: 2 is unknown
            } else {
                log::warn!("Unknown flash protect status: {}", ret);
            }
        }
        self.probe.send_command(commands::Program::EraseFlash)?;
        self.probe.send_command(commands::control::AttachChip)?;

        Ok(())
    }

    // wlink_write
    pub fn write_flash(&mut self, data: &[u8], address: u32) -> Result<()> {
        let chip_family = self.chip_family;
        let write_pack_size = chip_family.write_pack_size();
        let data_packet_size = chip_family.data_packet_size();

        if chip_family.support_flash_protect() {
            self.unprotect_flash()?;
        }

        let data = data.to_vec();

        // if data.len() % data_packet_size != 0 {
        //     data.resize((data.len() / data_packet_size + 1) * data_packet_size, 0xff);
        //     log::debug!("Data resized to {}", data.len());
        // }
        log::debug!(
            "Using write pack size {} data pack size {}",
            write_pack_size,
            data_packet_size
        );

        // wlink_ready_write
        // self.send_command(Program::Prepare)?; // no need for CH32V307
        self.probe.send_command(commands::SetWriteMemoryRegion {
            start_addr: address,
            len: data.len() as _,
        })?;

        // if self.chip.as_ref().unwrap().chip_family == RiscvChip::CH32V103 {}
        self.probe.send_command(commands::Program::WriteFlashOP)?;
        // wlink_ramcodewrite
        let flash_op = self.chip_family.get_flash_op();
        self.probe.write_data(flash_op, data_packet_size)?;

        log::debug!("Flash OP written");

        let n = self
            .probe
            .send_command(commands::Program::Unknown07AfterFlashOPWritten)?;
        if n != 0x07 {
            return Err(Error::Custom(
                "Unknown07AfterFlashOPWritten failed".to_string(),
            ));
        }

        // wlink_fastprogram
        let bar = ProgressBar::new(data.len() as _);

        self.probe.send_command(commands::Program::WriteFlash)?;
        for chunk in data.chunks(write_pack_size as usize) {
            self.probe
                .write_data_with_progress(chunk, data_packet_size, &|nbytes| {
                    bar.inc(nbytes as _);
                })?;
            let rxbuf = self.probe.read_data(4)?;
            // 41 01 01 04
            if rxbuf[3] != 0x04 {
                return Err(Error::Custom(format!(
                    // 0x05, 0x18, 0xff
                    "Error while fastprogram: {:02x?}",
                    rxbuf
                )));
            }
        }
        bar.finish();

        log::debug!("Fastprogram done");

        // wlink_endprogram
        let _ = self.probe.send_command(commands::Program::End)?;

        Ok(())
    }

    pub fn soft_reset(&mut self) -> Result<()> {
        self.probe.send_command(commands::Reset::Soft)?; // quit reset
        Ok(())
    }

    /// Read a continuous memory region, require MCU to be halted
    pub fn read_memory(&mut self, address: u32, length: u32) -> Result<Vec<u8>> {
        let mut length = length;
        if length % 4 != 0 {
            length = (length / 4 + 1) * 4;
        }
        self.probe.send_command(commands::SetReadMemoryRegion {
            start_addr: address,
            len: length,
        })?;
        self.probe.send_command(commands::Program::ReadMemory)?;

        let mut mem = self.probe.read_data(length as usize)?;
        // Fix endian
        for chunk in mem.chunks_exact_mut(4) {
            chunk.reverse();
        }

        if mem.starts_with(&[0xA9, 0xBD, 0xF9, 0xF3]) {
            log::warn!("A9 BD F9 F3 sequence detected!");
            log::warn!("If the chip is just put into debug mode, you should flash the new firmware to the chip first");
            log::warn!("Or else this indicates a reading to invalid location");
        }

        Ok(mem)
    }

    pub fn set_sdi_print_enabled(&mut self, enable: bool) -> Result<()> {
        if !self.probe.info.variant.support_sdi_print() {
            return Err(Error::Custom(
                "Probe doesn't support SDI print functionality".to_string(),
            ));
        }
        if !self.chip_family.support_sdi_print() {
            return Err(Error::Custom(
                "Chip doesn't support SDI print functionality".to_string(),
            ));
        }

        self.probe
            .send_command(commands::control::SetSdiPrintEnabled(enable))?;
        Ok(())
    }

    /// Clear All Code Flash - By Power off
    pub fn erase_flash_by_power_off(probe: &mut WchLink, chip_family: RiscvChip) -> Result<()> {
        if !probe.info.variant.support_power_funcs() {
            return Err(Error::Custom(
                "Probe doesn't support power off erase".to_string(),
            ));
        }
        if !chip_family.support_special_erase() {
            return Err(Error::Custom(
                "Chip doesn't support power off erase".to_string(),
            ));
        }

        probe.send_command(commands::SetSpeed {
            riscvchip: chip_family as u8,
            speed: Speed::default(),
        })?;
        probe.send_command(commands::control::EraseCodeFlash::ByPowerOff(chip_family))?;
        Ok(())
    }

    /// Clear All Code Flash - By RST pin
    pub fn erase_flash_by_rst_pin(probe: &mut WchLink, chip_family: RiscvChip) -> Result<()> {
        if !probe.info.variant.support_power_funcs() {
            return Err(Error::Custom(
                "Probe doesn't support reset pin erase".to_string(),
            ));
        }
        if !chip_family.support_special_erase() {
            return Err(Error::Custom(
                "Chip doesn't support reset pin erase".to_string(),
            ));
        }

        probe.send_command(commands::SetSpeed {
            riscvchip: chip_family as u8,
            speed: Speed::default(),
        })?;
        probe.send_command(commands::control::EraseCodeFlash::ByPinRST(chip_family))?;
        Ok(())
    }
}

/*

    // NOTE: this halts the MCU, so it's not suitable except for dumping info
    pub fn dump_info(&mut self, detailed: bool) -> Result<()> {
        let probe_info = self.probe.as_ref().unwrap();
        let chip_family = self.chip.as_ref().unwrap().chip_family;

        if chip_family.support_query_info() {
            let chip_id = if probe_info.version() >= (2, 9) {
                self.send_command(commands::GetChipInfo::V2)?
            } else {
                self.send_command(commands::GetChipInfo::V1)?
            };
            log::info!("Chip UID: {chip_id}");

            let flash_protected = self.send_command(commands::ConfigChip::CheckReadProtect)?;
            let protected = flash_protected == commands::ConfigChip::FLAG_PROTECTED;
            log::info!("Flash protected: {}", protected);
            if protected {
                log::warn!("Flash is protected, debug access is not available");
            }
        }
        if chip_family.support_ram_rom_mode() {
            let sram_code_mode = self.send_command(commands::control::GetChipRomRamSplit)?;
            log::debug!("SRAM CODE split mode: {}", sram_code_mode);
        }

        if detailed {
            let misa = self.read_reg(regs::MISA)?;
            log::trace!("Read csr misa: {misa:08x}");
            let misa = parse_misa(misa);
            log::info!("RISC-V ISA: {misa:?}");

            // detect chip's RISC-V core version, QingKe cores
            let marchid = self.read_reg(regs::MARCHID)?;
            log::trace!("Read csr marchid: {marchid:08x}");
            let core_type = parse_marchid(marchid);
            log::info!("RISC-V arch: {core_type:?}");
        }
        Ok(())
    }




    // wlink_endprocess

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
        self.send_command(SetReadMemoryRegion {
            start_addr: address,
            len: length,
        })?;
        self.send_command(Program::ReadMemory)?;

        let mut mem = self.read_data_ep(length as usize)?;
        // Fix endian
        for chunk in mem.chunks_exact_mut(4) {
            chunk.reverse();
        }

        if mem.starts_with(&[0xA9, 0xBD, 0xF9, 0xF3]) {
            log::warn!("A9 BD F9 F3 sequence detected!");
            log::warn!("If the chip is just put into debug mode, you should flash the new firmware to the chip first");
            log::warn!("Or else this indicates a reading to invalid location");
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





    pub fn ensure_mcu_halt(&mut self) -> Result<()> {
        let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
        if dmstatus.allhalted() && dmstatus.anyhalted() {
            log::trace!("Already halted, nop");
        } else {
            loop {
                // Initiate a halt request
                self.send_command(DmiOp::write(0x10, 0x80000001))?;
                let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
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



    /// Write a memory word, require MCU to be halted.
    ///
    /// V2 microprocessor debug module abstract command only supports the register access mode,
    /// So this function will use the register access mode to write a memory word,
    /// instead of using the memory access mode.
    pub fn write_memory_word(&mut self, address: u32, data: u32) -> Result<()> {
        // self.ensure_mcu_halt()?;

        self.send_command(DmiOp::write(0x20, 0x0072a023))?; // sw x7,0(x5)
        self.send_command(DmiOp::write(0x21, 0x00100073))?; // ebreak
        self.send_command(DmiOp::write(0x04, address))?; // data0 <- address
        self.clear_abstractcs_cmderr()?;
        self.send_command(DmiOp::write(0x17, 0x00231005))?; // x5 <- data0

        let abstractcs = self.read_dmi_reg::<Abstractcs>()?;
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
        let abstractcs = self.read_dmi_reg::<Abstractcs>()?;
        log::trace!("{:?}", abstractcs);
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        Ok(())
    }


    /// Clear cmderror field of abstractcs register.
    /// write 1 to clean the corresponding bit.
    fn clear_abstractcs_cmderr(&mut self) -> Result<()> {
        let mut abstractcs = self.read_dmi_reg::<Abstractcs>()?;
        abstractcs.set_cmderr(0b111);
        self.write_dmi_reg(abstractcs)?;
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
        let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
        println!("{:?}", dmstatus);
        if dmstatus.allhavereset() && dmstatus.anyhavereset() {
            // reseted
            log::debug!("Reseted");
        } else {
            log::warn!("Reset failed");
        }

        // Clear the reset status signal
        self.send_command(DmiOp::write(0x10, 0x10000001))?; // ackhavereset
        let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
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
        let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
        if dmstatus.allhavereset() && dmstatus.anyhavereset() {
            log::debug!("Reseted");
        } else {
            log::debug!("Reset failed")
        }
        // Clear the reset status signal and hold the halt request
        loop {
            self.send_command(DmiOp::write(0x10, 0x90000001))?;
            let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
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




}
*/
