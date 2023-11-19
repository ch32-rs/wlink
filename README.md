# wlink - WCH-Link(RV) command line tool

[![Crates.io][badge-license]][crates]
[![Crates.io][badge-version]][crates]
[![docs.rs][badge-docsrs]][docsrs]
[![GitHub release][badge-release]][nightly]

[badge-license]: https://img.shields.io/crates/l/wlink?style=for-the-badge
[badge-version]: https://img.shields.io/crates/v/wlink?style=for-the-badge
[badge-docsrs]: https://img.shields.io/docsrs/wlink?style=for-the-badge
[badge-release]: https://img.shields.io/github/v/release/ch32-rs/wlink?include_prereleases&style=for-the-badge
[crates]: https://crates.io/crates/wlink
[docsrs]: https://docs.rs/wlink
[nightly]: https://github.com/ch32-rs/wlink/releases/tag/nightly

> **Note**
> This tool is still in development and not ready for production use.

## Feature Support

- [x] Flash firmware, support Intel HEX, ELF and raw binary format
- [x] Erase chip
- [x] Halt, resume, reset support
- [x] Read chip info
- [x] Read chip memory(flash)
- [x] Read/write chip register - very handy for debugging
- [x] Code-Protect & Code-Unprotect for supported chips
- [x] [SDI print](https://www.cnblogs.com/liaigu/p/17628184.html) support, requires 2.10+ firmware
- [x] [Serial port watching](https://github.com/ch32-rs/wlink/pull/36) for a smooth development experience

## Tested On

### Probes

Current firmware version: 2.11 (aka. v31).

> **NOTE**: The firmware version is not the same as the version shown by WCH's toolchain. Because WCH calculates the version number by `major * 10 + minor`, so the firmware version 2.10 is actually v30 `0x020a`.

- WCH-Link [CH549] - the first version, reflash required when switching mode
- WCH-LinkE [CH32V305][CH32V307] - the recommended debug probe
- WCH-LinkW [CH32V208][CH32V208] - wireless version
- WCH-Link? [CH32V203][CH32V203]

[CH549]: https://www.wch-ic.com/products/CH549.html

### MCU

- [CH32V003]
- [CH32V103]
- [CH32V203]
- [CH32V307]
- [CH569]/CH565
- [CH573]/CH571
- [CH583]/CH582/CH581
- [CH592]/CH591
- [ ] [CH643] - I don't have this chip, help wanted
- [ ] [CH641] - I don't have this chip, help wanted
- [CH32X035]
- [ ] [CH32L103] - I don't have this chip, help wanted
- [ ] [CH8571] - No other source about this chip, help wanted
- ... (Feel free to open an issue if you have tested on other chips)

[CH32V003]: https://www.wch-ic.com/products/CH32V003.html
[CH32V103]: https://www.wch-ic.com/products/CH32V103.html
[CH32V203]: https://www.wch-ic.com/products/CH32V203.html
[CH32V208]: https://www.wch-ic.com/products/CH32V208.html
[CH32V307]: https://www.wch-ic.com/products/CH32V307.html
[CH32X035]: https://www.wch-ic.com/products/CH32X035.html
[CH32L103]: https://www.wch-ic.com/products/CH32L103.html
[CH569]: https://www.wch-ic.com/products/CH569.html
[CH573]: https://www.wch-ic.com/products/CH573.html
[CH583]: https://www.wch-ic.com/products/CH583.html
[CH592]: https://www.wch-ic.com/products/CH592.html
[CH641]: https://www.wch.cn/downloads/CH641DS0_PDF.html
[CH643]: https://www.wch-ic.com/products/CH643.html
[CH8571]: https://www.wch.cn/news/606.html

## Install

`cargo install --git https://github.com/ch32-rs/wlink` or download a binary from the [Nightly Release page](https://github.com/ch32-rs/wlink/releases/tag/nightly).

> **Note**
> On Linux, you should install libudev and libusb development lib first.
> Like `sudo apt install libudev-dev libusb-1.0-0-dev` on Ubuntu.

## Usage

> **Note**
> For help of wire connection for specific chips, please refer to `docs` subdirectory.

```console
> # Flash firmware.bin to Code FLASH at address 0x08000000
> wlink flash --address 0x08000000 ./firmware.bin
12:10:26 [INFO] WCH-Link v2.10 (WCH-Link-CH549)
12:10:26 [INFO] Attached chip: CH32V30X(0x30700518)
12:10:26 [INFO] Flashing 8068 bytes to 0x08000000
12:10:27 [INFO] Flash done
12:10:28 [INFO] Now reset...
12:10:28 [INFO] Resume executing...

> # Flash firmware.bin to System FLASH, enable SDI print, then watch serial port
> wlink flash --enable-sdi-print --watch-serial firmware.bin
02:54:34 [INFO] WCH-Link v2.11 (WCH-LinkE-CH32V305)
02:54:34 [INFO] Attached chip: CH32V003 [CH32V003F4P6] (ChipID: 0x00300500)
02:54:34 [INFO] Flash already unprotected
02:54:34 [INFO] Flash protected: false
02:54:35 [INFO] Flash done
02:54:35 [INFO] Now reset...
02:54:35 [INFO] Now connect to the WCH-Link serial port to read SDI print
Hello world from ch32v003 SDI print!
led toggle
led toggle
...


> # Dump Code FLASH, for verification
> # use `-v` or `-vv` for more logs
> wlink -v dump 0x08000000 100
18:31:18 [DEBUG] (1) wlink::device: Acquired libusb context.
18:31:18 [DEBUG] (1) wlink::device: Claimed interface 0 of USB device.
18:31:18 [INFO] WCH-Link v2.8 (WCH-LinkE-CH32V305)
18:31:18 [DEBUG] (1) wlink::operations: attached chip: ChipInfo { chip_family: CH32V20X, chip_type: "0x20360510" }
18:31:18 [DEBUG] (1) wlink::operations: Chip UID: cd-ab-b4-ae-45-bc-c6-16
18:31:18 [DEBUG] (1) wlink::operations: flash protected: false
18:31:18 [DEBUG] (1) wlink::operations: SRAM CODE mode: 3
18:31:18 [DEBUG] (1) wlink::operations: RISC-V core version: Some("WCH-V4B")
18:31:18 [INFO] Read memory from 0x08000000 to 0x08000064
08000000:   b7 00 00 08  67 80 80 00  73 50 40 30  73 50 40 34   ×00•g××0sP@0sP@4
08000010:   81 40 01 41  81 41 01 42  81 42 01 43  81 43 01 44   ×@•A×A•B×B•C×C•D
08000020:   81 44 81 46  01 47 81 47  01 48 81 48  01 49 81 49   ×D×F•G×G•H×H•I×I
08000030:   01 4a 81 4a  01 4b 81 4b  01 4c 81 4c  01 4d 81 4d   •J×J•K×K•L×L•M×M
08000040:   01 4e 81 4e  01 4f 81 4f  97 01 00 18  93 81 81 7b   •N×N•O×O×•0•×××{
08000050:   f3 23 40 f1  b7 02 00 00  93 82 02 00  63 f4 72 00   ×#@××•00××•0c×r0
08000060:   6f 00 c0 29                                          o0×)


> # Dump System FLASH, BOOT_28KB
> wlink dump 0x1FFF8000 0x7000
....


> # Dump all general purpose registers
> wlink regs
16:24:20 [INFO] Dump GPRs
dpc(pc):   0x2000011a
x0   zero: 0x00000000
x1     ra: 0x49c85c07
x2     sp: 0x20002800
x3     gp: 0x206e24c4
x4     tp: 0x9add07a3
x5     t0: 0xb4a9b38a
....


> # Set dpc(pc) to System Flash
> wlink write-reg 0x7b1 0x000009a8
```

## References

- [docs/references.md](docs/references.md)
- WCH's openocd fork: <https://github.com/treideme/openocd-hacks>
