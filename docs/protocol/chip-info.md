# Chip Info and ESIG

Two commands identify the attached MCU: `0x0d / 0x02` (attach) returns the
chip family and ChipID, while `0x11` returns the "ESIG" block with flash size,
UID and a second ChipID field.

Sources: `AttachChipResponse` in
[`src/commands/control.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/control.rs),
`GetChipInfo` / `ESignature` in
[`src/commands/mod.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/mod.rs),
and [`src/chips.rs`](https://github.com/ch32-rs/wlink/blob/main/src/chips.rs)
for the ChipID database.

## `0x0d / 0x02` — AttachChip

Request: `81 0d 01 02`.

Response payload must be exactly 5 bytes:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 1 | `riscvchip` — chip family code, decoded via `RiscvChip::try_from_u8` |
| 1–4 | 4 | `chip_id` (`u32_be`), `0` when not reported |

Example: `82 0d 05 05 20 36 05 10` → family `0x05` (CH32V20X), ChipID
`0x20360510`.

`ProbeSession::attach` retries this command up to 3 times with a 100 ms delay
because a freshly powered target can fail the first attach. When `--chip` is
given, the returned family must match or wlink raises `ChipMismatch`.

The ChipID-to-part-name table lives in `src/chips.rs`
(`chip_id_to_chip_name`); the per-family tables in the *Chip Notes* section of
this site list the known part numbers.

## `0x11` — GetChipInfo / ESignature

| PAYLOAD[0] | Variant | When used |
| --- | --- | --- |
| `0x09` | `GetChipInfo::V1` | Firmware older than v2.9 (`version_code < 29`) |
| `0x06` | `GetChipInfo::V2` | Firmware v2.9 and newer |

> **This command does not follow the standard `82 CMD LEN PAYLOAD` response
> format.** `ESignature` overrides `Response::from_raw` and reads a raw
> 20-byte buffer directly, with no `0x82`/`0x81` marker validation. This is
> tracked in [wlink#58](https://github.com/ch32-rs/wlink/issues/58).

Observed raw response (from a source comment in `src/commands/mod.rs`):

```text
ffff 0020 aeb4abcd 16c6bc45 e339e339 20360510
^^^^ ^^^^ ^^^^^^^^ ^^^^^^^^ ^^^^^^^^ ^^^^^^^^
 │    │    │        │        │        └─ ChipID (0x20360510)
 │    │    │        │        └────────── erased-flash pattern 0xe339e339
 │    │    │        └─────────────────── UID words 1 and 2 (see byte order below)
 │    │    └─────────────────────────── flash size in KB (0x0020 = 32 KB)
 │    └──────────────────────────────── prefix / header, meaning unverified
 └───────────────────────────────────── prefix / header, meaning unverified
```

`ESignature::from_raw` only parses the first 12 bytes:

| Offset | Size | Field |
| --- | --- | --- |
| 2–3 | 2 | `flash_size_kb` (`u16_be`) |
| 4–7 | 4 | UID word 1 (`u32_be`) |
| 8–11 | 4 | UID word 2 (`u32_be`) |

The trailing 8 bytes (erased-flash marker and ChipID) are observed but **not
parsed** by wlink; their semantics are `unverified`.

### UID byte order

wlink stores the two `u32_be` words and reinterprets them as native-endian
bytes for display, producing the same order as `wchisp`:

```text
words:  aeb4abcd  16c6bc45
bytes:  cd ab b4 ae 45 bc c6 16
UID:    cd-ab-b4-ae-45-bc-c6-16
```

### Erased flash-size field

`0xE339` (per halfword) is what erased flash reads back on these MCUs, not
`0xFFFF`. When the flash-size field equals `ESignature::ERASED_FLASH_SIZE`
(`0xE339`), wlink substitutes the family's fixed flash size
(`RiscvChip::fixed_flash_size_kb`). A programmed value always wins.

Example from a test in `src/commands/mod.rs` (CH32V407VET):
`ffff 0240 50ad061a ab33546c e339e339 46700000` → 576 KB, ChipID
`0x46700000`.

## Chip family codes

The one-byte `riscvchip` code is required by `SetSpeed` (`0x0c`), by the
special erase subcommands (`0x0d / 0x08`, `0x0d / 0x0f`) and by several
per-family behaviours. The complete 28-entry table is in
[Constants and Chip Families](constants.md).
