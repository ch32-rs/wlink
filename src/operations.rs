//! Predefined operations for WCH-Link

use std::{thread::sleep, time::Duration};

use crate::{
    commands::{DmiOp, Program, ReadMemory, SetRamAddress},
    device::WchLink,
    error::Error,
    error::Result,
    flash_op,
    transport::Transport,
};

impl WchLink {
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

        let mut mem = self.device_handle.read_from_data_channel(length as usize)?;
        // Fix endian
        for chunk in mem.chunks_exact_mut(4) {
            chunk.reverse();
        }

        Ok(mem)
    }

    pub fn write_flash(&mut self, data: &[u8]) -> Result<()> {
        let mut data = data.to_vec();
        if data.len() % 256 != 0 {
            data.resize((data.len() / 256 + 1) * 256, 0);
        }
        self.send_command(SetRamAddress {
            start_addr: 0x0800_0000,
            len: data.len() as u32,
        })?;
        self.send_command(Program::BeginWriteMemory)?;
        self.device_handle
            .write_to_data_channel(&flash_op::CH32V307)?;

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
            sleep(Duration::from_millis(500));
        }
        // wlink_fastprogram

        self.send_command(Program::BeginWriteFlash)?;

        // TODO: send pack by pack
        self.device_handle.write_to_data_channel(&data)?;
        let rxbuf = self.device_handle.read_from_data_channel(4)?;
        if rxbuf[3] == 0x02 || rxbuf[3] == 0x04 {
            // success
            // wlink_endprogram
            self.send_command(Program::End)?;
            Ok(())
        } else {
            Err(Error::Custom("error while fastprogram".into()))
        }
    }

    pub fn halt_mcu(&mut self) -> Result<()> {
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        Ok(())
    }

    pub fn resume_mcu(&mut self) -> Result<()> {
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x10, 0x40000001))?;
        self.send_command(DmiOp::read(0x11))?;
        Ok(())
    }

    pub fn reset_mcu_and_run(&mut self) -> Result<()> {
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000003))?; // initiate a core reset request

        let dmcontrol = self.dmi_read(0x11)?;
        if (dmcontrol >> 18) & 0b11 == 0b11 {
            // reseted
            println!("reseted");
        } else {
            println!("not reseted");
        }

        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x10, 0x10000001))?;
        let dmcontrol = self.dmi_read(0x11)?;
        if (dmcontrol >> 18) & 0b11 == 0b00 {
            println!("reset status cleared");
        } else {
            println!("reset status not cleared");
        }
        Ok(())
    }

    pub fn read_csr(&mut self, csr: u16) -> Result<u32> {
        let csr = (csr as u32) & 0xfff;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x04, 0x00000000))?;
        self.send_command(DmiOp::write(0x17, 0x00220000 | csr))?;
        let _ = self.send_command(DmiOp::read(0x16))?;
        let resp = self.send_command(DmiOp::read(0x04))?;

        Ok(resp.data)
    }

    pub fn reset_debug_module(&mut self) -> Result<()> {
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x80000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000001))?;
        self.send_command(DmiOp::write(0x10, 0x00000003))?;
        if self.send_command(DmiOp::read(0x10))?.data != 0x00000003 {
            return Err(Error::Custom("Failed to reset debug module".into()));
        }
        self.send_command(DmiOp::write(0x10, 0x00000002))?;
        if self.send_command(DmiOp::read(0x10))?.data != 0xb0 {
            return Err(Error::Custom("Failed to reset debug module".into()));
        }

        Ok(())
    }

    pub fn dmi_read(&mut self, addr: u8) -> Result<u32> {
        loop {
            let resp = self.send_command(DmiOp::read(addr))?;
            if resp.is_success() {
                return Ok(resp.data);
            }
            sleep(Duration::from_millis(10));
        }
    }
}
