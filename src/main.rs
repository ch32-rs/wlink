use std::{thread::sleep, time::Duration};

use anyhow::Result;
use wlink::{
    commands,
    dmi::DebugModuleInterface,
    firmware::{read_firmware_from_file, Firmware},
    operations::ProbeSession,
    probe::WchLink,
    regs, RiscvChip,
};

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional device index to operate on
    #[arg(long, short = 'd', value_name = "INDEX")]
    device: Option<usize>,

    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,

    /// Detach chip after operation
    #[arg(long, global = true, default_value = "false")]
    no_detach: bool,

    /// Specify the chip type, e.g. CH32V30X
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
    /// Enable 3.3V output
    Enable3V3 {},
    /// Disable 3.3V output
    Disable3V3 {},
    /// Enable 5V output
    Enable5V {},
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
    simplelog::TermLogger::init(
        cli.verbose.log_level_filter(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("initialize simple logger");

    let device_index = cli.device.unwrap_or(0);
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
                WchLink::switch_from_rv_to_dap(device_index)?;
            } else {
                WchLink::switch_from_dap_to_rv(device_index)?;
            }
        }
        Some(Commands::List {}) => {
            WchLink::list_probes()?;
        }

        Some(Commands::Erase { method }) if method != EraseMode::Default => {
            // Special handling for non-default erase: bypass attach chip
            // So a chip family info is required, no detection
            let chip_family = cli.chip.ok_or(wlink::Error::Custom(
                "--chip required to do a special erase".into(),
            ))?;

            let mut probe = WchLink::open_nth(device_index)?;
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
            let probe = WchLink::open_nth(device_index)?;
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
                Commands::Reset { mode } => {
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
                    sleep(Duration::from_millis(300));
                }
                Commands::Status {} => {
                    sess.dump_info()?;
                    sess.dump_core_csrs()?;
                    sess.dump_dmi()?;
                }               
                Commands::Enable3V3 {} => {
                    log::info!("Enable 3.3V Output");
                    sess.set_3v3_output_enabled(true)?;
                }
                Commands::Disable3V3 {} => {
                    log::info!("Disable 3.3V Output");
                    sess.set_3v3_output_enabled(false)?;
                }
                Commands::Enable5V {} => {
                    log::info!("Enable 5V Output");
                    sess.set_5v_output_enabled(true)?;
                }
                Commands::Disable5V {} => {
                    log::info!("Disable 5V Output");
                    sess.set_5v_output_enabled(false)?;
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
