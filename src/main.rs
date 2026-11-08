use std::{thread::sleep, time::Duration};

use anyhow::Result;
use wlink::{
    RiscvChip, commands,
    dmi::DebugModuleInterface,
    firmware::{Firmware, read_firmware_from_file},
    operations::ProbeSession,
    probe::{ProbeChoice, WchLink},
    regs,
    usb_device::DeviceSelector,
};

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Which probe to operate on: an index from `wlink list`, or a stable
    /// `VID:PID:SERIAL` (same form as probe-rs' PROBE_RS_PROBE)
    #[arg(long, short = 'd', value_name = "INDEX|VID:PID:SERIAL")]
    device: Option<DeviceSelector>,

    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,

    /// Detach chip after operation
    #[arg(long, global = true, default_value = "false")]
    no_detach: bool,

    /// Specify the chip type
    #[arg(long, global = true, ignore_case = true)]
    chip: Option<RiscvChip>,

    /// Connection Speed
    #[arg(long, global = true, default_value = "high")]
    speed: crate::commands::Speed,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum EraseMode {
    /// Erase code flash by power off, the probe will power off the target chip
    PowerOff,
    /// Erase code flash by RST pin, the probe will active the nRST line. Requires a RST pin connection
    PinRst,
    /// Erase code flash by probe command
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ResetMode {
    /// Quit reset
    Quit,
    /// Reset and run
    Run,
    /// Reset and halt
    Halt,
    /// Reset DM(Debug module)
    Dm,
}

#[derive(Subcommand)]
enum Commands {
    /// Dump memory region
    Dump {
        /// Start address
        #[arg(value_parser = parse_number)]
        address: u32,

        /// Length in bytes, will be rounded up to the next multiple of 4
        #[arg(value_parser = parse_number)]
        length: u32,

        /// Write the dumped memory region to a file
        #[arg(short = 'o', long = "out")]
        filename: Option<String>,
    },
    /// Dump registers
    Regs {},
    /// Erase flash
    Erase {
        /// Erase mode
        #[arg(long, default_value = "default")]
        method: EraseMode,
    },
    /// Program the code flash
    Flash {
        /// Address in u32
        #[arg(short, long, value_parser = parse_number)]
        address: Option<u32>,
        /// Erase flash before flashing
        #[arg(long, short, default_value = "false")]
        erase: bool,
        /// Do not reset and run after flashing
        #[arg(long, short = 'R', default_value = "false")]
        no_run: bool,
        /// Enable SDI print after reset
        #[arg(long, default_value = "false")]
        enable_sdi_print: bool,
        /// Open serial port(print only) after reset
        #[arg(long, default_value = "false")]
        watch_serial: bool,
        /// Path to the firmware file to flash
        path: String,
    },
    /// Unlock flash
    Unprotect {},
    /// Protect flash
    Protect {},
    /// Force set register
    WriteReg {
        /// Reg in u16
        #[arg(value_parser = parse_number)]
        reg: u32,
        /// Value in u32
        #[arg(value_parser = parse_number)]
        value: u32,
    },
    /// Force write a memory word
    WriteMem {
        /// Address in u32
        #[arg(value_parser = parse_number)]
        address: u32,
        /// Value in u32
        #[arg(value_parser = parse_number)]
        value: u32,
    },
    /// Halts the MCU
    Halt {},
    /// Resumes the MCU
    Resume {},
    /// Reset the MCU
    Reset {
        /// Reset mode
        #[arg(default_value = "quit")]
        mode: ResetMode,
        /// Enable SDI print after reset
        #[arg(long, default_value = "false")]
        enable_sdi_print: bool,
        /// Open serial port(SDI print) after reset
        #[arg(long, default_value = "false")]
        watch_serial: bool,
    },
    /// Debug, check status
    Status {},
    /// Switch mode from RV to DAP or vice versa
    ModeSwitch {
        #[arg(long)]
        rv: bool,
        #[arg(long)]
        dap: bool,
    },
    /// List probes
    List {},
    /// Enable or disable power output
    SetPower {
        #[command(subcommand)]
        cmd: commands::control::SetPower,
    },
    /// SDI virtual serial port,
    #[command(subcommand)]
    SdiPrint(SdiPrint),
    Dev {},
}

#[derive(clap::Subcommand, PartialEq, Clone, Copy, Debug)]
pub enum SdiPrint {
    /// Enable SDI print, implies --no-detach
    Enable,
    /// Disable SDI print
    Disable,
}

impl SdiPrint {
    pub fn is_enable(&self) -> bool {
        *self == SdiPrint::Enable
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // init simplelogger
    // log timestamps in the local timezone instead of UTC,
    // silently falling back to UTC if the local offset is unavailable
    let log_config = simplelog::ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap_or_else(|builder| builder)
        .build();
    simplelog::TermLogger::init(
        cli.verbose.log_level_filter(),
        log_config,
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("initialize simple logger");

    // Without `-d` a flash uses whichever probe enumerates first. Refuse that
    // when several probes are attached, so a firmware image is never written
    // to the wrong board. Other commands keep the historical behaviour.
    if flashing_without_explicit_probe(&cli.device, &cli.command) {
        let choices = WchLink::probe_choices().map_err(|e| {
            anyhow::anyhow!(
                "cannot enumerate WCH-Link probes ({e}); refusing to flash without an explicit probe. \
                 Pass `-d <INDEX>` or `-d VID:PID:<SERIAL>` to select one"
            )
        })?;
        ensure_unambiguous_flash_probe(&choices)?;
    }

    // No `-d` keeps the historical "first probe" behaviour.
    let device = cli.device.unwrap_or(DeviceSelector::Index(0));
    let mut will_detach = !cli.no_detach;

    match cli.command {
        None => {
            WchLink::list_probes()?;

            println!("No command given, use --help for help.");
            println!("hint: use `wlink status` to get started.");
        }
        Some(Commands::ModeSwitch { rv, dap }) => {
            WchLink::list_probes()?;
            log::warn!("This is an experimental feature, better use the WCH-LinkUtility!");
            if !(rv ^ dap) {
                println!("Please choose one mode to switch, either --rv or --dap");
            } else if dap {
                WchLink::switch_from_rv_to_dap(device)?;
            } else {
                WchLink::switch_from_dap_to_rv(device)?;
            }
        }
        Some(Commands::List {}) => {
            WchLink::list_probes()?;
        }
        Some(Commands::SetPower { cmd }) => {
            WchLink::set_power_output_enabled(device, cmd)?;
        }

        Some(Commands::Erase { method }) if method != EraseMode::Default => {
            // Special handling for non-default erase: bypass attach chip
            // So a chip family info is required, no detection
            let chip_family = cli.chip.ok_or(wlink::Error::Custom(
                "--chip required to do a special erase".into(),
            ))?;

            let mut probe = WchLink::open_nth(device)?;
            log::info!("Erase chip by {:?}", method);
            match method {
                EraseMode::PowerOff => {
                    ProbeSession::erase_flash_by_power_off(&mut probe, chip_family)?;
                }
                EraseMode::PinRst => {
                    log::warn!("Code flash erase by RST pin requires a RST pin connection");
                    ProbeSession::erase_flash_by_rst_pin(&mut probe, chip_family)?;
                }
                _ => unreachable!(),
            }
        }
        Some(command) => {
            let probe = WchLink::open_nth(device)?;
            let mut sess = ProbeSession::attach(probe, cli.chip, cli.speed)?;

            match command {
                Commands::Dev {} => {
                    // dev only
                }
                Commands::Dump {
                    address,
                    length,
                    filename,
                } => {
                    log::info!(
                        "Read memory from 0x{:08x} to 0x{:08x}",
                        address,
                        address + length
                    );

                    let out = sess.read_memory(address, length)?;

                    if let Some(fname) = filename {
                        std::fs::write(&fname, &out)?;
                        log::info!("{} bytes written to file {}", length, &fname);
                    } else {
                        println!(
                            "{}",
                            nu_pretty_hex::config_hex(
                                &out,
                                nu_pretty_hex::HexConfig {
                                    title: true,
                                    ascii: true,
                                    address_offset: address as _,
                                    ..Default::default()
                                },
                            )
                        );
                    }
                }
                Commands::Regs {} => {
                    log::info!("Dump GPRs");
                    sess.dump_regs()?;
                    sess.dump_pmp_csrs()?;
                }
                Commands::WriteReg { reg, value } => {
                    let regno = reg as u16;
                    log::info!("Set reg 0x{:04x} to 0x{:08x}", regno, value);
                    sess.write_reg(regno, value)?;
                }
                Commands::WriteMem { address, value } => {
                    log::info!("Write memory 0x{:08x} to 0x{:08x}", value, address);
                    sess.write_mem32(address, value)?;
                }
                Commands::Halt {} => {
                    log::info!("Halt MCU");
                    sess.reset_debug_module()?;
                    sess.ensure_mcu_halt()?;

                    will_detach = false; // detach will resume the MCU

                    let dmstatus: regs::Dmstatus = sess.probe.read_dmi_reg()?;
                    log::info!("{dmstatus:#x?}");
                }
                Commands::Resume {} => {
                    log::info!("Resume MCU");
                    sess.ensure_mcu_resume()?;

                    let dmstatus: regs::Dmstatus = sess.probe.read_dmi_reg()?;
                    log::info!("{dmstatus:#?}");
                }
                Commands::Erase { method } => {
                    log::info!("Erase Flash...");
                    match method {
                        EraseMode::Default => {
                            sess.erase_flash()?;
                        }
                        _ => unreachable!(),
                    }
                    log::info!("Erase done");
                }
                Commands::Flash {
                    address,
                    erase,
                    no_run,
                    path,
                    enable_sdi_print,
                    watch_serial,
                } => {
                    sess.dump_info()?;

                    if erase {
                        log::info!("Erase Flash");
                        sess.erase_flash()?;
                    }

                    let firmware = read_firmware_from_file(path)?;

                    match firmware {
                        Firmware::Binary(data) => {
                            let start_address =
                                address.unwrap_or_else(|| sess.chip_family.code_flash_start());
                            log::info!("Flashing {} bytes to 0x{:08x}", data.len(), start_address);
                            sess.write_flash(&data, start_address)?;
                        }
                        Firmware::Sections(sections) => {
                            // Flash section by section
                            if address.is_some() {
                                log::warn!("--address is ignored when flashing ELF or ihex");
                            }
                            for section in sections {
                                let start_address =
                                    sess.chip_family.fix_code_flash_start(section.address);
                                log::info!(
                                    "Flashing {} bytes to 0x{:08x}",
                                    section.data.len(),
                                    start_address
                                );
                                sess.write_flash(&section.data, start_address)?;
                            }
                        }
                    }

                    log::info!("Flash done");

                    sleep(Duration::from_millis(500));

                    if !no_run {
                        log::info!("Now reset...");
                        sess.soft_reset()?;
                        if enable_sdi_print {
                            sess.set_sdi_print_enabled(true)?;

                            will_detach = false;
                            log::info!("Now connect to the WCH-Link serial port to read SDI print");
                        }
                        if watch_serial {
                            wlink::probe::watch_serial()?;
                        } else {
                            sleep(Duration::from_millis(500));
                        }
                    }
                }
                Commands::Unprotect {} => {
                    log::info!("Unprotect Flash");
                    sess.unprotect_flash()?;
                }
                Commands::Protect {} => {
                    log::info!("Protect Flash");
                    sess.protect_flash()?;
                }
                Commands::Reset {
                    mode,
                    enable_sdi_print,
                    watch_serial,
                } => {
                    log::info!("Reset {:?}", mode);
                    match mode {
                        ResetMode::Quit => {
                            sess.probe.send_command(commands::Reset::Soft)?;
                        }
                        ResetMode::Run => {
                            sess.ensure_mcu_resume()?;
                        }
                        ResetMode::Halt => {
                            sess.ensure_mcu_halt()?;

                            will_detach = false; // detach will resume the MCU
                        }
                        ResetMode::Dm => {
                            sess.reset_debug_module()?;

                            will_detach = false; // detach will resume the MCU
                        }
                    }
                    sleep(Duration::from_millis(500));
                    if enable_sdi_print {
                        sess.set_sdi_print_enabled(true)?;
                        will_detach = false;
                        log::info!("Now connect to the WCH-Link serial port to read SDI print");
                    }
                    if watch_serial {
                        wlink::probe::watch_serial()?;
                    } else {
                        sleep(Duration::from_millis(300));
                    }
                }
                Commands::Status {} => {
                    sess.dump_info()?;
                    sess.dump_core_csrs()?;
                    sess.dump_dmi()?;
                }
                Commands::SdiPrint(v) => match v {
                    // By enabling SDI print and modifying the _write function called by printf in the mcu code,
                    // the WCH-Link can be used to read data from the debug interface of the mcu
                    // and print it to the serial port of the WCH-Link instead of using its UART peripheral.
                    // An example can be found here:
                    // https://github.com/openwch/ch32v003/tree/main/EVT/EXAM/SDI_Printf/SDI_Printf
                    SdiPrint::Enable => {
                        log::info!("Enabling SDI print");
                        sess.set_sdi_print_enabled(true)?;
                        will_detach = false;
                        log::info!("Now you can connect to the WCH-Link serial port");
                    }
                    SdiPrint::Disable => {
                        log::info!("Disabling SDI print");
                        sess.set_sdi_print_enabled(false)?;
                    }
                },
                _ => unreachable!("unimplemented command"),
            }
            if will_detach {
                sess.detach_chip()?;
            }
        }
    }

    Ok(())
}

/// Whether this invocation would flash to a probe chosen by enumeration order
/// alone, i.e. `flash` without `-d`.
fn flashing_without_explicit_probe(
    device: &Option<DeviceSelector>,
    command: &Option<Commands>,
) -> bool {
    device.is_none() && matches!(command, Some(Commands::Flash { .. }))
}

/// "The first enumerated probe" is ambiguous when more than one probe is
/// attached. Zero or one probe is fine: zero still fails later with a proper
/// "probe not found" error.
fn ensure_unambiguous_flash_probe(choices: &[ProbeChoice]) -> Result<()> {
    if choices.len() > 1 {
        Err(wlink::Error::AmbiguousProbe(choices.to_vec()).into())
    } else {
        Ok(())
    }
}

pub fn parse_number(s: &str) -> std::result::Result<u32, String> {
    let s = s.replace('_', "").to_lowercase();
    if let Some(hex_str) = s.strip_prefix("0x") {
        Ok(
            u32::from_str_radix(hex_str, 16)
                .unwrap_or_else(|_| panic!("error while parsing {s:?}")),
        )
    } else if let Some(bin_str) = s.strip_prefix("0b") {
        Ok(u32::from_str_radix(bin_str, 2).unwrap_or_else(|_| panic!("error while parsing {s:?}")))
    } else {
        Ok(s.parse().expect("must be a number"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn erase_by_power_off_accepts_a_probe_rs_selector() {
        // Regression: `-d` used to be an Option<usize>, so this exact command
        // line was rejected by clap with "invalid digit found in string".
        let cli = Cli::try_parse_from([
            "wlink",
            "-d",
            "1a86:8010:D7708F065C7F",
            "erase",
            "--method",
            "power-off",
            "--chip",
            "CH32H41X",
        ])
        .expect("the reported command line should parse");

        assert_eq!(
            cli.device,
            Some(DeviceSelector::Serial("D7708F065C7F".into()))
        );
        assert_eq!(cli.chip, Some(RiscvChip::CH32H41X));
        assert!(matches!(
            cli.command,
            Some(Commands::Erase {
                method: EraseMode::PowerOff
            })
        ));
    }

    #[test]
    fn device_still_accepts_an_index_and_a_bare_serial() {
        let cli = Cli::try_parse_from(["wlink", "-d", "2", "status"]).unwrap();
        assert_eq!(cli.device, Some(DeviceSelector::Index(2)));

        let cli = Cli::try_parse_from(["wlink", "-d", "D7708F065C7F", "status"]).unwrap();
        assert_eq!(
            cli.device,
            Some(DeviceSelector::Serial("D7708F065C7F".into()))
        );

        let cli = Cli::try_parse_from(["wlink", "status"]).unwrap();
        assert_eq!(cli.device, None);
    }

    #[test]
    fn malformed_device_selector_has_a_useful_error() {
        let err = Cli::try_parse_from(["wlink", "-d", "1a86:8010", "status"])
            .err()
            .expect("an incomplete selector should be rejected");
        assert!(
            err.to_string().contains("expected INDEX or VID:PID:SERIAL"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn only_a_selector_less_flash_needs_the_probe_check() {
        let cli = Cli::try_parse_from(["wlink", "flash", "firmware.bin"]).unwrap();
        assert!(flashing_without_explicit_probe(&cli.device, &cli.command));

        // An explicit index or serial names the probe, so no check is needed.
        for selector in ["0", "1a86:8010:D7708F065C7F"] {
            let cli =
                Cli::try_parse_from(["wlink", "-d", selector, "flash", "firmware.bin"]).unwrap();
            assert!(
                !flashing_without_explicit_probe(&cli.device, &cli.command),
                "{selector} should count as an explicit selection"
            );
        }

        // Only flashing is guarded; other commands keep the old behaviour.
        for args in [
            vec!["wlink", "status"],
            vec!["wlink", "list"],
            vec!["wlink", "erase"],
            vec!["wlink", "mode-switch", "--rv"],
        ] {
            let cli = Cli::try_parse_from(args.clone()).unwrap();
            assert!(
                !flashing_without_explicit_probe(&cli.device, &cli.command),
                "{args:?} should not be guarded"
            );
        }
    }

    #[test]
    fn flashing_error_lists_every_probe_with_a_selector() {
        let choice = |serial: &str, index: usize| ProbeChoice {
            selector: format!("1a86:8010:{serial}"),
            description: format!("WCH-Link#{index} (RV mode)"),
        };

        assert!(ensure_unambiguous_flash_probe(&[]).is_ok());
        assert!(ensure_unambiguous_flash_probe(&[choice("AAA", 0)]).is_ok());

        let choices = vec![
            choice("7E238F0602BE", 0),
            choice("F93D8F067E10", 1),
            choice("D7708F065C7F", 2),
        ];
        let err = ensure_unambiguous_flash_probe(&choices)
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
}
