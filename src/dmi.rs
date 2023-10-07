use crate::{
    commands::DmiOp,
    device::WchLink,
    error::{AbstractcsCmdErr, Error, Result},
    regs::{Abstractcs, DMReg, Dmcontrol, Dmstatus},
};
use std::{thread, time::Duration};

// FPEC, OPTWRE to unlock,
pub const KEY1: u32 = 0x45670123;
pub const KEY2: u32 = 0xCDEF89AB;

/// RISC-V DMI
pub trait DebugModuleInterface {
    fn dmi_nop(&mut self) -> Result<()>;
    fn dmi_read(&mut self, reg: u8) -> Result<u32>;
    fn dmi_write(&mut self, reg: u8, value: u32) -> Result<()>;

    fn read_dmi_reg<R>(&mut self) -> Result<R>
    where
        R: DMReg,
    {
        let val = self.dmi_read(R::ADDR)?;
        Ok(R::from(val))
    }

    fn write_dmi_reg<R>(&mut self, reg: R) -> Result<()>
    where
        R: DMReg,
    {
        self.dmi_write(R::ADDR, reg.into())?;
        Ok(())
    }
}

impl DebugModuleInterface for WchLink {
    fn dmi_nop(&mut self) -> Result<()> {
        self.send_command(DmiOp::nop())?;
        Ok(())
    }

    fn dmi_read(&mut self, reg: u8) -> Result<u32> {
        let mut n = 0;
        loop {
            let resp = self.send_command(DmiOp::read(reg))?;
            if resp.op == 0x03 && resp.data == 0xffffffff && resp.addr == 0x7d {
                // special code for NotAttached
                return Err(Error::NotAttached);
            }
            if resp.is_success() {
                return Ok(resp.data);
            } else if n > 100 {
                return Err(Error::Timeout);
            } else if resp.is_busy() {
                log::warn!("dmi_read: busy, retrying");
                thread::sleep(Duration::from_millis(10));
                n += 1;
            } else {
                return Err(Error::DmiFailed);
            }
        }
    }

    fn dmi_write(&mut self, reg: u8, value: u32) -> Result<()> {
        self.send_command(DmiOp::write(reg, value))?;
        Ok(())
    }
}

pub enum HaltMode {
    NoReset,
    Reset,
    Reboot,
    Resume,
    /// Halt then go to bootloader
    Bootloader,
}

pub struct Algorigthm<'a, D: DebugModuleInterface> {
    dmi: &'a mut D,
}

impl<'a, D: DebugModuleInterface> Algorigthm<'a, D> {
    pub fn new(dmi: &'a mut D) -> Self {
        Self { dmi }
    }

    fn clear_abstractcs_cmderr(&mut self) -> Result<()> {
        let mut abstractcs = Abstractcs::from(0);
        abstractcs.set_cmderr(0b111);
        self.dmi.write_dmi_reg(abstractcs)?;
        Ok(())
    }

