//! The probe - WCH-Link

use crate::commands::{self, RawCommand, Response};
use crate::{Error, Result, RiscvChip, usb_device};
use crate::{
    commands::control::ProbeInfo,
    usb_device::{DeviceSelector, USBDeviceBackend},
};
use std::fmt;

pub const VENDOR_ID: u16 = 0x1a86;
pub const PRODUCT_ID: u16 = 0x8010;

pub const ENDPOINT_OUT: u8 = 0x01;
pub const ENDPOINT_IN: u8 = 0x81;

pub const DATA_ENDPOINT_OUT: u8 = 0x02;
pub const DATA_ENDPOINT_IN: u8 = 0x82;

pub const VENDOR_ID_DAP: u16 = 0x1a86;
pub const PRODUCT_ID_DAP: u16 = 0x8012;

pub const ENDPOINT_OUT_DAP: u8 = 0x02;

/// All WCH-Link probe variants, see-also: <http://www.wch-ic.com/products/WCH-Link.html>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum WchLinkVariant {
    /// WCH-Link-CH549, does not support CH32V00X
    Ch549 = 1,
    /// WCH-LinkE-CH32V305
    #[default]
    ECh32v305 = 2,
    /// WCH-LinkS-CH32V203
    SCh32v203 = 3,
    /// WCH-LinkW-CH32V208
    WCh32v208 = 5,
}

impl WchLinkVariant {
    pub fn try_from_u8(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Ch549),
            2 | 0x12 => Ok(Self::ECh32v305),
            3 => Ok(Self::SCh32v203),
            5 | 0x85 => Ok(Self::WCh32v208),
            _ => Err(Error::UnknownLinkVariant(value)),
        }
    }

    /// CH549 variant does not support mode switch. re-program is needed.
    pub fn support_switch_mode(&self) -> bool {
        !matches!(self, WchLinkVariant::Ch549)
    }

    /// Only W, E mode support this, power functions
    pub fn support_power_funcs(&self) -> bool {
        matches!(self, WchLinkVariant::WCh32v208 | WchLinkVariant::ECh32v305)
    }

    /// Only E mode support SDR print functionality
    pub fn support_sdi_print(&self) -> bool {
        matches!(self, WchLinkVariant::ECh32v305)
    }

    /// Better use E variant, the Old CH549-based variant does not support all chips
    pub fn support_chip(&self, chip: RiscvChip) -> bool {
        match self {
            WchLinkVariant::Ch549 => !matches!(
                chip,
                RiscvChip::CH32V003 | RiscvChip::CH32X035 | RiscvChip::CH643
            ),
            WchLinkVariant::WCh32v208 => !matches!(
                chip,
                RiscvChip::CH56X | RiscvChip::CH57X | RiscvChip::CH582 | RiscvChip::CH59X
            ),
            _ => true,
        }
    }
}

impl fmt::Display for WchLinkVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WchLinkVariant::Ch549 => write!(f, "WCH-Link-CH549"),
            WchLinkVariant::ECh32v305 => write!(f, "WCH-LinkE-CH32V305"),
            WchLinkVariant::SCh32v203 => write!(f, "WCH-LinkS-CH32V203"),
            WchLinkVariant::WCh32v208 => write!(f, "WCH-LinkW-CH32V208"),
        }
    }
}

/// Abstraction of WchLink probe interface
#[derive(Debug)]
pub struct WchLink {
    pub(crate) device: Box<dyn USBDeviceBackend>,
    pub info: ProbeInfo,
}

/// Whether an attached probe can be flashed right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeMode {
    /// RV mode (`1a86:8010`): usable for flashing.
    Rv,
    /// DAP mode (`1a86:8012`): must be switched back to RV mode first.
    Dap,
}

/// A `-d` selector for one attached probe, plus how it can be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeChoice {
    /// Value to pass to `-d`. For DAP-mode probes this selects the probe for
    /// `mode-switch --rv`, not for flashing.
    pub selector: String,
    /// Short description shown next to it in an error message.
    pub description: String,
    /// Whether the probe is in RV mode and can be flashed.
    pub mode: ProbeMode,
}

