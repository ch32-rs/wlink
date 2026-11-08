# Constants and Chip Families

## `riscvchip` — chip family codes

Source: `RiscvChip` in
[`src/lib.rs`](https://github.com/ch32-rs/wlink/blob/main/src/lib.rs). The
one-byte value is returned by `AttachChip` and is passed to `SetSpeed` and to
the special-erase subcommands.

| Code | Family | Series / notes |
| --- | --- | --- |
| `0x01` | `CH32V103` | RISC-V3A |
| `0x02` | `CH57X` | CH571/CH573, RISC-V3A BLE 4.2 |
| `0x03` | `CH56X` | CH565/CH569, RISC-V3A |
| `0x04` | `CH32F10X` | Cortex-M (not a RISC-V target) |
| `0x05` | `CH32V20X` | RISC-V4B/V4C |
| `0x06` | `CH32V30X` | RISC-V4C/V4F, shares type 5 traits |
| `0x07` | `CH582` | CH581/CH582/CH583, RISC-V4A BLE 5.3 |
| `0x08` | `CH32F20X` | Cortex-M (not a RISC-V target) |
| `0x09` | `CH32V003` | RISC-V2A |
| `0x0a` | `CH8571` | Undocumented RISC-V EC controller |
| `0x0b` | `CH59X` | CH59x RISC-V4C BLE 5.4, falls back as CH58X |
| `0x0c` | `CH643` | RISC-V4C, RGB display driver |
| `0x0d` | `CH32X035` | CH32X035/033, RISC-V4C USB-PD |
| `0x0e` | `CH32L103` | RISC-V4C low-power USB-PD |
| `0x0f` | `CH564` | RISC-V4J |
| `0x46` | `CH645` | CH645/CH653, RISC-V4C |
| `0x49` | `CH641` | RISC-V2A USB-PD, falls back as CH32V003 |
| `0x4b` | `CH585` | CH585/CH584, RISC-V3C BLE 5.4 / NFC / USB HS |
| `0x4e` | `CH32V00X` | CH32V002/4/5/6/7, CH32M007 |
| `0x86` | `CH32V317` | RISC-V4 |
| `0x8b` | `CH570` | CH570/CH572, RISC-V BLE |
| `0x8e` | `CH32M030` | RISC-V motor control |
| `0xa6` | `CH32V4X7` | CH32V407/CH32V467 |
| `0xab` | `CH586` | CH586/CH587, RISC-V BLE 5.4 |
| `0xc6` | `CH32H41X` | CH32H415/416/417, RISC-V5F + RISC-V3F |
| `0xce` | `CH32V205` | New-generation CH32V205/CH32V203 (QingKe V3B), unrelated to `CH32V20X` |
| `0xe6` | `CH32X3X5` | CH32X315/CH32X305, RISC-V3F |

`RiscvChip` also implements clap's `ValueEnum`, so `--chip` accepts the family
name plus part-number aliases such as `CH32H417QEU` or `CH32X315MCU6`. The
ChipID-to-part-name database is `chip_id_to_chip_name` in
[`src/chips.rs`](https://github.com/ch32-rs/wlink/blob/main/src/chips.rs).

## Speed codes (`0x0c SetSpeed`)

| Code | `Speed` variant | Clock |
| --- | --- | --- |
| `0x01` | `High` (default) | 6000 kHz |
| `0x02` | `Medium` | 4000 kHz |
| `0x03` | `Low` | 400 kHz |

## Protection flags (`0x06 ConfigChip`)

| Constant | Value |
| --- | --- |
| `FLAG_READ_PROTECTED` | `0x01` |
| `FLAG_READ_UNPROTECTED` | `0x02` |
| `FLAG_WRITE_PROTECTED` | `0x11` |
| `FLAG_WRITE_UNPROTECTED` | `0x00` |

`CheckReadProtect` returns the read flags; `CheckReadProtectEx` returns the
write flags.

## Error reasons

| `REASON` | Meaning |
| --- | --- |
| `0x55` | Failed to connect to the target RISC-V chip |
| other | Unknown; wlink preserves the whole frame in `Error::Protocol(reason, bytes)` |

## USB constants

| Constant | Value |
| --- | --- |
| `VENDOR_ID` | `0x1a86` |
| `PRODUCT_ID` (RV) | `0x8010` |
| `PRODUCT_ID_DAP` | `0x8012` |
| `ENDPOINT_OUT` / `ENDPOINT_IN` | `0x01` / `0x81` |
| `DATA_ENDPOINT_OUT` / `DATA_ENDPOINT_IN` | `0x02` / `0x82` |
| `ENDPOINT_OUT_DAP` | `0x02` |

## Flash algorithm blobs

Each supported family has a binary "flash OP" blob in
[`src/flash_op.rs`](https://github.com/ch32-rs/wlink/blob/main/src/flash_op.rs),
sent over the data endpoint after `0x02 / 0x05 Program::WriteFlashOP`. They are
opaque byte arrays selected by `RiscvChip::get_flash_op`, for example
`CH32V307: [u8; 446]`, `CH32V003: [u8; 498]`, `CH32H417: [u8; 618]`,
`CH564: [u8; 1532]`. Some families have an `_ALT` variant. Packet and write
chunk sizes come from `RiscvChip::data_packet_size` and
`RiscvChip::write_pack_size` in `src/lib.rs`.

## Debug interface keys

| Constant | Value |
| --- | --- |
| `KEY1` | `0x45670123` |
| `KEY2` | `0xCDEF89AB` |

Used to unlock flash/option-byte writes through the debug interface; see
[DMI](dmi.md).
