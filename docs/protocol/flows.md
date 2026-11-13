# Operation Flows

This page describes the order in which wlink issues commands. Implementations
live in [`src/operations.rs`](https://github.com/ch32-rs/wlink/blob/main/src/operations.rs)
and [`src/probe.rs`](https://github.com/ch32-rs/wlink/blob/main/src/probe.rs).
Command byte layouts are in [Command Reference](commands.md).

## Attach (start of a session)

`ProbeSession::attach`:

1. Check that the probe variant supports the requested chip. Without `--chip`,
   `CH32V103` (`0x01`) is used as the provisional family.
2. **Loop** (up to 3 retries, 100 ms apart):
   - `0x0c SetSpeed` with the provisional `riscvchip` and the selected speed;
   - `0x0d / 0x02 AttachChip` — break on success.
3. If `--chip` was given, verify the detected family matches; otherwise issue
   `0x0c SetSpeed` again using the detected family.
4. Run the family-specific post-init hook (`RiscvChip::do_post_init`).

```text
81 0c 02 01 01      SetSpeed(riscvchip=0x01, High)
81 0d 01 02          AttachChip
82 0d 05 05 20 36 05 10   → CH32V20X, ChipID 0x20360510
81 0c 02 05 01      SetSpeed again with the detected family
```

The CH32V205 family requires firmware v3.6 (`version_code >= 42`); older
firmware misdetects it as CH32V20X and wlink warns.

## Status / chip info

`ProbeSession::dump_info`:

1. `0x11 / 0x06 GetChipInfo::V2` when `version_code >= 29`, else
   `0x11 / 0x09 GetChipInfo::V1` — see [Chip Info](chip-info.md).
2. `0x06 / 0x01 CheckReadProtect` → report whether flash is read-protected.
3. If the family supports it, `0x0d / 0x04 GetChipRomRamSplit` → SRAM/CODE split mode.

> Querying info halts the MCU. wlink re-enables execution on detach.

## Erase (default method)

`ProbeSession::erase_flash`:

1. If the family supports flash protection:
   - `0x06 / 0x01 CheckReadProtect`; protected → `unprotect_flash()`;
   - if unprotected, `0x06 / 0x04 CheckReadProtectEx`; write-protected → `unprotect_flash()`.
2. `0x02 / 0x01 Program::EraseFlash`.
3. `0x0d / 0x02 AttachChip` again to re-establish the session.

### Unprotect

`ProbeSession::unprotect_flash` starts with a fresh attach
(`0x0d / 0xff OptEnd` then `0x0d / 0x02`), then:

1. `0x06 / 0x01 CheckReadProtect`; if `0x01`, `0x06 / 0x02 Unprotect`, reattach, re-check.
2. `0x06 / 0x04 CheckReadProtectEx`; if `0x11`, `0x06 02 ff ff ff ff ff ff ff`
   (`UnprotectEx(0xff)`), reattach, re-check.

wlink deliberately skips `Unprotect` when flash is already unprotected: the
probe firmware mass-erases the option-byte page, destroying USER/Data/WRPR.

## Flash programming

`ProbeSession::write_flash(data, address)`:

```text
 1  (optional) unprotect_flash()
 2  0x01 SetWriteMemoryRegion(address, len)
 3  0x02 / 0x05 Program::WriteFlashOP
 4  data EP 0x02: write the family's flash algorithm blob
                  (padded to data_packet_size, see src/flash_op.rs)
 5  0x02 / 0x07 Program::Unknown07AfterFlashOPWritten
    → response must be 0x07
 6  0x02 / 0x02 Program::WriteFlash
 7  for each write_pack_size chunk:
       data EP 0x02: write chunk (0xff-padded to data_packet_size)
       data EP 0x82: read 4-byte ack, last byte must be 0x04
                     (observed 41 01 01 04)
 8  0x02 / 0x08 Program::End
```

`data_packet_size` and `write_pack_size` are per-family; see
`RiscvChip::data_packet_size` / `write_pack_size` in `src/lib.rs`.

## Memory read

`ProbeSession::read_memory(address, length)`:

1. Round `length` up to a multiple of 4.
2. `0x03 SetReadMemoryRegion(address, len)`.
3. `0x02 / 0x0c Program::ReadMemory`.
4. `0x82` data endpoint: read `len` bytes in 64-byte bulk chunks.
5. Reverse each 4-byte word to fix the probe's word order.

A leading `A9 BD F9 F3` sequence indicates a read from an invalid location or
a chip that has just entered debug mode; wlink logs a warning.

## Special erase (power-off / RST pin)

`erase --method power-off|pin-rst --chip <FAMILY>` bypasses attach entirely:

```text
 0x0c SetSpeed(riscvchip=<--chip>, High)
 0x0d / 0x0f EraseCodeFlash::ByPowerOff    → 81 0d 02 0f <riscvchip>
   or 0x0d / 0x08 EraseCodeFlash::ByPinRST  → 81 0d 02 08 <riscvchip>
```

Both require a probe with power functions and a chip family whose
`support_special_erase()` is true. Because there is no auto-detection, `--chip`
is mandatory. The RST-pin variant additionally needs the nRST line wired to
the target.

## Reset and detach

| CLI | Commands |
| --- | --- |
| `wlink reset` (quit) | `0x0b / 0x01 Reset::Soft`, then ~300 ms delay |
| `wlink reset run` / `halt` | DMI halt/resume via `0x08` — see [DMI](dmi.md) |
| end of most commands | `0x0d / 0xff OptEnd` (skipped with `--no-detach`, or when the MCU must stay halted) |
| `wlink sdi-print enable` | `0x0d / 0xee 00` |
| `wlink set-power` | `0x0d / 0x09`…`0x0c` |