/// Refuses to continue when more than one probe is attached. Zero or one is
/// fine: zero still fails later with a proper "probe not found" error.
///
/// Library users that would otherwise fall back to `open_nth(0)` can call this
/// first; `open_nth` itself stays selector-driven and cannot tell an explicit
/// `-d 0` from a default.
pub fn ensure_unambiguous_probe(choices: &[ProbeChoice]) -> Result<()> {
    if choices.len() > 1 {
        Err(Error::AmbiguousProbe(choices.to_vec()))
    } else {
        Ok(())
    }
}

/// Builds the `-d` selector and description for one attached probe.
///
/// When a serial is usable, the selector is the full `VID:PID:SERIAL` form: a
/// bare serial that is all decimal digits would be parsed as an index by
/// `DeviceSelector::from_str`. Devices without a usable serial fall back to an
/// index.
fn probe_choice(
    vid: u16,
    pid: u16,
    mode: ProbeMode,
    device: &usb_device::AttachedDevice,
) -> ProbeChoice {
    let mode_label = match mode {
        ProbeMode::Rv => "RV mode",
        ProbeMode::Dap => "DAP mode",
    };
    let (selector, description) = match (&device.serial, device.serial_selectable) {
        (Some(serial), true) => (
            format!("{vid:04x}:{pid:04x}:{serial}"),
            format!("WCH-Link#{} ({mode_label})", device.index),
        ),
        (Some(serial), false) => (
            device.index.to_string(),
            format!(
                "WCH-Link#{} ({mode_label}, index-only Windows driver: index order may differ; serial {serial})",
                device.index
            ),
        ),
        (None, _) => (
            device.index.to_string(),
            format!(
                "WCH-Link#{} ({mode_label}, no USB serial; index may change on replug)",
                device.index
            ),
        ),
    };
    ProbeChoice {
        selector,
        description,
        mode,
    }
}

impl WchLink {
    /// Opens a probe. `selector` takes either a `usize` index, which is the
    /// original behaviour, or a [`DeviceSelector`] naming a USB serial number.
    pub fn open_nth(selector: impl Into<DeviceSelector>) -> Result<Self> {
        let selector = &selector.into();
        let device = match crate::usb_device::open(VENDOR_ID, PRODUCT_ID, selector) {
            Ok(dev) => dev,
            Err(e) => {
                // Detect if it is in DAP mode
                if crate::usb_device::open(VENDOR_ID_DAP, PRODUCT_ID_DAP, selector).is_ok() {
                    return Err(Error::ProbeModeNotSupported);
                } else {
                    return Err(e);
                }
            }
        };
        let mut this = WchLink {
            device,
            info: Default::default(),
        };
        let info = this.send_command(commands::control::GetProbeInfo)?;
        this.info = info;

        log::info!("Connected to {}", this.info);

        Ok(this)
    }

    pub fn probe_info(&mut self) -> Result<ProbeInfo> {
        let info = self.send_command(commands::control::GetProbeInfo)?;
        log::info!("{}", info);
        self.info = info;
        Ok(info)
    }

    /// Attached probes and a `-d` selector for each, in both RV and DAP mode.
    ///
    /// A probe in DAP mode still counts: several attached probes make a
    /// selector-less operation ambiguous. It is not offered as an operation
    /// target, though, because `WchLink::open_nth` rejects DAP-mode probes.
    /// A serial is preferred when the probe's driver supports one, because an
    /// index shifts on replug or mode switch.
    pub fn probe_choices() -> Result<Vec<ProbeChoice>> {
        let mut choices = Vec::new();
        for (vid, pid, mode) in [
            (VENDOR_ID, PRODUCT_ID, ProbeMode::Rv),
            (VENDOR_ID_DAP, PRODUCT_ID_DAP, ProbeMode::Dap),
        ] {
            for device in usb_device::list_attached_devices(vid, pid)? {
                choices.push(probe_choice(vid, pid, mode, &device));
            }
        }
        Ok(choices)
    }

    pub fn list_probes() -> Result<()> {
        let devs = usb_device::list_devices(VENDOR_ID, PRODUCT_ID)?;
        let rv_count = devs.len();
        for dev in devs {
            println!("{} (RV mode)", dev)
        }
        let devs = usb_device::list_devices(VENDOR_ID_DAP, PRODUCT_ID_DAP)?;
        let dap_count = devs.len();
        for dev in devs {
            println!("{} (DAP mode)", dev)
        }
        if rv_count + dap_count > 1 {
            // A serial is only offered when the probe is bound to WinUSB; the
            // index-only Windows drivers have to be named by index.
            println!(
                "hint: use `-d <INDEX>` to select one of the probes above, or \
                 `-d 1a86:8010:<SERIAL>` when a WinUSB driver exposes a serial"
            );
        }
        Ok(())
    }

