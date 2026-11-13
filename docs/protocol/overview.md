# Overview and Status

The WCH-Link probe exposes a vendor-specific USB bulk protocol. There is no
public specification from WCH; this reference documents the protocol as
implemented by wlink, which was reconstructed from USB captures, WCH's
toolchain, and other open-source implementations.

## Status of this document

This reference replaces the repository-root `protocol.md`. That file was last
updated in November 2023 and had fallen behind the implementation — it had an
empty `0x01` section, two empty DMI tables, a dozen `???` markers, listed
`0x0c` as unknown (it is `SetSpeed`), omitted `0x11` entirely, listed only 7
chip families (there are now 28), and stopped at firmware v2.8.

The tables in this reference are derived from the current source on `main`.
Every command page names the Rust symbol that implements it. Anything that
could not be confirmed from the implementation is collected in
[Unverified Commands and Captures](unverified.md) and marked `unverified`
rather than guessed.

> If you find a mismatch between this document and the code, the code is
> correct and the document needs a PR.

## Notation

| Convention | Meaning |
| --- | --- |
| `0x81 0d 01 01` | Hex bytes in transmission order |
| `u32_be` | 32-bit unsigned integer, most significant byte first |
| `LEN` | Payload byte count, one byte |
| `()` | Command succeeds with no payload of interest |
| `unverified` | Seen in a USB capture or an older note, not implemented/confirmed in wlink |
| `riscvchip` | The one-byte chip family code, see [Constants](constants.md) |

All multi-byte integers in command payloads are **big-endian**. Memory read
data from the data endpoint is an exception: wlink reverses every 4-byte word
after reading it (see [Operation Flows](flows.md)).

## Layer model

```text
Host (wlink CLI)
  │
  ├─ Command endpoint (0x01 OUT / 0x81 IN)
  │    81 CMD LEN PAYLOAD   →   82 CMD LEN PAYLOAD
  │    all control, configuration and DMI operations
  │
  ├─ Data endpoint (0x02 OUT / 0x82 IN)
  │    unframed bulk stream: flash algorithm blobs, firmware bytes, memory dumps
  │
  └─ DMI operations (command 0x08)
        └─ RISC-V Debug Module registers (0x04 … 0x40)
```

- [USB Transport](usb-transport.md) describes the device, interfaces and endpoints.
- [Frame Format](frame-format.md) describes the command frames.
- [Command Reference](commands.md) lists every command ID and payload.
- [Control Subcommands](control-subcommands.md) expands the multi-purpose `0x0d` command.
- [DMI and the Debug Module](dmi.md) covers the debug layer above command `0x08`.

## Known limitations of the protocol itself

- `LEN` is a single byte. wlink reads responses into a 64-byte buffer, so a
  response payload is limited to 61 bytes in practice; large transfers always
  go through the data endpoint.
- The response parser does not check that the response `CMD` byte matches the
  request; it only distinguishes the `0x82` success marker from the `0x81`
  error marker.
- Error frames carry an opaque one-byte `REASON`. Only `0x55` (failed to
  connect to the target) has a known meaning.
- Some subcommands were observed only in captures from WCH's Windows utility;
  they are listed separately as `unverified`.

## Further reading

- [External References](../references.md) — WCH manuals, schematics, firmware archives and other FOSS implementations.
- `CHANGELOG.md` in the repository records protocol-level fixes over time, for
  example the correction that `0x0c` is a speed command rather than a verify command.
