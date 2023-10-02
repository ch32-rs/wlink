use crate::{
    commands::DmiOp,
    device::WchLink,
    error::{AbstractcsCmdErr, Error, Result},
    regs::{Abstractcs, DMReg, Dmstatus},
};
use std::{thread, time::Duration};

// FPEC, OPTWRE to unlock,
const KEY1: u32 = 0x45670123;
const KEY2: u32 = 0xCDEF89AB;

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

    pub fn unlock_flash(&mut self) {}

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
}
