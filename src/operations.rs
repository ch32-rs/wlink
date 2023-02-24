//! Predefined operations for WCH-Link

use crate::{
    commands::{DmiOp, Program, ReadMemory},
    device::WchLink,
    error::Error,
    error::Result,
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
        self.send_command(Program::PerformOperation)?;

        let mut mem = self.device_handle.read_from_data_channel(length as usize)?;
        // Fix endian
        for chunk in mem.chunks_exact_mut(4) {
            chunk.reverse();
        }

        Ok(mem)
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
}
