# Unverified Commands and Captures

This page collects protocol knowledge that wlink does **not** implement and
cannot confirm. It exists so that the old `protocol.md`'s `???` markers are not
silently dropped: they are either resolved (and documented on other pages) or
listed here as observations.

> `unverified` means: seen in a USB capture from WCH's utility, or written in
> an older note, without a confirmed implementation in wlink. Do not rely on
> these byte sequences in new code without re-capturing.

## Raw captures recorded in the source

Copied verbatim from the comment block at the end of
[`src/commands/mod.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/mod.rs):

```text
81 0D 05 11 SetAccessAddress
81 0F 01 02 GetDeviceMode
81 0D 01 07 EnableQE
81 0D 01 06 CheckQE
81 FE 01 00 DisEncrypt
81 0D 01 0F ClearCodeFlashB
81 0D 02 08 xx ClearCodeFlash
81 11 01 0D unknown in query info, before GetChipRomRamSplit
81 0D 02 EE 00/02/03 SetSDLineMode
81 0F 01 01 SetIAPMode
```

Observations:

- `0x0f` appears as a command ID in `GetDeviceMode` / `SetIAPMode` captures,
  not as a "begin verify" `0x02` subcommand as the old document claimed.
- `0xfe` appears once as `DisEncrypt`.
- `81 0D 01 0F` (`ClearCodeFlashB`) has `LEN = 1` while two payload bytes
  follow — see the discussion of the analogous `0x0f` length quirk in
  [Control Subcommands](control-subcommands.md).
- `81 11 01 0D` is a `0x11` variant not used by wlink, which only sends `0x09`
  (V1) and `0x06` (V2).
- `81 0D 02 EE 00/02/03` suggests additional SDI line modes; wlink only uses
  `ee 00` / `ee 01`.

## Subcommands not implemented by wlink

| `CMD / SUB` | Capture label | Status |
| --- | --- | --- |
| `0x0d / 0x03` | "stage after connect chip" (old note, `riscvchip 1`) | `unverified` |
| `0x0d / 0x06` | `CheckQE` | `unverified` |
| `0x0d / 0x07` | `EnableQE` | `unverified` |
| `0x0d / 0x11` | `SetAccessAddress` | `unverified` |
| `0x0f / 0x01` | `SetIAPMode` | `unverified` |
| `0x0f / 0x02` | `GetDeviceMode` | `unverified` |
| `0xfe / 0x00` | `DisEncrypt` | `unverified` |
| `0x06 Config { data, wrp }` | option-byte config flags | known capture `81 06 08 02 3f 00 00 ff ff ff ff`, but `todo!()` in wlink |

## Old `protocol.md` questions and their resolution

| Old entry | Resolution |
| --- | --- |
| `0x01 - Set RAM address and size` (empty section) | `SetWriteMemoryRegion`, 8-byte payload — [Command Reference](commands.md) |
| `0x02` subcommand guesses (`0x03 execute ram`, `0x05 begin bulk transfer`, `0x07 verify`, `0x09 bulk transfer ends`, `0x0a verify`, `0x0b verify for riscvchip 1`) | Replaced by the implemented `Program` enum; unconfirmed labels are `unverified` |
| `0x08 - DMI OP` (two empty tables) | Filled in — [DMI](dmi.md) |
| `0x0c - ???` | `SetSpeed`; see also the v0.0.6 CHANGELOG correction |
| `0x0d - Info` in the command table vs `0x0d - Control` as a heading | Both refer to the **Control** command — [Control Subcommands](control-subcommands.md) |
| `0x0e` (unnamed) | `DisableDebug` |
| `0x0f - ? begin verify` | `unverified`; captures point at `GetDeviceMode` / `SetIAPMode` |
| `0x03 Reset, normal?` | `Reset::Normal` (`0x03`) |
| `0x03 ? stage after connect chip` | Still `unverified` |
| `Error Reason: 0x55 only` | Still the only reason with a known meaning |
| `RiscvChip` with 7 families | 28 entries — [Constants](constants.md) |
| `Firmware Versions: v2.8` | Current baseline v2.15 (v35); encoding explained in [Probe Info](probe-info.md) |
| `Variants: ?? / WCH-LinkB` | Still `unverified` |
| `beigin`, `buck`, `Swith` typos | Corrected |

## Contributing a correction

1. Capture the exchange with `wlink -vv` (trace logs print `send` and `recv`
   hex for every frame) or with a USB sniffer.
2. Identify the Rust symbol in `src/commands/` that produces or parses it.
3. Update the relevant page here, and move the entry out of this page if it is
   now confirmed. If the implementation changes, update the code and the
   documentation in the same PR.
