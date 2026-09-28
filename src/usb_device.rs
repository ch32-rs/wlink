//! USB Device abstraction - The USB Device of WCH-Link.

use crate::Result;
use std::{
    fmt::Debug,
    time::Duration,
};

pub mod libusb;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub mod wch_link_dll;

#[cfg(all(target_os = "windows"))]
pub mod wch_link_driver;

pub trait USBDeviceBackend: Debug {
    fn set_timeout(&mut self, _timeout: Duration) {}

    fn read_endpoint(&mut self, ep: u8, buf: &mut [u8]) -> Result<usize>;

    fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>>
    where
        Self: Sized;

    fn write_endpoint(&mut self, ep: u8, buf: &[u8]) -> Result<()>;
}

pub fn open_nth(vid: u16, pid: u16, nth: usize) -> Result<Box<dyn USBDeviceBackend>> {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    if let Ok(backend) = wch_link_dll::CH375USBDevice::open_nth(vid, pid, nth) {
        return Ok(backend);
    }

    #[cfg(all(target_os = "windows"))]
    if let Ok(backend) = wch_link_driver::WCHLinkUSBDevice::open_nth(vid, pid, nth) {
        return Ok(backend);
    }

    libusb::NusbDevice::open_nth(vid, pid, nth)
}

pub fn list_devices(vid: u16, pid: u16) -> Result<Vec<String>> {
    let mut ret = vec![];

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        ret.extend(
            wch_link_dll::list_devices(vid, pid)?
                .into_iter()
                .map(|s| s.to_string()),
        );
    }

    #[cfg(all(target_os = "windows"))]
    {
        ret.extend(
            wch_link_driver::list_devices(vid, pid)?
                .into_iter()
                .map(|s| s.to_string()),
        );
    }

    ret.extend(
        libusb::list_libusb_devices(vid, pid)?
            .into_iter()
            .map(|s| s.to_string()),
    );

    Ok(ret)
}
