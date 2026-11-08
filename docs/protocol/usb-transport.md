# USB Transport

WCH-Link uses USB **bulk transfers**. There is no kernel driver in the loop on
Linux/macOS: wlink opens the device with `nusb`, claims interface 0, and reads
and writes bulk endpoints directly. On 32-bit Windows an optional native
`WCHLinkDLL.dll` (CH375) backend is available, falling back to `nusb`.

Source: [`src/probe.rs`](https://github.com/ch32-rs/wlink/blob/main/src/probe.rs)
(constants), [`src/usb_device.rs`](https://github.com/ch32-rs/wlink/blob/main/src/usb_device.rs)
(backend implementations).

## USB identifiers

| Mode | VID | PID | Notes |
| --- | --- | --- | --- |
| RV (RISC-V debug/flash) | `0x1a86` | `0x8010` | The mode wlink operates in |
| DAP (ARM SWD) | `0x1a86` | `0x8012` | wlink refuses to flash in this mode |

The same physical probe appears under one PID or the other depending on its
mode. `wlink mode-switch` flips between them.

## Endpoints

| Endpoint | Direction | Purpose |
| --- | --- | --- |
| `0x01` | OUT | Command frames (`81 CMD LEN PAYLOAD`) |
| `0x81` | IN | Command responses (`82 …` / `81 …`) |
| `0x02` | OUT | Data endpoint: flash algorithm blob, firmware chunks |
| `0x82` | IN | Data endpoint: memory dump, per-chunk write acknowledgement |
| `0x02` | OUT (DAP mode only) | Raw mode-switch frame |

Command reads use a 64-byte buffer; a single bulk read is treated as one
response frame. Data-endpoint transfers are streams, not frames: wlink loops
over 64-byte bulk reads until the requested byte count is reached, and pads the
final written chunk with `0xff` to the family's data packet size.

The `nusb` backend stores a 5-second default in `NusbDevice::timeout`; the
current bulk read/write calls do not apply it and rely on nusb's own blocking
behaviour.

## Selecting a probe

With several probes attached, enumeration order is not stable. wlink accepts
either an index or a USB serial number:

```bash
wlink list
# <WCH-Link#0 nusb device> ID 1a86:8010 Serial F93D8F067E10 (USB-FS 12 Mbps) (RV mode)
# <WCH-Link#1 nusb device> ID 1a86:8010 Serial D7708F065C7F (USB-FS 12 Mbps) (RV mode)

wlink -d 1a86:8010:D7708F065C7F status   # probe-rs style VID:PID:SERIAL
wlink -d D7708F065C7F status             # serial alone
wlink -d 1 status                        # legacy index
```

Serial matching is case-insensitive and works in either RV or DAP mode; the
VID/PID in the selector are validated but not used for matching.

## Mode switching

Switching from RV to DAP mode sends a command frame on the normal command
endpoint:

| Direction | Raw bytes | Meaning |
| --- | --- | --- |
| RV → DAP | `81 ff 01 41` | `RawCommand<0xff>` with payload `0x41` |
| DAP → RV | `81 ff 01 52` | written raw to endpoint `0x02` while in DAP mode |

Both operations are experimental; the CH549-based first-generation probe can
only switch modes by reflashing its firmware. Support is reported by
`WchLinkVariant::support_switch_mode` in `src/probe.rs`.

## Variants

| Code | Variant | Hardware | Capabilities |
| --- | --- | --- | --- |
| `0x01` | `WchLinkVariant::Ch549` | WCH-Link (CH549) | No CH32V00X, no mode switch, no power/SDI functions |
| `0x02` / `0x12` | `WchLinkVariant::ECh32v305` | WCH-LinkE (CH32V305) | Full support, including SDI print |
| `0x03` | `WchLinkVariant::SCh32v203` | WCH-LinkS (CH32V203) | Full chip support |
| `0x05` / `0x85` | `WchLinkVariant::WCh32v208` | WCH-LinkW (CH32V208) | Wireless variant; no CH56X/CH57X/CH582/CH59X |
| other | — | — | `Error::UnknownLinkVariant` |

See [Probe Info and Firmware Version](probe-info.md) for how the variant byte
is returned and how firmware versions are decoded.
