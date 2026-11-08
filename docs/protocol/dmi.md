# DMI and the Debug Module

The Debug Module Interface (DMI) is a separate layer above the WCH-Link command
protocol: a single `0x08` command carries one DMI transaction. The register
addresses and semantics come from the RISC-V Debug Specification 0.13.2 and
WCH's QingKe debug manuals.

Sources: `DmiOp` / `DmiOpResponse` in
[`src/commands/mod.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/mod.rs),
[`src/dmi.rs`](https://github.com/ch32-rs/wlink/blob/main/src/dmi.rs),
[`src/regs.rs`](https://github.com/ch32-rs/wlink/blob/main/src/regs.rs).

## `0x08` transaction layout

Request payload (6 bytes):

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 1 | `addr` — DMI register address |
| 1–4 | 4 | `data` (`u32_be`), ignored for NOP/READ |
| 5 | 1 | `op` — `0x00` NOP, `0x01` READ, `0x02` WRITE |

Response payload uses the same 6-byte shape; `op` is now a status:

| `op` | Meaning | `DmiOpResponse` helper |
| --- | --- | --- |
| `0x00` | Success | `is_success()` |
| `0x02` | Failed | `is_failed()` |
| `0x03` | Busy / failed | `is_busy()`, also counted by `is_failed()` |

Example — read `DMSTATUS` (`0x11`):

```text
→ 81 08 06 11 00 00 00 00 01
← 82 08 06 11 00 00 00 00 00
```

`dmi_read` polls on busy (up to 100 retries with 10 ms sleeps) and raises
`Error::NotAttached` when it sees `op == 0x03 && data == 0xffffffff &&
addr == 0x7d`.

## DMI register addresses

From `src/regs.rs`:

| Address | Name | Notes |
| --- | --- | --- |
| `0x04` | `DMDATA0` | Abstract command data |
| `0x05` | `DMDATA1` | |
| `0x10` | `DMCONTROL` | Halt/resume, `dmactive` |
| `0x11` | `DMSTATUS` | `allhalted`, `anyhalted`, `havereset` |
| `0x12` | `DMHARTINFO` | |
| `0x16` | `DMABSTRACTCS` | `cmderr`, `busy` |
| `0x17` | `DMCOMMAND` | Abstract command register |
| `0x18` | `DMABSTRACTAUTO` | Not available on QingKeV2 / QingKeV3 |
| `0x20`–`0x27` | `DMPROGBUF0`–`7` | Program buffer |
| `0x40` | `DMHALTSUM0` | |

CSR addresses used with abstract commands (16-bit `command.regno`):
`0x300 MSTATUS`, `0x301 MISA`, `0x305 MTVEC`, `0x340 MSCRATCH`,
`0x341 MEPC`, `0x342 MCAUSE`, `0x343 MTVAL`, `0x7b1 DPC`,
`0xf12 MARCHID`, `0xf13 MIMPID`. General-purpose registers `x0`–`x31`
occupy `0x1000`–`0x101f`.

## Abstract command errors

`AbstractcsCmdErr::try_from_cmderr` maps the `cmderr` field of
`DMABSTRACTCS`:

| Value | Meaning |
| --- | --- |
| 0 | No error |
| 1 | Busy (register access during an abstract command) |
| 2 | Command not supported |
| 3 | Exception during execution |
| 4 | Hart in wrong state (not halted/running) |
| 5 | Bus error (alignment, size, timeout) |
| 6 | Parity error (WCH extension) |
| 7 | Other |

`ProbeSession::clear_abstractcs_cmderr` writes `0b111` to the `cmderr` field
to clear it (W1C), and `clear_dmstatus_havereset` acknowledges `havereset`.

## Flash/option unlock keys

`src/dmi.rs` defines two keys used when unlocking flash/option-byte writes
through the probe's debug interface:

```text
KEY1 = 0x45670123
KEY2 = 0xCDEF89AB
```

## Higher-level operations

`DebugModuleInterface` (implemented for `WchLink`) provides `dmi_nop`,
`dmi_read`, `dmi_write` and typed register helpers. `ProbeSession` builds on
them for:

- `ensure_mcu_halt` / `ensure_mcu_resume` — write `0x80000001` to `DMCONTROL`
  (`0x10`) and poll `DMSTATUS`;
- register and memory inspection via abstract commands and the program buffer;
- `dump_regs`, `dump_core_csrs`, `dump_pmp_csrs`, `dump_dmi`.

> The RISC-V Debug Specification details are out of scope for this project
> documentation. See [External References](../references.md) for the QingKeV2
> debug manual and other implementations.
