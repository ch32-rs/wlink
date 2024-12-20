# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2024-11-15

### Fixed

- Fix ELF firmware parsing error, #77

## [0.1.0] - 2024-11-11

### Added

- Support CH585/CH584, CH32V317, CH32V002/4/5/6/7, CH32M007, #76

## [0.0.9] - 2024-10-03

### Added

- Add functions to control 3.3V and 5V outputs of probe, #72

### Fixed

- Rename ChipUID response to ESignature, #58
- Ensure consistent nanosecond precision in log timestamps, #64

## [0.0.8] - 2024-03-30

### Added

- Add `--watch-serial` for `flash` subcommand, #36
- Add `-o/--out` for `dump` sumcommand, #38
- BREAKING CHANGE: Refactor
- Add Windows native driver support, #39
- Use loaded memory address from ELF file or ihex file
- Add timestamp in serial output

### Fixed

- Merge gaps in firmware sections, #56

### Changed

- No erase by default when flashing

## [0.0.7] - 2023-11-04

### Added

- Dump CSRs using `regs` subcommand
- Show firmware size info when flashing
- Add a progress bar when flashing
- New chip: CH641, a RV32EC chip almost the same as CH32V003
- SDI print support, #34
- Add serial number field to probe

### Fixed

- Bypass attach chip when using special erase in #32

## [0.0.6] - 2023-10-07

### Added

- Add `--speed` option to specify protocol speed
- Add `erase --method power-off` option to support erase with power off
- Add `erase --method pin-rst` option to support erase with RST pin, close #26
- Add a simple chip ID db, now wlink can identify chip type automatically

### Fixed

- Regression in `flash` command
- Use chip type from the protocol, close #25
- Support 2.10 (aka. v30) firmware, close #27
- Fix `--no-detach` option is not working
- Use DMI to read memory, avoid using probe commands

### Changed

- Allow underscore `_` in number literals in command line
- Refine protocol naming
- Add a simple DMI algorithm skeleton

## [0.0.5] - 2023-07-31

### Added

- Support WCH-LinkW, a CH32V208 flasher with wireless connection
- Support WCH-Link firmware 2.9, some raw commands are changed
- Support Flash protect and unprotect (#14)
- Fix stuck for CH5xx devices, due to unsppported read ram rom split command
- Add `--chip` option to specify chip type
- Check probe type when doing mode-switch
- Add support for CH32X035
- Add support for CH59X

### Fixed

- Constraint regs for riscv32ec variant
- Wrong 0x0c command interpretation, this should be a set chip speed command
- Cannot flash CH32V003 (#23). Now wlink won't get info when attaching chip

### Changed

- Refine error messages
- `--address` for flash is now optional, default to device flash start address

## [0.0.4] - 2023-07-01

### Added

- Add `mode-switch` subcommand to switch between RV mode and DAP mode (#3)
- Add `hex`, `ihex` and `elf` format support for `flash` subcommand

### Fixed

- Fix communication parity error of abstractcs register (#16)
- Do not halt when read register

### Changed

- Refine attach chip logic, more robust now
- Refine docs

## [0.0.3] - 2023-03-01

### Added

- Everything just works
