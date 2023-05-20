# wlink - WCH-Link command line tool

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

**NOTE**: This tool is still in development and not ready for production use.

**Known issues**:

- Only support binary firmware file

## Tested On

### Probes

- WCH-Link [CH549]
- WCH-LinkE [CH32V305][CH32V307]

[CH549]: http://www.wch-ic.com/products/CH549.html

### MCU

- [CH32V003] - flash ok, reset not work
- [CH32V103] - flash ok
- [CH32V203] - flash & reset ok
- [CH32V307] - flash & reset ok
- ... (Feel free to open an issue if you have tested on other chips)

[CH32V003]: http://www.wch-ic.com/products/CH32V003.html
[CH32V103]: http://www.wch-ic.com/products/CH32V103.html
[CH32V203]: http://www.wch-ic.com/products/CH32V203.html
[CH32V307]: http://www.wch-ic.com/products/CH32V307.html

## Install

`cargo install --git` or download a binary from the [Nightly Release page](https://github.com/ch32-rs/wlink/releases/tag/nightly).

## Usage

```console
> # Flash firmware.bin to Code FLASH at address 0x08000000
> cargo run -- flash 0x08000000 ./firmware.bin`
12:10:26 [INFO] WCH-Link v2.8 (WCH-Link-CH549)
12:10:26 [INFO] Attached chip: CH32V30x(0x30700518)
12:10:26 [INFO] Flashing 8068 bytes to 0x08000000
12:10:27 [INFO] Flash done
12:10:28 [INFO] Now reset...
12:10:28 [INFO] Resume executing...

> # Dump Code FLASH, for verification
> cargo run -- -v dump 0x08000000 100`
18:31:18 [DEBUG] (1) wlink::device: Acquired libusb context.
18:31:18 [DEBUG] (1) wlink::device: Claimed interface 0 of USB device.
18:31:18 [INFO] WCH-Link v2.8 (WCH-LinkE-CH32V305)
18:31:18 [DEBUG] (1) wlink::operations: attached chip: ChipInfo { chip_family: CH32V20x, chip_type: "0x20360510" }
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
> cargo run -- dump 0x1FFF8000 0x7000
....

> # Dump all general purpose registers
> cargo run -- regs
16:24:20 [INFO] Dump GPRs
dpc(pc):   0x2000011a
x0   zero: 0x00000000
x1     ra: 0x49c85c07
x2     sp: 0x20002800
x3     gp: 0x206e24c4
x4     tp: 0x9add07a3
x5     t0: 0xb4a9b38a
....


> # Set dpc(pc) to System Flash and Run - Not working :(
> cargo run -- write-reg 0x7b1 0x1fff8000

````

## References

- <https://github.com/openwch/ch32v003> RISC-V QingKeV2 Microprocessor Debug Manual
- <https://github.com/cnlohr/ch32v003fun> A miniwchlink implementation
- <https://github.com/blackmagic-debug/blackmagic/pull/1399>
