# USB Transport

WCH-Link uses USB **bulk transfers**. On Linux/macOS wlink opens the device
with `nusb`, claims interface 0, and reads and writes bulk endpoints directly.
Windows additionally has two vendor-specific backends that are tried before
`nusb`; see [Windows driver backends](#windows-driver-backends).

Source: [`src/probe.rs`](https://github.com/ch32-rs/wlink/blob/main/src/probe.rs)
(constants), [`src/usb_device.rs`](https://github.com/ch32-rs/wlink/blob/main/src/usb_device.rs)
(backend selection),
[`src/usb_device/wch_link_driver.rs`](https://github.com/ch32-rs/wlink/blob/main/src/usb_device/wch_link_driver.rs)
(native Windows driver protocol).

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

## Windows driver backends

On Windows, wlink tries these backends in order and uses the first one that can
open a probe with the requested VID/PID:

1. the legacy `WCHLinkDLL.dll` (CH375) DLL — 32-bit builds only;
2. WCH's native driver (`WCHLinkDrv` / `WCHLinkW64.SYS`) — 64-bit Windows
   (x64 and ARM64). The backend is compiled for x86 too, but WCH ships no
   32-bit build of that driver;
3. WinUSB via [Zadig](https://zadig.akeo.ie), through `nusb` — all platforms.

The native and WinUSB backends are not interchangeable at the USB level. A
probe bound to WCH's native driver is **not** a WinUSB device: `nusb` can still
enumerate it and read its USB serial number, but opening interface 0 fails with
`incompatible driver is installed for this interface`. `wlink list` therefore
reports one view (the first backend that finds probes), and the ambiguous-probe
guard counts probes visible to the native driver as well.

### Native driver IOCTL

The driver installer registers a device interface class
`{F8D5EDCA-B647-4E9C-9BD3-A5BD-2328D55C}`. wlink enumerates present interfaces
with `SetupDiGetClassDevsW` / `SetupDiEnumDeviceInterfaces`, resolves each path
with `SetupDiGetDeviceInterfaceDetailW`, and opens it with `CreateFileW`.
Every descriptor query, bulk read, bulk write and timeout goes through a single
vendor `DeviceIoControl` (`0x00223CDC`, `METHOD_BUFFERED`) on that handle —
there are no bulk USB transfers in this backend.

There is no public specification for this interface; the layout below comes
from observed driver behaviour, originally proposed in
[#116](https://github.com/ch32-rs/wlink/pull/116). Request and response share
one buffer, with an 8-byte header in front of the response:

| Offset | Size | Meaning |
| --- | --- | --- |
| `0..4` | u32 | Command word. For bulk transfers byte 0 is the endpoint index (USB address minus 1) and byte 2 is `0x01` (read) or `0x02` (write) |
| `4..8` | u32 | Payload length; on a read the driver replaces it with the number of bytes returned |

| Command | Header | Payload | Purpose |
| --- | --- | --- | --- |
| GET_DESCRIPTOR(Device) | `0x04`, `8` | 8-byte USB setup packet (`80 06 00 01 00 00 12 00`) | Returns the 18-byte device descriptor, used to match VID/PID |
| bulk read | endpoint, `0x01`, `length` | response only | Reads from a bulk IN endpoint |
| bulk write | endpoint, `0x02`, `length` | bytes to write | Writes to a bulk OUT endpoint |
| SET_TIMEOUT | `0x0F`, `0x10` | four u32 millisecond values | Sets the driver's timeouts |

The driver exposes no USB serial through this interface, so native-driver (and
CH375) probes can only be selected by index. See
[Selecting a probe](#selecting-a-probe): a `-d 1a86:8010:<SERIAL>` selector
requires WinUSB; with either vendor driver use `-d <INDEX>` instead.

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
VID/PID in the selector are validated but not used for matching. A serial
selector requires a WinUSB/libusb driver, since the Windows vendor backends
cannot open a device by serial; a mistyped serial never falls back to an index.

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