    /// Switch from DAP mode to RV mode
    // ref: https://github.com/cjacker/wchlinke-mode-switch/blob/main/main.c
    pub fn switch_from_rv_to_dap(selector: impl Into<DeviceSelector>) -> Result<()> {
        let mut probe = Self::open_nth(selector)?;

        if probe.info.variant.support_switch_mode() {
            log::info!("Switch mode for WCH-LinkRV");

            let _ = probe.send_command(RawCommand::<0xff>(vec![0x41]));
            Ok(())
        } else {
            log::error!("Cannot switch mode for WCH-LinkRV: not supported");
            Err(crate::Error::Custom(format!(
                "The probe {} does not support mode switch",
                probe.info.variant
            )))
        }
    }

    pub fn switch_from_dap_to_rv(selector: impl Into<DeviceSelector>) -> Result<()> {
        let selector = &selector.into();
        let mut dev = crate::usb_device::open(VENDOR_ID_DAP, PRODUCT_ID_DAP, selector)?;
        log::info!(
            "Switch mode WCH-LinkDAP {:04x}:{:04x} {}",
            VENDOR_ID_DAP,
            PRODUCT_ID_DAP,
            selector
        );

        let buf = [0x81, 0xff, 0x01, 0x52];
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        let _ = dev.write_endpoint(ENDPOINT_OUT_DAP, &buf);

        Ok(())
    }

    pub fn set_power_output_enabled(
        selector: impl Into<DeviceSelector>,
        cmd: commands::control::SetPower,
    ) -> Result<()> {
        let mut probe = Self::open_nth(selector)?;

        if !probe.info.variant.support_power_funcs() {
            return Err(Error::Custom(
                "Probe doesn't support power control".to_string(),
            ));
        }

        probe.send_command(cmd)?;

        match cmd {
            commands::control::SetPower::Enable3v3 => log::info!("Enable 3.3V Output"),
            commands::control::SetPower::Disable3v3 => log::info!("Disable 3.3V Output"),
            commands::control::SetPower::Enable5v => log::info!("Enable 5V Output"),
            commands::control::SetPower::Disable5v => log::info!("Disable 5V Output"),
        }

        Ok(())
    }

    fn write_raw_cmd(&mut self, buf: &[u8]) -> Result<()> {
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        self.device.write_endpoint(ENDPOINT_OUT, buf)?;
        Ok(())
    }

    fn read_raw_cmd_resp(&mut self) -> Result<Vec<u8>> {
        let mut buf = [0u8; 64];
        let bytes_read = self.device.read_endpoint(ENDPOINT_IN, &mut buf)?;

        let resp = buf[..bytes_read].to_vec();
        log::trace!(
            "recv {} {}",
            hex::encode(&resp[..3]),
            hex::encode(&resp[3..])
        );
        Ok(resp)
    }

    pub fn send_command<C: crate::commands::Command>(&mut self, cmd: C) -> Result<C::Response> {
        log::trace!("send command: {:?}", cmd);
        let raw = cmd.to_raw();
        self.write_raw_cmd(&raw)?;
        let resp = self.read_raw_cmd_resp()?;

        C::Response::from_raw(&resp)
    }

    pub(crate) fn read_data(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(n);
        let mut bytes_read = 0;
        while bytes_read < n {
            let mut chunk = vec![0u8; 64];
            let chunk_read = self.device.read_endpoint(DATA_ENDPOINT_IN, &mut chunk)?;
            buf.extend_from_slice(&chunk[..chunk_read]);
            bytes_read += chunk_read;
        }
        if bytes_read != n {
            return Err(crate::Error::InvalidPayloadLength);
        }
        log::trace!("read data ep {} bytes", bytes_read);
        if bytes_read <= 10 {
            log::trace!("recv data {}", hex::encode(&buf[..bytes_read]));
        }
        if bytes_read != n {
            log::warn!("read data ep {} bytes", bytes_read);
            return Err(Error::InvalidPayloadLength);
        }
        Ok(buf[..n].to_vec())
    }

