# Frame Format

Every command exchange uses the command endpoints (`0x01` OUT / `0x81` IN).
The data endpoints are a separate, unframed stream.

Source: `Command::to_raw` and `Response::from_raw` in
[`src/commands/mod.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/mod.rs),
and `WchLink::send_command` in
[`src/probe.rs`](https://github.com/ch32-rs/wlink/blob/main/src/probe.rs).

## Request

| Byte 0 | Byte 1 | Byte 2 | Bytes 3 … n |
| --- | --- | --- | --- |
| `0x81` | `CMD` | `LEN` | `PAYLOAD` |

```rust
let mut bytes = vec![0x81, Self::COMMAND_ID, 0x00];
bytes.extend(self.payload());
bytes[2] = bytes.len() as u8 - 3;   // LEN = PAYLOAD.len()
```

`LEN` is a single byte holding the payload length, so the format allows up to
255 payload bytes. In practice wlink reads responses into a 64-byte buffer,
which caps a **response** payload at 61 bytes; all implemented commands use far
smaller payloads.

Example — `GetProbeInfo` (`0x0d`, subcommand `0x01`):

```text
81 0d 01 01
^^ ^^ ^^ ^^
|  |  |  └─ PAYLOAD[0] = 0x01 (GetProbeInfo)
|  |  └──── LEN = 1
|  └─────── CMD = 0x0d (Control)
└────────── request marker
```

## Success response

| Byte 0 | Byte 1 | Byte 2 | Bytes 3 … n |
| --- | --- | --- | --- |
| `0x82` | `CMD` | `LEN` | `PAYLOAD` |

The parser checks `resp[0] == 0x82`, verifies `LEN == resp[3..].len()`, then
hands `resp[3..]` to the response type. It does **not** verify that `CMD`
matches the request.

Example — `GetProbeInfo` responding with firmware `2.15` on a WCH-LinkE:

```text
82 0d 03 02 0f 02
         ^^^^^^^^^^ PAYLOAD = [0x02, 0x0f, 0x02]
```

## Error response

| Byte 0 | Byte 1 | Byte 2 | Bytes 3 … n |
| --- | --- | --- | --- |
| `0x81` | `REASON` | `LEN` | `PAYLOAD` |

An `0x81` first byte means failure regardless of `CMD`. wlink validates the
length and surfaces `Error::Protocol(reason, response_bytes)`.

| `REASON` | Meaning |
| --- | --- |
| `0x55` | Failed to connect to the target RISC-V chip |
| other | Opaque; the full frame is preserved in the error |

## Response decoding by type

| Rust type | Rule |
| --- | --- |
| `()` | Payload ignored, success is enough |
| `u8` | Payload must be exactly 1 byte |
| `bool` | Payload must be 1 byte; `0x01` means success (`SetSpeed`) |
| `Vec<u8>` | Raw payload returned |
| `AttachChipResponse` | Exactly 5 bytes: family `u8` + ChipID `u32_be` |
| `DmiOpResponse` | Exactly 6 bytes: addr `u8` + data `u32_be` + op `u8` |
| `ProbeInfo` | At least 3 bytes: major, minor, optional variant |
| `ESignature` | **Non-standard** 20-byte raw response, see [Chip Info](chip-info.md) |
| invalid marker | `Error::InvalidPayload` |
| length mismatch | `Error::InvalidPayloadLength` |

## Data endpoint stream

The data endpoints carry no `81/82` header. Two shapes occur:

- **Bulk output**: the flash algorithm blob for the chip family, then firmware
  data split into `write_pack_size` chunks, each padded with `0xff` to
  `data_packet_size` (see `WchLink::write_data_with_progress`).
- **Bulk input**: `read_data(n)` accumulates 64-byte bulk reads until `n`
  bytes are collected. During a fast-program cycle, each written chunk is
  followed by a 4-byte acknowledgement on `0x82`; wlink checks that the last
  byte is `0x04` (observed `41 01 01 04`).

`ProbeSession::read_memory` reverses each 4-byte word of the returned memory
buffer to compensate for the probe's word order.
