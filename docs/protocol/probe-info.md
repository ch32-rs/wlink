# Probe Info and Firmware Version

Source: `GetProbeInfo`, `ProbeInfo` and the version helpers in
[`src/commands/control.rs`](https://github.com/ch32-rs/wlink/blob/main/src/commands/control.rs);
`WchLinkVariant` in
[`src/probe.rs`](https://github.com/ch32-rs/wlink/blob/main/src/probe.rs).

## `0x0d / 0x01` — GetProbeInfo

Request: `81 0d 01 01`.

Response payload (`ProbeInfo::from_payload` requires at least 3 bytes):

| Offset | Field | Notes |
| --- | --- | --- |
| 0 | `major_version` | Raw major byte as reported by the probe |
| 1 | `minor_version` | Raw minor byte (may exceed 15 — see below) |
| 2 | `variant` | Only interpreted when the payload is **4** bytes long; otherwise wlink assumes `Ch549` |
| 3 | — | Present in the 4-byte form; meaning `unverified` |

Example: `82 0d 04 02 0f 02 00` → major `0x02`, minor `0x0f`, variant
`0x02` (WCH-LinkE-CH32V305), with a fourth payload byte of unknown meaning.
A 3-byte payload carries no variant and makes wlink assume `Ch549`.

## Firmware version encoding

The probe does **not** report a decimal version. Its two bytes are a packed
value, and WCH's tooling displays versions as `major * 10 + minor`. This is why
README says "firmware version 2.15 is actually v35".

For raw bytes `(major, minor)`:

```text
packed        = 16 * major + minor
version_code  = if packed < 0x30 { 10 * major + minor } else { packed - 12 }
display       = (packed >> 4, packed & 0x0f)
```

`version_code` is monotonic and is what wlink uses for all comparisons.
`display_version` reconstructs the human-readable version by treating the
packed value as two nibbles, so an "impossible" minor carries into the major.

| Raw bytes | `packed` | `version_code` | Displayed as | Note |
| --- | --- | --- | --- | --- |
| `02 08` | `0x28` | 28 | 2.8 | Older firmware |
| `02 0f` | `0x2f` | 35 | 2.15 (v35) | Current baseline in README |
| `02 16` | `0x36` | 42 | 3.6 | wlink source spells newer builds as `2.22` |
| `02 0a` | `0x2a` | 30 | 2.10 (v30) | First firmware with SDI print |

### Minimum firmware versions used by wlink

| Constant | Value | Purpose |
| --- | --- | --- |
| `MIN_FW_VERSION_CHIP_INFO_V2` | 29 (`v2.9`) | Use `GetChipInfo::V2` (`0x11 / 0x06`) instead of V1 |
| `MIN_FW_VERSION_CH32V205` | 42 (`v3.6`, sometimes written `2.22`) | First firmware that reports the new-generation CH32V205 family (`0xce`) correctly |

Older firmware misdetects CH32V205 as CH32V20X with garbage ChipID/ESIG;
wlink emits a warning but continues.

## Variant codes and capabilities

`WchLinkVariant::try_from_u8` accepts:

| Byte | Variant |
| --- | --- |
| `0x01` | `Ch549` |
| `0x02`, `0x12` | `ECh32v305` |
| `0x03` | `SCh32v203` |
| `0x05`, `0x85` | `WCh32v208` |
| other | `Error::UnknownLinkVariant` |

| Variant | Mode switch | Power output | SDI print | Chip restrictions |
| --- | --- | --- | --- | --- |
| `Ch549` | no | no | no | No CH32V003 / CH32X035 / CH643 |
| `ECh32v305` | yes | yes | yes | none |
| `SCh32v203` | yes | no | no | none |
| `WCh32v208` | yes | yes | no | No CH56X / CH57X / CH582 / CH59X |

## Hardware variants (historical table)

The old `protocol.md` listed these; they are retained here for orientation but
have not been re-verified against current hardware.

| Probe MCU | Variant | Note |
| --- | --- | --- |
| CH549 | WCH-Link-R1-1v1 | Switch mode by reflashing firmware |
| CH32V305F | WCH-LinkE-R0-1v3 | Switch mode by button or EEPROM setting |
| CH32V203 | WCH-LinkS-CH32V203 | — |
| `unverified` | WCH-LinkB | Hardware MCU unknown |

See [USB Transport](usb-transport.md) for VID/PID and mode-switch frames.
