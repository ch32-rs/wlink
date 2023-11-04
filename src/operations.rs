//! Predefined operations for WCH-Link

use indicatif::ProgressBar;
use std::{thread::sleep, time::Duration};

use crate::{
    commands::{
        self,
        control::{self, ProbeInfo},
        DmiOp, Program, SetReadMemoryRegion, SetWriteMemoryRegion,
    },
    device::{ChipInfo, WchLink},
    dmi::DebugModuleInterface,
    error::AbstractcsCmdErr,
    regs::{self, Abstractcs, Dmcontrol, Dmstatus},
    transport::Transport,
    Error, Result, RiscvChip,
};

impl WchLink {
    pub fn probe_info(&mut self) -> Result<ProbeInfo> {
        let info = self.send_command(commands::control::GetProbeInfo)?;
        log::info!("{}", info);
        self.probe = Some(info);
        Ok(info)
    }

    /// Attach chip and get chip info
    pub fn attach_chip(&mut self, expected_chip: Option<RiscvChip>) -> Result<()> {
        if self.chip.is_some() {
            log::warn!("Chip already attached");
        }

        let probe_info = self.probe_info()?;

        if let Some(chip) = expected_chip {
            if !probe_info.variant.support_chip(chip) {
                log::error!("WCH-Link doesn't support the choosen MCU, please use WCH-LinkE!");
                return Err(Error::UnsupportedChip(chip));
            }
        }

        let mut chip_info = None;
        for _ in 0..3 {
            self.send_command(commands::SetSpeed {
                riscvchip: expected_chip.unwrap_or(RiscvChip::CH32V103) as u8,
                speed: self.speed,
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
                // set speed again
                if expected_chip.is_none() {
                    self.send_command(commands::SetSpeed {
                        riscvchip: resp.chip_family as u8,
                        speed: self.speed,
                    })?;
                }

                break;
            } else {
                log::debug!("retrying...");
                sleep(Duration::from_millis(100));
            }
        }
        let chip_info = chip_info.ok_or(Error::NotAttached)?;

        chip_info.chip_family.post_init(self)?;

        //let ret = self.send_command(control::CheckQE)?;
        //log::info!("Check QE: {:?}", ret);

        // riscvchip = 7 => 2
        //let flash_addr = chip_info.chip_family.code_flash_start();
        //let page_size = chip_info.chip_family.data_packet_size();

        let info = ChipInfo {
            uid: None, // TODO
            chip_family: chip_info.chip_family,
            chip_id: chip_info.chip_id,
            march: None,
        };

        self.chip = Some(info);
        self.probe = Some(probe_info);

        Ok(())
    }

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

    pub fn protect_flash(&mut self, protect: bool) -> Result<()> {
        // HACK: requires a fresh attach
        self.reattach_chip()?;

        let flash_protected_flag = self.send_command(commands::ConfigChip::CheckReadProtect)?;
        let protected = flash_protected_flag == commands::ConfigChip::FLAG_PROTECTED;
        if protect == protected {
            log::info!(
                "Flash already {}",
                if protected {
                    "protected"
                } else {
                    "unprotected"
                }
            );
        }

        let use_v2 = self.probe.as_ref().unwrap().version() >= (2, 9);
        let cmd = match (protect, use_v2) {
            (true, true) => commands::ConfigChip::ProtectEx(0xbf),
            (true, false) => commands::ConfigChip::Protect,
            (false, true) => commands::ConfigChip::UnprotectEx(0xbf),
            (false, false) => commands::ConfigChip::Unprotect,
        };
        self.send_command(cmd)?;

        self.send_command(commands::Reset::ResetAndRun)?; // quit reset
        self.send_command(control::AttachChip)?;

        let flash_protected = self.send_command(commands::ConfigChip::CheckReadProtect)?;
        log::info!(
            "Flash protected: {}",
            flash_protected == commands::ConfigChip::FLAG_PROTECTED
        );

        Ok(())
    }

    pub fn enable_sdi_print(&mut self, enable: bool) -> Result<()> {
        if !self.probe.as_ref().unwrap().variant.support_sdi_print() {
            return Err(Error::Custom(
                "Probe doesn't support sdi printf functionality".to_string(),
            ));
        }
        if !self.chip.as_ref().unwrap().chip_family.support_sdi_print() {
            return Err(Error::Custom(
                "Chip doesn't support sdi printf functionality".to_string(),
            ));
        }

        if enable {
            self.send_command(control::SetSDIPrint::Enable)?;
        } else {
            self.send_command(control::SetSDIPrint::Disable)?;
        }
        Ok(())
    }

    // wlink_endprocess
    /// Detach chip and let it resume
    pub fn detach_chip(&mut self) -> Result<()> {
        log::debug!("Detach chip");
        self.send_command(commands::control::OptEnd)?;
        self.chip = None;
        Ok(())
    }

    fn reattach_chip(&mut self) -> Result<()> {
        let current_chip_family = self.chip.as_ref().map(|i| i.chip_family);
        self.detach_chip()?;
        self.attach_chip(current_chip_family)?;
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
        self.send_command(SetReadMemoryRegion {
            start_addr: address,
            len: length,
        })?;
        self.send_command(Program::ReadMemory)?;

        let mut mem = self.device_handle.read_data_endpoint(length as usize)?;
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

    /// Clear All Code Flash - By Power off
    pub fn erase_flash_by_power_off(&mut self, target_chip: Option<RiscvChip>) -> Result<()> {
        if !self.probe.as_ref().unwrap().variant.support_power_funcs() {
            return Err(Error::Custom(
                "Probe doesn't support power off erase".to_string(),
            ));
        }

        let chip_family = target_chip.and(self.chip.clone().map(|c| c.chip_family));
        if let Some(chip_family) = chip_family {
            if chip_family.support_special_erase() {
                self.send_command(control::EraseCodeFlash::ByPowerOff(chip_family))?;
                return Ok(());
            }
        } else {
            log::error!("--chip not specified");
        }

        Err(Error::Custom(
            "Chip doesn't support power off erase".to_string(),
        ))
    }

    /// Clear All Code Flash - By RST pin
    pub fn erase_flash_by_rst_pin(&mut self, target_chip: Option<RiscvChip>) -> Result<()> {
        if !self.probe.as_ref().unwrap().variant.support_power_funcs() {
            return Err(Error::Custom(
                "Probe doesn't support power off erase".to_string(),
            ));
        }

        let chip_family = target_chip.and(self.chip.clone().map(|c| c.chip_family));
        if let Some(chip_family) = chip_family {
            if chip_family.support_special_erase() {
                self.send_command(control::EraseCodeFlash::ByPinRST(chip_family))?;
                return Ok(());
            }
        } else {
            log::error!("--chip not specified");
        }
        return Err(Error::Custom(
            "Chip doesn't support RST pin erase".to_string(),
        ));
    }

    /// Erases flash and re-attach
    pub fn erase_flash(&mut self) -> Result<()> {
        if self
            .chip
            .as_ref()
            .unwrap()
            .chip_family
            .support_flash_protect()
        {
            let ret = self.send_command(commands::ConfigChip::CheckReadProtect)?;
            if ret == commands::ConfigChip::FLAG_PROTECTED {
                log::warn!("Flash is protected, unprotecting...");
                self.protect_flash(false)?;
            } else if ret == 2 {
                self.protect_flash(false)?;
            } else {
                log::warn!("Unknown flash protect status: {}", ret);
            }
        }
        self.send_command(Program::EraseFlash)?;
        self.send_command(control::AttachChip)?;

        Ok(())
    }

    // wlink_write
    pub fn write_flash(&mut self, data: &[u8], address: u32) -> Result<()> {
        let chip_family = self.chip.as_ref().unwrap().chip_family;
        let write_pack_size = chip_family.write_pack_size();
        let data_packet_size = chip_family.data_packet_size();

        if chip_family.support_flash_protect() {
            self.protect_flash(false)?;
        }

        let data = data.to_vec();

        //        if data.len() % data_packet_size != 0 {
        //          data.resize((data.len() / data_packet_size + 1) * data_packet_size, 0xff);
        //        log::debug!("Data resized to {}", data.len());
        //  }
        log::debug!(
            "Using write pack size {} data pack size {}",
            write_pack_size,
            data_packet_size
        );

        // wlink_ready_write
        // self.send_command(Program::Prepare)?; // no need for CH32V307
        self.send_command(SetWriteMemoryRegion {
            start_addr: address,
            len: data.len() as _,
        })?;

        // if self.chip.as_ref().unwrap().chip_family == RiscvChip::CH32V103 {}
        self.send_command(Program::WriteFlashOP)?;
        // wlink_ramcodewrite
        self.device_handle.write_data_endpoint(
            self.chip.as_ref().unwrap().chip_family.flash_op(),
            data_packet_size,
        )?;

        log::debug!("Flash OP written");

        let n = self.send_command(Program::Unknown07AfterFlashOPWritten)?;
        if n != 0x07 {
            return Err(Error::Custom(
                "Unknown07AfterFlashOPWritten failed".to_string(),
            ));
        }

        // wlink_fastprogram
        let bar = ProgressBar::new(data.len() as _);

        self.send_command(Program::WriteFlash)?;
        for chunk in data.chunks(write_pack_size as usize) {
            self.device_handle.write_data_endpoint_with_progress(
                chunk,
                data_packet_size,
                &|nbytes| {
                    bar.inc(nbytes as _);
                },
            )?;
            let rxbuf = self.device_handle.read_data_endpoint(4)?;
            // 41 01 01 04
            if rxbuf[3] != 0x04 {
                return Err(Error::Custom(format!(
                    // 0x05
                    // 0x18
                    // 0xff
                    "Error while fastprogram: {:02x?}",
                    rxbuf
                )));
            }
        }
        bar.finish();

        log::debug!("Fastprogram done");

        // wlink_endprogram
        let _ = self.send_command(Program::End)?;

        Ok(())
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

    // SingleLineExitPauseMode
    pub fn ensure_mcu_resume(&mut self) -> Result<()> {
        self.clear_dmstatus_havereset()?;
        let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
        if dmstatus.allrunning() && dmstatus.anyrunning() {
            log::debug!("Already running, nop");
            return Ok(());
        }

        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x10, 0x40000001))?;

        let dmstatus = self.read_dmi_reg::<Dmstatus>()?;
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

    fn clear_dmstatus_havereset(&mut self) -> Result<()> {
        let mut dmcontrol = self.read_dmi_reg::<Dmcontrol>()?;
        dmcontrol.set_ackhavereset(true);
        self.write_dmi_reg(dmcontrol)?;
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

        for (reg, regno) in &regs::CSRS {
            let val = self.read_reg(*regno)?;
            println!("{reg:<9}: 0x{val:08x}");
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

        let abstractcs = self.read_dmi_reg::<Abstractcs>()?;
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

        let abstractcs = self.read_dmi_reg::<Abstractcs>()?;
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

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

/// SDI print
pub fn watch_serial() -> Result<()> {
    use serialport::SerialPortType;

    let port_info = serialport::available_ports()?
        .into_iter()
        .find(|port| {
            if let SerialPortType::UsbPort(info) = &port.port_type {
                info.vid == crate::device::VENDOR_ID && info.pid == crate::device::PRODUCT_ID
            } else {
                false
            }
        })
        .ok_or_else(|| Error::Custom("No serial port found".to_string()))?;
    log::debug!("Opening serial port: {:?}", port_info.port_name);

    let mut port = serialport::new(&port_info.port_name, 115200)
        .timeout(Duration::from_millis(1000))
        .open()?;

    log::trace!("Serial port opened: {:?}", port);

    loop {
        let mut buf = [0u8; 1024];
        match port.read(&mut buf) {
            Ok(n) => {
                if n > 0 {
                    let s = String::from_utf8_lossy(&buf[..n]);
                    print!("{}", s);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => return Err(e.into()),
        }
    }
}
