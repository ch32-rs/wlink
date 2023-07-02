# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Support WCH-LinkW, a CH32V208 flasher with wireless connection
- Support WCH-Link firmware 2.9, some raw commands are changed
- Support Flash protect and unprotect (#14)

### Fixed

- Constraint regs for riscv32ec variant

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
