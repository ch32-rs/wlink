# Command Reference

`CMD` is byte 1 of a request frame (see [Frame Format](frame-format.md)). All
payload integers are big-endian. `Source` links point at
[`src/commands/mod.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/mod.rs)
unless noted otherwise; `0x0d` subcommands live in
[`src/commands/control.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/control.rs)
and are expanded in [Control Subcommands](control-subcommands.md).

## Command ID summary

| `CMD` | Name | Payload | Response | Status |
| --- | --- | --- | --- | --- |
| `0x01` | `SetWriteMemoryRegion` | 8 bytes: address, length | `()` | implemented |
| `0x02` | `Program` | 1 byte: subcommand | `u8` | implemented |
| `0x03` | `SetReadMemoryRegion` | 8 bytes: address, length | `()` | implemented |
| `0x06` | `ConfigChip` | 1–8 bytes: protection/config | `u8` | partially implemented |
| `0x08` | `DmiOp` | 6 bytes | 6 bytes | implemented |
| `0x0b` | `Reset` | 1 byte | `()` | implemented |
| `0x0c` | `SetSpeed` | 2 bytes: `riscvchip`, speed | `bool` | implemented |
| `0x0d` | Control | subcommand + args | varies | implemented — see [Control Subcommands](control-subcommands.md) |
| `0x0e` | `DisableDebug` | 1 byte `0x01` | `()` | implemented |
| `0x0f` | Get device mode | 1 byte | unknown | `unverified` |
| `0x11` | `GetChipInfo` (ESIG) | 1 byte: `0x09` / `0x06` | 20 raw bytes | implemented |
| `0xfe` | Disable encryption | 1 byte | unknown | `unverified` |
| `0xff` | Mode switch | 1 byte `0x41` | — | implemented (RV → DAP) |

The old `protocol.md` listed `0x0c` as `???` and `0x0f` as "begin verify".
Both are wrong: `0x0c` is `SetSpeed` (corrected in CHANGELOG for v0.0.6), and
`0x0f` only appears in a `GetDeviceMode` capture.

---

## `0x01` — SetWriteMemoryRegion

Sets the target address and byte count for the following write operation.

| Offset | Type | Field |
| --- | --- | --- |
| 0 | `u32_be` | `start_addr` (typically `0x08000000` code flash or `0x00000000` SRAM) |
| 4 | `u32_be` | `len` (firmware length in bytes) |

Request: `81 01 08 <addr:4> <len:4>`. Response: `()`.

Source: `SetWriteMemoryRegion`. Used at the start of `ProbeSession::write_flash`.

## `0x02` — Program

One-byte subcommand selector. Response is a single status byte; wlink checks
the returned value for the `0x07` step during flashing.

| Sub | Rust variant | Meaning |
| --- | --- | --- |
| `0x01` | `EraseFlash` | Erase code flash |
| `0x02` | `WriteFlash` | Begin fast-program (bulk data follows on the data endpoint) |
| `0x04` | `WriteFlashAndVerify` | Write flash with verification |
| `0x05` | `WriteFlashOP` | Begin writing the per-family flash algorithm blob |
| `0x06` | `Prepare` | Prepare before setting the RAM address (not used by current flows) |
| `0x07` | `Unknown07AfterFlashOPWritten` | Commit step after the flash blob was written; response must be `0x07` |
| `0x08` | `End` | End program |
| `0x0b` | `Unknown0BAfterFlashOPWritten` | Alternative commit step for `riscvchip == 1` |
| `0x0c` | `ReadMemory` | Start a memory read on the data endpoint |

Older notes listed `0x03` "execute ram", `0x05` "begin bulk transfer", `0x07`
"verify", `0x09` "bulk transfer ends", `0x0a` "verify" and `0x0b` "= verify for
riscvchip 1". Those labels are **unverified**; see
[Unverified Commands](unverified.md).

## `0x03` — SetReadMemoryRegion

Identical layout to `0x01`: `start_addr` and `len`, both `u32_be`. Response `()`.
Followed by `0x02 / 0x0c` (`ReadMemory`) and a bulk read on endpoint `0x82`.

Source: `SetReadMemoryRegion`, `ProbeSession::read_memory`.

## `0x06` — ConfigChip

Flash read/write protection and option-byte configuration. Response is a status
`u8` for the query variants.

