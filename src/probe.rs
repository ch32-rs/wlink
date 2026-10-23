//! The probe - WCH-Link

use crate::commands::{self, RawCommand, Response};
use crate::{Error, Result, RiscvChip, usb_device};
use crate::{commands::control::ProbeInfo, usb_device::USBDeviceBackend};
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

impl WchLink {
    pub fn open_nth(nth: usize) -> Result<Self> {
        let device = match crate::usb_device::open_nth(VENDOR_ID, PRODUCT_ID, nth) {
            Ok(dev) => dev,
            Err(e) => {
                // Detect if it is in DAP mode
                if crate::usb_device::open_nth(VENDOR_ID_DAP, PRODUCT_ID_DAP, nth).is_ok() {
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

    pub fn list_probes() -> Result<()> {
        let devs = usb_device::list_devices(VENDOR_ID, PRODUCT_ID)?;
        for dev in devs {
            println!("{} (RV mode)", dev)
        }
        let devs = usb_device::list_devices(VENDOR_ID_DAP, PRODUCT_ID_DAP)?;
        for dev in devs {
            println!("{} (DAP mode)", dev)
        }
        Ok(())
    }

    /// Switch from DAP mode to RV mode
    // ref: https://github.com/cjacker/wchlinke-mode-switch/blob/main/main.c
    pub fn switch_from_rv_to_dap(nth: usize) -> Result<()> {
        let mut probe = Self::open_nth(nth)?;

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

    pub fn switch_from_dap_to_rv(nth: usize) -> Result<()> {
        let mut dev = crate::usb_device::open_nth(VENDOR_ID_DAP, PRODUCT_ID_DAP, nth)?;
        log::info!(
            "Switch mode WCH-LinkDAP {:04x}:{:04x} #{}",
            VENDOR_ID_DAP,
            PRODUCT_ID_DAP,
            nth
        );

        let buf = [0x81, 0xff, 0x01, 0x52];
        log::trace!("send {} {}", hex::encode(&buf[..3]), hex::encode(&buf[3..]));
        let _ = dev.write_endpoint(ENDPOINT_OUT_DAP, &buf);

        Ok(())
    }

    pub fn set_power_output_enabled(nth: usize, cmd: commands::control::SetPower) -> Result<()> {
        let mut probe = Self::open_nth(nth)?;

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
    let port_info = find_wch_link_serial_port(serialport::available_ports()?)
        .ok_or_else(|| Error::Custom("No serial port found".to_string()))?;
    log::debug!("Opening serial port: {:?}", port_info.port_name);

    let mut port = serialport::new(&port_info.port_name, 115200)
        .timeout(std::time::Duration::from_millis(1000))
        .open()?;

    log::trace!("Serial port opened: {:?}", port);

    let mut printer = TimestampedSerialPrinter::default();
    let mut sink = StdoutSerialWatchSink;
    watch_serial_port(&mut *port, &mut printer, &mut sink)
}

fn find_wch_link_serial_port(
    ports: impl IntoIterator<Item = serialport::SerialPortInfo>,
) -> Option<serialport::SerialPortInfo> {
    ports.into_iter().find(is_wch_link_serial_port)
}

fn is_wch_link_serial_port(port: &serialport::SerialPortInfo) -> bool {
    if let serialport::SerialPortType::UsbPort(info) = &port.port_type {
        info.vid == VENDOR_ID && info.pid == PRODUCT_ID
    } else {
        false
    }
}

trait SerialWatchSink {
    fn write_str(&mut self, s: &str) -> Result<()>;
}

struct StdoutSerialWatchSink;

impl SerialWatchSink for StdoutSerialWatchSink {
    fn write_str(&mut self, s: &str) -> Result<()> {
        use std::io::Write;

        std::io::stdout().write_all(s.as_bytes())?;
        std::io::stdout().flush()?;
        Ok(())
    }
}

struct TimestampedSerialPrinter {
    line_start: bool,
}

impl Default for TimestampedSerialPrinter {
    fn default() -> Self {
        Self { line_start: true }
    }
}

impl TimestampedSerialPrinter {
    fn write_bytes(&mut self, bytes: &[u8], sink: &mut dyn SerialWatchSink) -> Result<()> {
        self.write_str(&String::from_utf8_lossy(bytes), sink)
    }

    fn write_str(&mut self, s: &str, sink: &mut dyn SerialWatchSink) -> Result<()> {
        self.write_str_with_timestamp(s, sink, || {
            chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string()
        })
    }

    fn write_str_with_timestamp(
        &mut self,
        s: &str,
        sink: &mut dyn SerialWatchSink,
        timestamp: impl Fn() -> String,
    ) -> Result<()> {
        for c in s.chars() {
            if c == '\r' || c == '\n' {
                if self.line_start {
                    // continuous line break
                    sink.write_str(&format!("{}:\n", chrono::Local::now()))?;
                } else {
                    self.line_start = true;
                    sink.write_str("\n")?;
                }
            } else if self.line_start {
                sink.write_str(&format!("{}: {}", timestamp(), c))?;
                self.line_start = false;
            } else {
                sink.write_str(&c.to_string())?;
            }
        }
        Ok(())
    }
}

fn watch_serial_port(
    port: &mut dyn serialport::SerialPort,
    printer: &mut TimestampedSerialPrinter,
    sink: &mut dyn SerialWatchSink,
) -> Result<()> {
    loop {
        let mut buf = [0u8; 1024];
        match port.read(&mut buf) {
            Ok(n) => printer.write_bytes(&buf[..n], sink)?,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => return Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct StringSink(String);

    impl SerialWatchSink for StringSink {
        fn write_str(&mut self, s: &str) -> Result<()> {
            self.0.push_str(s);
            Ok(())
        }
    }

    fn usb_port(name: &str, vid: u16, pid: u16) -> serialport::SerialPortInfo {
        serialport::SerialPortInfo {
            port_name: name.to_string(),
            port_type: serialport::SerialPortType::UsbPort(serialport::UsbPortInfo {
                vid,
                pid,
                serial_number: None,
                manufacturer: None,
                product: None,
            }),
        }
    }

    #[test]
    fn matches_wch_link_serial_port_by_vid_pid() {
        let port = usb_port("/dev/ttyACM0", VENDOR_ID, PRODUCT_ID);
        assert!(is_wch_link_serial_port(&port));
    }

    #[test]
    fn rejects_non_wch_link_serial_port() {
        let port = usb_port("/dev/ttyACM0", VENDOR_ID, PRODUCT_ID_DAP);
        assert!(!is_wch_link_serial_port(&port));
    }

    #[test]
    fn finds_first_wch_link_serial_port() {
        let ports = vec![
            usb_port("/dev/ttyACM0", 0x1234, 0x5678),
            usb_port("/dev/ttyACM1", VENDOR_ID, PRODUCT_ID),
        ];

        let port = find_wch_link_serial_port(ports).unwrap();
        assert_eq!(port.port_name, "/dev/ttyACM1");
    }

    #[test]
    fn timestamped_printer_defaults_to_line_start() {
        let mut printer = TimestampedSerialPrinter::default();
        let mut sink = StringSink::default();

        printer
            .write_str_with_timestamp("a", &mut sink, || "TS".to_string())
            .unwrap();

        assert_eq!(sink.0, "TS: a");
    }

    #[test]
    fn timestamped_printer_keeps_existing_line_format() {
        let mut printer = TimestampedSerialPrinter { line_start: true };
        let mut sink = StringSink::default();

        printer
            .write_str_with_timestamp("abc\ndef", &mut sink, || "TS".to_string())
            .unwrap();

        assert_eq!(sink.0, "TS: abc\nTS: def");
    }

    #[test]
    fn timestamped_printer_preserves_partial_line_across_chunks() {
        let mut printer = TimestampedSerialPrinter { line_start: true };
        let mut sink = StringSink::default();

        printer
            .write_str_with_timestamp("ab", &mut sink, || "TS".to_string())
            .unwrap();
        printer
            .write_str_with_timestamp("c\n", &mut sink, || "TS".to_string())
            .unwrap();

        assert_eq!(sink.0, "TS: abc\n");
    }
}