    pub fn ensure_mcu_halt(&mut self) -> Result<()> {
        let dmstatus = self.dmi.read_dmi_reg::<Dmstatus>()?;
        if dmstatus.allhalted() && dmstatus.anyhalted() {
            log::trace!("Already halted, nop");
        } else {
            loop {
                // Initiate a halt request
                self.dmi.dmi_write(0x10, 0x80000001)?;
                let dmstatus = self.dmi.read_dmi_reg::<Dmstatus>()?;
                if dmstatus.anyhalted() && dmstatus.allhalted() {
                    break;
                } else {
                    log::warn!("Not halt, try send");
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }

        // Clear the halt request bit.
        self.dmi.dmi_write(0x10, 0x00000001)?;

        Ok(())
    }

    pub fn reset_debug_module(&mut self) -> Result<()> {
        self.dmi.dmi_write(0x10, 0x00000000)?;
        self.dmi.dmi_write(0x10, 0x00000001)?;

        let dmcontrol = self.dmi.read_dmi_reg::<Dmcontrol>()?;

        if dmcontrol.dmactive() {
            Ok(())
        } else {
            Err(Error::DmiFailed)
        }
    }

    pub fn read_mem32(&mut self, addr: u32) -> Result<u32> {
        self.dmi.dmi_write(0x20, 0x0002a303)?; // lw x6,0(x5)
        self.dmi.dmi_write(0x21, 0x00100073)?; // ebreak

        self.dmi.dmi_write(0x04, addr)?; // data0 <- address
        self.clear_abstractcs_cmderr()?;

        self.dmi.dmi_write(0x17, 0x00271005)?;

        let abstractcs: Abstractcs = self.dmi.read_dmi_reg()?;
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        self.dmi.dmi_write(0x17, 0x00221006)?; // data0 <- x6
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        let data0 = self.dmi.dmi_read(0x04)?;
        Ok(data0)
    }

    pub fn write_mem32(&mut self, addr: u32, data: u32) -> Result<()> {
        // rasm2 -a riscv -d 23a07200
        // sw t2, 0(t0)
        self.dmi.dmi_write(0x20, 0x0072a023)?; // sw x7,0(x5)
        self.dmi.dmi_write(0x21, 0x00100073)?; // ebreak

        self.dmi.dmi_write(0x04, addr)?; // data0 <- address

        self.clear_abstractcs_cmderr()?;
        self.dmi.dmi_write(0x17, 0x00231005)?; // x5 <- data0

        let abstractcs: Abstractcs = self.dmi.read_dmi_reg()?;
        log::trace!("{:?}", abstractcs);
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        self.dmi.dmi_write(0x04, data)?; // data0 <- data
        self.clear_abstractcs_cmderr()?;

        self.dmi.dmi_write(0x17, 0x00271007)?; // x7 <- data0

        let abstractcs: Abstractcs = self.dmi.read_dmi_reg()?;
        log::trace!("{:?}", abstractcs);
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }
        Ok(())
    }

    pub fn write_mem8(&mut self, addr: u32, data: u8) -> Result<()> {
        self.dmi.dmi_write(0x20, 0x00728023)?; // sb x7,0(x5)
        self.dmi.dmi_write(0x21, 0x00100073)?; // ebreak

        self.dmi.dmi_write(0x04, addr)?; // data0 <- address

        self.clear_abstractcs_cmderr()?;
        self.dmi.dmi_write(0x17, 0x00231005)?; // x5 <- data0

        let abstractcs: Abstractcs = self.dmi.read_dmi_reg()?;
        log::trace!("{:?}", abstractcs);
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        self.dmi.dmi_write(0x04, data as u32)?; // data0 <- data
        self.clear_abstractcs_cmderr()?;

        self.dmi.dmi_write(0x17, 0x00271007)?; // x7 <- data0

        let abstractcs: Abstractcs = self.dmi.read_dmi_reg()?;
        log::trace!("{:?}", abstractcs);
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); //resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }
        Ok(())
    }

    /// Read register value
    /// CSR: 0x0000 - 0x0fff
    /// GPR: 0x1000 - 0x101f
    /// FPR: 0x1020 - 0x103f
    // ref: QingKeV2 Microprocessor Debug Manual
    pub fn read_reg(&mut self, regno: u16) -> Result<u32> {
        self.clear_abstractcs_cmderr()?;

        let reg = regno as u32;
        self.dmi.dmi_write(0x04, 0x00000000)?; // Clear the Data0 register
        self.dmi.dmi_write(0x17, 0x00220000 | (reg & 0xFFFF))?;

        let abstractcs = self.dmi.read_dmi_reg::<Abstractcs>()?;
        if abstractcs.busy() {
            return Err(Error::AbstractCommandError(AbstractcsCmdErr::Busy)); // resue busy
        }
        if abstractcs.cmderr() != 0 {
            AbstractcsCmdErr::try_from_cmderr(abstractcs.cmderr() as _)?;
        }

        let resp = self.dmi.dmi_read(0x04)?;

        Ok(resp)
    }

    pub fn dump_pmp(&mut self) -> Result<()> {
        let regs = [
            ("pmpcfg0", 0x3A0),
            ("pmpaddr0", 0x3B0),
            ("pmpaddr1", 0x3B1),
            ("pmpaddr2", 0x3B2),
            ("pmpaddr3", 0x3B3),
        ];
        for (name, addr) in regs.iter() {
            let val = self.read_reg(*addr)?;
            log::info!("{}: 0x{:08x}", name, val);
        }

        Ok(())
    }

    pub fn modify_mem32<F>(&mut self, addr: u32, f: F) -> Result<()>
    where
        F: FnOnce(u32) -> u32,
    {
        let data = self.read_mem32(addr)?;
        let data = f(data);
        self.write_mem32(addr, data)?;
        Ok(())
    }