| PAYLOAD[0] | Rust variant | Payload | Meaning / response |
| --- | --- | --- | --- |
| `0x01` | `CheckReadProtect` | 1 byte | `0x01` protected, `0x02` unprotected |
| `0x02` | `Unprotect` | 1 byte | Remove read protection (mass-erases option bytes) |
| `0x03` | `Protect` | 1 byte | Set read protection |
| `0x04` | `CheckReadProtectEx` | 1 byte | `0x11` write-protected, `0x00` unprotected |
| `0x02` + arg | `UnprotectEx(b)` | 8 bytes: `02 b ff ff ff ff ff ff` | Extended unprotect; `b` is `0xff` or `0xbf` |
| `0x03` + arg | `ProtectEx(b)` | 8 bytes: `03 b ff ff ff ff ff ff` | Extended protect |
| — | `Config { data, wrp }` | 8 bytes | **Unimplemented** in wlink (`todo!()`); capture: `81 06 08 02 3f 00 00 ff ff ff ff` |

Flags (`ConfigChip` associated constants): `FLAG_READ_PROTECTED = 0x01`,
`FLAG_READ_UNPROTECTED = 0x02`, `FLAG_WRITE_PROTECTED = 0x11`,
`FLAG_WRITE_UNPROTECTED = 0x00`.

> Changing protection requires a fresh `0x0d / 0x02` attach afterwards. wlink
> skips `Unprotect` when flash is already unprotected, because the probe
> firmware mass-erases the option-byte page (USER/Data/WRPR).

## `0x08` — DmiOp

Six-byte payload, six-byte response. See [DMI and the Debug Module](dmi.md) for
register addresses and status handling.

| Offset | Size | Request field | Response field |
| --- | --- | --- | --- |
| 0 | 1 | `addr` (DMI register) | `addr` |
| 1–4 | 4 | `data` (`u32_be`) | `data` (`u32_be`) |
| 5 | 1 | `op`: `0` NOP, `1` READ, `2` WRITE | `op`: `0x00` success, `0x02` failed, `0x03` busy |

Example — read `DMSTATUS` (`0x11`): `81 08 06 11 00 00 00 00 01`.

## `0x0b` — Reset

| PAYLOAD[0] | Rust variant | Meaning |
| --- | --- | --- |
| `0x01` | `Reset::Soft` | Quit reset / reset and run (most common); wlink waits ~300 ms afterwards |
| `0x02` | `Reset::Chip` | Chip reset (memory is not reset) |
| `0x03` | `Reset::Normal` | Normal reset |

Response: `()`. Source: `Reset`, `ProbeSession::soft_reset`.

## `0x0c` — SetSpeed

| Offset | Field | Values |
| --- | --- | --- |
| 0 | `riscvchip` | Chip family code, see [Constants](constants.md) |
| 1 | `speed` | `0x01` High (6000 kHz, default), `0x02` Medium (4000 kHz), `0x03` Low (400 kHz) |

Response payload `0x01` means success (`Response for bool`). wlink sends this
before attaching and again after auto-detection. Source: `SetSpeed`, `Speed`.

## `0x0d` — Control

The multi-purpose control command. Its first payload byte selects a
subcommand; the remainder depends on it. See
[Control Subcommands](control-subcommands.md) for the complete table.

Quick reference: `0x01` probe info, `0x02` attach chip, `0x04`/`0x05` ROM/RAM
split, `0x08` erase by RST pin, `0x09`–`0x0c` power output, `0x0e` RST pin,
`0x0f` erase by power-off, `0xee` SDI print, `0xff` detach.

## `0x0e` — DisableDebug

Payload `0x01`, response `()`. Only used for specific `riscvchip` families
(older notes mention families 2 and 3). Frame: `81 0e 01 01`.

Source: `DisableDebug`.

## `0x11` — GetChipInfo / ESignature

| PAYLOAD[0] | Rust variant | Notes |
| --- | --- | --- |
| `0x09` | `GetChipInfo::V1` | Used with firmware older than v2.9 |
| `0x06` | `GetChipInfo::V2` | Introduced in firmware v2.9 |

This command **does not use the standard response frame** — see
[Chip Info and ESIG](chip-info.md) for the raw 20-byte layout. Source:
`GetChipInfo`, `ESignature`.

## `0xff` — Mode switch

`RawCommand<0xff>` with payload `0x41` (`81 ff 01 41`) switches an RV-mode
probe into DAP mode. The reverse direction writes `81 ff 01 52` raw to endpoint
`0x02` while the probe is in DAP mode. See [USB Transport](usb-transport.md).
