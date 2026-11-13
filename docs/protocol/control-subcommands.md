# Control Subcommands (`0x0d`)

`0x0d` is a multiplexed command: `PAYLOAD[0]` selects an operation and the
remaining bytes are its arguments. Its Rust implementation is
[`src/commands/control.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/control.rs).

Frame shape: `81 0d LEN SUB [ARGS…]`.

## Complete subcommand table

| `SUB` | Rust symbol | Request payload | Response | Notes |
| --- | --- | --- | --- | --- |
| `0x01` | `GetProbeInfo` | `01` | ≥3 bytes | Firmware version and variant; see [Probe Info](probe-info.md) |
| `0x02` | `AttachChip` | `02` | 5 bytes | Auto-detect chip family + ChipID; see [Chip Info](chip-info.md) |
| `0x03` | — | — | — | `unverified`, seen in older notes as a post-attach stage for `riscvchip 1` |
| `0x04` | `GetChipRomRamSplit` | `04` | `u8` | SRAM/CODE split mode `0..3`; CH32V2/V3, CH56X |
| `0x05` | `SetChipRomRamSplit(mode)` | `05 mode` | `()` | Set split mode `0..3` |
| `0x06` | — | `06` | — | `unverified`, capture labels it `CheckQE` |
| `0x07` | — | `07` | — | `unverified`, capture labels it `EnableQE` |
| `0x08` | `EraseCodeFlash::ByPinRST` | `08 riscvchip` | `()` | Erase via the nRST line; requires an RST pin connection |
| `0x09` | `SetPower::Enable3v3` | `09` | `()` | Enable 3.3 V output |
| `0x0a` | `SetPower::Disable3v3` | `0a` | `()` | Disable 3.3 V output |
| `0x0b` | `SetPower::Enable5v` | `0b` | `()` | Enable 5 V output |
| `0x0c` | `SetPower::Disable5v` | `0c` | `()` | Disable 5 V output |
| `0x0e` | `SetRSTPin` | `0e 13` / `0e 14` / `0e 15` | `()` | Drive RST low / high / floating |
| `0x0f` | `EraseCodeFlash::ByPowerOff` | `0f riscvchip` | `()` | Power-cycle erase; see the length caveat below |
| `0x11` | — | `11 …` | — | `unverified`, capture labels it `SetAccessAddress` |
| `0xee` | `SetSdiPrintEnabled` | `ee 00` (on) / `ee 01` (off) | `u8` | `0x00` success, `0xff` unsupported |
| `0xff` | `OptEnd` | `ff` | `()` | Detach / end the session |

`0x09`–`0x0c` require a probe with power functions (`WchLinkVariant::ECh32v305`
or `WCh32v208`). `0xee` requires WCH-LinkE with firmware ≥ 2.10.

## Length caveat for `0x0f`

The captured frame from WCH's utility is:

```text
81 0d 01 0f 09
   ^^ ^^ ^^ ^^
   │  │  │  └─ chip family (0x09 = CH32V003)
   │  │  └──── SUB = 0x0f
   │  └─────── LEN = 1   ← inconsistent: two payload bytes follow
   └────────── CMD = 0x0d
```

`LEN` is wrong in the capture. wlink assembles the frame from its own
`to_raw`, producing `81 0d 02 0f 09` with the correct length. Treat the
captured length byte as a firmware quirk, not as the protocol rule.

## Special-erase operations

`0x08` and `0x0f` bypass chip attach entirely: the probe performs the erase
without a debug connection, which is useful when read protection or a broken
auto-detect prevents attaching. Because no detection happens, the caller must
supply the chip family:

```bash
wlink erase --method power-off --chip CH32H41X
wlink erase --method pin-rst   --chip CH32V003
```

The `riscvchip` argument is the one-byte family code from
[Constants](constants.md).

## SDI print

`0xee` enables the probe's virtual serial port used by WCH's SDI print
feature. Captures also show `81 0d 02 ee 00` / `02` / `03` labelled
`SetSDLineMode`; this three-value form is `unverified` and not implemented by
wlink. wlink only uses `ee 00` (enable) and `ee 01` (disable).

## Session lifecycle

A typical session opens with `0x01` (probe info), then `0x02` (attach), runs
operations, and ends with `0xff` (`OptEnd`). wlink sends `OptEnd` on detach
unless `--no-detach` is given; commands that leave the MCU halted deliberately
skip detach so the halt is preserved.