    pub fn wait_mem32<F>(&mut self, addr: u32, until: F) -> Result<u32>
    where
        F: Fn(u32) -> bool,
    {
        loop {
            let data = self.read_mem32(addr)?;
            if until(data) {
                return Ok(data);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn read_memory(&mut self, addr: u32, len: u32) -> Result<Vec<u8>> {
        if len % 4 != 0 {
            return Err(Error::Custom("len must be 4 bytes aligned".to_string()));
        }

        let mut ret = Vec::with_capacity(len as usize);
        for i in 0..len / 4 {
            let data = self.read_mem32(addr + i * 4)?;
            ret.extend_from_slice(&data.to_le_bytes());
        }
        Ok(ret)
    }

    fn lock_flash(&mut self) -> Result<()> {
        const FLASH_CTLR: u32 = 0x40022010;

        self.modify_mem32(FLASH_CTLR, |r| r | 0x00008080)?;
        Ok(())
    }

    /// unlock FLASH LOCK and FLOCK
    fn unlock_flash(&mut self) -> Result<()> {
        const FLASH_CTLR: u32 = 0x40022010;
        const FLASH_KEYR: u32 = 0x40022004;
        const FLASH_MODEKEYR: u32 = 0x40022024;
        const KEY1: u32 = 0x45670123;
        const KEY2: u32 = 0xCDEF89AB;

        let flash_ctlr = self.read_mem32(FLASH_CTLR)?;
        log::debug!("flash_ctlr: 0x{:08x}", flash_ctlr);
        // Test LOCK, FLOCK bits
        if flash_ctlr & 0x00008080 == 0 {
            // already unlocked
            return Ok(());
        }
        // unlock LOCK
        self.write_mem32(FLASH_KEYR, KEY1)?;
        self.write_mem32(FLASH_KEYR, KEY2)?;

        // unlock FLOCK
        self.write_mem32(FLASH_MODEKEYR, KEY1)?;
        self.write_mem32(FLASH_MODEKEYR, KEY2)?;

        let flash_ctlr = self.read_mem32(FLASH_CTLR)?;
        log::debug!("flash_ctlr: 0x{:08x}", flash_ctlr);

        Ok(())
    }

    /// Erase by 256 bytes page
    /// address must be 256 bytes aligned
    pub fn fast_erase(&mut self, address: u32) -> Result<()> {
        // require unlock
        self.unlock_flash()?;

        const FLASH_STATR: u32 = 0x4002200C;
        const BUSY_MASK: u32 = 0x00000001;
        const START_MASK: u32 = 1 << 6;
        // const EOP_MASK: u32 = 1 << 5;
        const WPROTECT_ERR_MASK: u32 = 1 << 4;

        const FLASH_ADDR: u32 = 0x40022014;
        const FLASH_CTLR: u32 = 0x40022010;

        const PAGE_ERASE_MASK: u32 = 1 << 17;

        if address & 0xff != 0 {
            return Err(Error::Custom(
                "address must be 256 bytes aligned".to_string(),
            ));
        }

        let statr = self.read_mem32(FLASH_STATR)?;
        // check if busy
        if statr & BUSY_MASK != 0 {
            return Err(Error::Custom("flash busy".to_string()));
        }

        self.modify_mem32(FLASH_CTLR, |r| r | PAGE_ERASE_MASK)?;

        self.write_mem32(FLASH_ADDR, address)?;

        self.modify_mem32(FLASH_CTLR, |r| r | START_MASK)?;

        loop {
            let statr = self.read_mem32(FLASH_STATR)?;
            // check if busy
            if statr & BUSY_MASK != 0 {
                thread::sleep(Duration::from_millis(1));
            } else {
                if statr & WPROTECT_ERR_MASK != 0 {
                    return Err(Error::Custom("flash write protect error".to_string()));
                }
                self.write_mem32(FLASH_STATR, statr)?; // write 1 to clear EOP

                break;
            }
        }
        // read 1 word to verify
        let word = self.read_mem32(address)?;
        println!("=> {:08x}", word);

        // end erase, disable page erase
        self.modify_mem32(FLASH_CTLR, |r| r & (!PAGE_ERASE_MASK))?;

        self.lock_flash()?;

        Ok(())
    }

    pub fn fast_erase_32k(&mut self, address: u32) -> Result<()> {
        // require unlock
        self.unlock_flash()?;

        const FLASH_STATR: u32 = 0x4002200C;
        const BUSY_MASK: u32 = 0x00000001;
        const START_MASK: u32 = 1 << 6;
        const WPROTECT_ERR_MASK: u32 = 1 << 4;

        const FLASH_ADDR: u32 = 0x40022014;
        const FLASH_CTLR: u32 = 0x40022010;

        const BLOCK_ERASE_32K_MASK: u32 = 1 << 18;

        if address & 0x7fff != 0 {
            return Err(Error::Custom(
                "address must be 32k bytes aligned".to_string(),
            ));
        }

        let statr = self.read_mem32(FLASH_STATR)?;
        // check if busy
        if statr & BUSY_MASK != 0 {
            return Err(Error::Custom("flash busy".to_string()));
        }

        self.modify_mem32(FLASH_CTLR, |r| r | BLOCK_ERASE_32K_MASK)?;

        self.write_mem32(FLASH_ADDR, address)?;

        self.modify_mem32(FLASH_CTLR, |r| r | START_MASK)?;

        loop {
            let statr = self.read_mem32(FLASH_STATR)?;
            // check if busy
            if statr & BUSY_MASK != 0 {
                thread::sleep(Duration::from_millis(1));
            } else {
                if statr & WPROTECT_ERR_MASK != 0 {
                    return Err(Error::Custom("flash write protect error".to_string()));
                }
                self.write_mem32(FLASH_STATR, statr)?; // write 1 to clear EOP

                break;
            }
        }
        // read 1 word to verify
        let word = self.read_mem32(address)?;
        println!("=> {:08x}", word);

        // end erase
        // disable page erase
        self.modify_mem32(FLASH_CTLR, |r| r & (!BLOCK_ERASE_32K_MASK))?;

        self.lock_flash()?;

        Ok(())
    }

    pub fn erase_all(&mut self) -> Result<()> {
        const FLASH_STATR: u32 = 0x4002200C;
        const BUSY_MASK: u32 = 0x00000001;

        const FLASH_CTLR: u32 = 0x40022010;
        const MASS_ERASE_MASK: u32 = 1 << 2; // MER
        const START_MASK: u32 = 1 << 6;

        self.unlock_flash()?;

        self.modify_mem32(FLASH_CTLR, |r| r | MASS_ERASE_MASK)?;

        self.modify_mem32(FLASH_CTLR, |r| r | START_MASK)?;

        let statr = self.wait_mem32(FLASH_STATR, |r| r & BUSY_MASK == 0)?;
        self.write_mem32(FLASH_STATR, statr)?; // write 1 to clear EOP

        // clear MER
        self.modify_mem32(FLASH_CTLR, |r| r & (!MASS_ERASE_MASK))?;

        Ok(())
    }

    /// Program bytes.
    ///
    /// # Arguments
    ///
    /// * `address` - The start address of the flash page to program.
    /// * `data` - The data to be written to the page.
    ///
    /// The page must be erased first
    pub fn program_page(&mut self, address: u32, data: &[u8]) -> Result<()> {
        // require unlock
        self.unlock_flash()?;

        const FLASH_STATR: u32 = 0x4002200C;
        const BUSY_MASK: u32 = 0x00000001;
        const WRITE_BUSY_MASK: u32 = 1 << 1;
        const WPROTECT_ERR_MASK: u32 = 1 << 4;

        const FLASH_CTLR: u32 = 0x40022010;
        const PAGE_START_MASK: u32 = 1 << 21; // start page program
        const PAGE_PROG_MASK: u32 = 1 << 16; //

        if address & 0xff != 0 {
            return Err(Error::Custom(
                "address must be 256 bytes aligned".to_string(),
            ));
        }

        // check if busy
        let statr = self.read_mem32(FLASH_STATR)?;
        if statr & BUSY_MASK != 0 {
            return Err(Error::Custom("flash busy".to_string()));
        }

        //let ctlr = self.read_mem32(FLASH_CTLR)?;
        //let ctlr = ctlr | PAGE_PROG_MASK;
        //self.write_mem32(FLASH_CTLR, ctlr)?;
        self.modify_mem32(FLASH_CTLR, |r| r | PAGE_PROG_MASK)?;

        for (i, word) in data.chunks(4).enumerate() {
            let word = u32::from_le_bytes(word.try_into().unwrap());
            self.write_mem32(address + (i as u32 * 4), word)?;

            // write busy wait
            self.wait_mem32(FLASH_STATR, |r| r & WRITE_BUSY_MASK == 0)?;
        }

        // start fast page program
        self.modify_mem32(FLASH_CTLR, |r| r | PAGE_START_MASK)?;

        // busy wait
        let statr = self.wait_mem32(FLASH_STATR, |r| r & BUSY_MASK == 0)?;

        self.write_mem32(FLASH_STATR, statr)?; // write 1 to clear EOP
        if statr & WPROTECT_ERR_MASK != 0 {
            return Err(Error::Custom("flash write protect error".to_string()));
        }

        // verify
        // read 1 word to verify
        //let word = self.read_mem32(address)?;
        //println!("=> {:08x}", word);

        // end program, clear PAGE_PROG
        //let ctlr = self.read_mem32(FLASH_CTLR)?;
        //let ctlr = ctlr & (!PAGE_PROG_MASK); // disable page erase
        //self.write_mem32(FLASH_CTLR, ctlr)?;
        self.modify_mem32(FLASH_CTLR, |r| r & (!PAGE_PROG_MASK))?;

        self.lock_flash()?;

        Ok(())
    }
}