    pub(crate) fn write_data(&mut self, buf: &[u8], packet_len: usize) -> Result<()> {
        self.write_data_with_progress(buf, packet_len, &|_| {})
    }

    pub(crate) fn write_data_with_progress(
        &mut self,
        buf: &[u8],
        packet_len: usize,
        progress_callback: &dyn Fn(usize),
    ) -> Result<()> {
        for chunk in buf.chunks(packet_len) {
            let mut chunk = chunk.to_vec();
            progress_callback(chunk.len());
            if chunk.len() < packet_len {
                chunk.resize(packet_len, 0xff);
            }
            log::trace!("write data ep {} bytes", chunk.len());
            self.device.write_endpoint(DATA_ENDPOINT_OUT, &chunk)?;
        }
        log::trace!("write data ep total {} bytes", buf.len());
        Ok(())
    }
}

/// Helper for SDI print
pub fn watch_serial() -> Result<()> {
    use serialport::SerialPortType;

    let port_info = serialport::available_ports()?
        .into_iter()
        .find(|port| {
            if let SerialPortType::UsbPort(info) = &port.port_type {
                info.vid == VENDOR_ID && info.pid == PRODUCT_ID
            } else {
                false
            }
        })
        .ok_or_else(|| Error::Custom("No serial port found".to_string()))?;
    log::debug!("Opening serial port: {:?}", port_info.port_name);

    let mut port = serialport::new(&port_info.port_name, 115200)
        .timeout(std::time::Duration::from_millis(1000))
        .open()?;

    log::trace!("Serial port opened: {:?}", port);

    let mut endl = true;
    loop {
        let mut buf = [0u8; 1024];
        match port.read(&mut buf) {
            Ok(n) => {
                let s = String::from_utf8_lossy(&buf[..n]);
                for c in s.chars() {
                    if c == '\r' || c == '\n' {
                        if endl {
                            // continous line break
                            println!("{}:", chrono::Local::now());
                        } else {
                            endl = true;
                            println!()
                        }
                    } else if endl {
                        print!(
                            "{}: {}",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                            c
                        );
                        endl = false;
                    } else {
                        print!("{}", c);
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => return Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usb_device::AttachedDevice;

    fn attached(index: usize, serial: Option<&str>, serial_selectable: bool) -> AttachedDevice {
        AttachedDevice {
            index,
            serial: serial.map(str::to_string),
            serial_selectable,
        }
    }

    #[test]
    fn rv_probe_with_a_serial_uses_the_probe_rs_selector() {
        let choice = probe_choice(
            VENDOR_ID,
            PRODUCT_ID,
            ProbeMode::Rv,
            &attached(0, Some("7E238F0602BE"), true),
        );
        assert_eq!(choice.selector, "1a86:8010:7E238F0602BE");
        assert_eq!(
            choice.selector.parse(),
            Ok(DeviceSelector::Serial("7E238F0602BE".into()))
        );
        assert_eq!(choice.description, "WCH-Link#0 (RV mode)");
    }

    #[test]
    fn dap_probe_uses_vid_pid_so_a_numeric_serial_is_not_an_index() {
        // A bare all-digit serial would be parsed as an index by
        // `DeviceSelector::from_str`; the full form keeps it a serial.
        let choice = probe_choice(
            VENDOR_ID_DAP,
            PRODUCT_ID_DAP,
            ProbeMode::Dap,
            &attached(2, Some("123456"), true),
        );
        assert_eq!(choice.selector, "1a86:8012:123456");
        assert_eq!(
            choice.selector.parse(),
            Ok(DeviceSelector::Serial("123456".into()))
        );
        assert_eq!(choice.mode, ProbeMode::Dap);
    }

    #[test]
    fn probes_without_a_serial_fall_back_to_an_index() {
        let choice = probe_choice(
            VENDOR_ID,
            PRODUCT_ID,
            ProbeMode::Rv,
            &attached(1, None, false),
        );
        assert_eq!(choice.selector, "1");
        assert_eq!(choice.selector.parse(), Ok(DeviceSelector::Index(1)));
        assert!(choice.description.contains("no USB serial"), "{choice:?}");
    }

    #[test]
    fn index_only_driver_probes_fall_back_to_an_index() {
        // Neither CH375 nor WCH's native driver can select by serial, so
        // suggesting one would fail; the serial stays in the description to
        // help identify the probe, with a warning that indices may differ.
        let choice = probe_choice(
            VENDOR_ID,
            PRODUCT_ID,
            ProbeMode::Rv,
            &attached(3, Some("D7708F065C7F"), false),
        );
        assert_eq!(choice.selector, "3");
        assert_eq!(choice.selector.parse(), Ok(DeviceSelector::Index(3)));
        assert!(
            choice.description.contains("index-only Windows driver"),
            "{choice:?}"
        );
        assert!(
            choice.description.contains("index order may differ"),
            "{choice:?}"
        );
        assert!(choice.description.contains("D7708F065C7F"), "{choice:?}");
    }

    #[test]
    fn error_lists_every_probe_with_a_selector() {
        let choice = |serial: &str, index: usize| ProbeChoice {
            selector: format!("1a86:8010:{serial}"),
            description: format!("WCH-Link#{index} (RV mode)"),
            mode: ProbeMode::Rv,
        };

        assert!(ensure_unambiguous_probe(&[]).is_ok());
        assert!(ensure_unambiguous_probe(&[choice("AAA", 0)]).is_ok());

        let choices = vec![
            choice("7E238F0602BE", 0),
            choice("F93D8F067E10", 1),
            choice("D7708F065C7F", 2),
        ];
        let err = ensure_unambiguous_probe(&choices)
            .expect_err("three attached probes must be ambiguous");
        let msg = err.to_string();
        assert!(msg.contains("3 WCH-Link probes attached"), "{msg}");
        assert!(msg.contains("Select one of the following"), "{msg}");
        assert_eq!(msg.lines().count(), 5, "{msg}");
        for c in &choices {
            assert!(msg.contains(&c.selector), "missing {}: {msg}", c.selector);
            assert!(
                msg.contains(&c.description),
                "missing {}: {msg}",
                c.description
            );
        }
        assert!(msg.contains("-d 1a86:8010:7E238F0602BE"), "{msg}");
    }

    #[test]
    fn dap_probes_are_not_offered_as_operation_targets() {
        let choices = vec![
            ProbeChoice {
                selector: "1a86:8010:AAA".into(),
                description: "WCH-Link#0 (RV mode)".into(),
                mode: ProbeMode::Rv,
            },
            ProbeChoice {
                selector: "1a86:8012:BBB".into(),
                description: "WCH-Link#0 (DAP mode)".into(),
                mode: ProbeMode::Dap,
            },
        ];
        let err = ensure_unambiguous_probe(&choices)
            .expect_err("two probes, one in DAP mode, must be ambiguous");
        let msg = err.to_string();
        assert!(msg.contains("2 WCH-Link probes attached"), "{msg}");
        assert!(msg.contains("-d 1a86:8010:AAA"), "{msg}");
        // A DAP-mode selector only appears in the mode-switch hint, never as
        // a `-d` line that could be copied into the failed command.
        assert!(
            !msg.lines()
                .any(|line| line.trim_start().starts_with("-d 1a86:8012:BBB")),
            "{msg}"
        );
        assert!(
            msg.contains("wlink -d 1a86:8012:BBB mode-switch --rv"),
            "{msg}"
        );
        assert!(msg.contains("cannot be used"), "{msg}");
    }

    #[test]
    fn all_dap_probes_report_no_usable_probe() {
        let choices = vec![
            ProbeChoice {
                selector: "1a86:8012:AAA".into(),
                description: "WCH-Link#0 (DAP mode)".into(),
                mode: ProbeMode::Dap,
            },
            ProbeChoice {
                selector: "1a86:8012:BBB".into(),
                description: "WCH-Link#1 (DAP mode)".into(),
                mode: ProbeMode::Dap,
            },
        ];
        let msg = ensure_unambiguous_probe(&choices)
            .expect_err("two DAP-mode probes must be ambiguous")
            .to_string();
        assert!(msg.contains("None of them is in RV mode"), "{msg}");
        assert!(!msg.contains("Select one of the following"), "{msg}");
        assert!(
            msg.contains("wlink -d 1a86:8012:AAA mode-switch --rv"),
            "{msg}"
        );
    }
}
