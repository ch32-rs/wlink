# WCH-LinkRV Protocol

WCH-Link uses USB Bulk Transfer.

```rust
const VENDOR_ID: u16 = 0x1a86;
const PRODUCT_ID: u16 = 0x8010;

const ENDPOINT_OUT: u8 = 0x01;
const ENDPOINT_IN: u8 = 0x81;

// const RAW_ENDPOINT_OUT: u8 = 0x02;
// const RAW_ENDPOINT_IN: u8 = 0x82;
```

## USB Packet

Request packet:

| 0    | 1   | 2   | 3 ... n     |
| ---- | --- | --- | ----------- |
| 0x81 | CMD | LEN | PAYLOAD ... |

Success response packet:

| 0    | 1   | 2   | 3 ... n     |
| ---- | --- | --- | ----------- |
| 0x82 | CMD | LEN | PAYLOAD ... |

Error response packet:

| 0    | 1      | 2   | 3 ... n     |
| ---- | ------ | --- | ----------- |
| 0x81 | REASON | LEN | PAYLOAD ... |

where:

- LEN = PAYLOAD.len()
- PAYLOAD

## Command

- 0x01 - Set address and size
- 0x02 - Program
- 0x03 - Memory read
- 0x06 - Flash Read Protect
- 0x08 - DMI OP
- 0x0b - Reset
- 0x0c - ???
- 0x0d - Info
- 0x0e - Disable debug for riscvchip 2, 3
- 0x0f - ? beigin verify

### 0x01 - Set RAM address and size

### 0x02 - Program

- 0x01 Erase
- 0x03 execute ram?
- 0x05 begin buck transfer?
- 0x06 ?? - wlink_ready_write
- 0x07 Verify
- 0x08 End Program
- 0x09 buck transfer ends
- 0x0a ? Verify
- 0x0b for riscvchip 1, = verify
- 0x0c BeginReadMemory

### 0x03 - Memory Read

- offset: u32
- len: u32

### 0x06 - Flash Read Protect

- 0x01 Check if flash is read-protected
  - 0x01 protected, read-memory return random data
  - 0x02 not protected
- 0x03 Set flash read-protected
- 0x02 Set flash read-unprotected

Set subcommand requires quitreset for riscvchip 1

### 0x08 - DMI OP

PAYLOAD

| iAddr_u8 | iData_u32_be | iOp_u8 |
| -------- | ------------ | ------ |

Response PAYLOAD

| oAddr_u8 | oData_u32_be | oOp_u8 |
| -------- | ------------ | ------ |

where:

oOp_u8 = 0x02 when failed

### 0x0b - Reset

- 0x01 Quit reset
  - 300ms delay after this command
- 0x02 Reset for riscvchip 0x02
- 0x03 Reset, normal?

### 0x0d - Control

- 0x01 Get firmware version
- 0x02 Connect chip
- 0x03 ? stage after connect chip and read riscvchip, for riscvchip 1
- 0x04 get rom ram split, for riscvchip 3, 5, 6, 9
- 0xff End process

### 0x0e

- 0x01 Disable debug for riscvchip 2, 3

## Error Reason

- 0x55: failed to connect with riscvchip

## Constants

### RiscvChip

```rust
/// Currently supported RISC-V chip series
#[repr(u8)]
pub enum RiscvChip {
    /// CH32V103 RISC-V3A series
    CH32V103 = 0x01,
    /// CH571/CH573 RISC-V3A BLE 4.2 series
    CH57x = 0x02,
    /// CH565/CH569 RISC-V3A series
    CH56x = 0x03,
    /// CH32V20x RISC-V4B/V4C series
    CH32V20x = 0x05,
    /// CH32V30x RISC-V4C/V4F series
    CH32V30x = 0x06,
    /// CH581/CH582/CH583 RISC-V4A BLE 5.3 series
    CH58x = 0x07,
    /// CH32V003 RISC-V2A series
    CH32V003 = 0x09,
}
```

## Firmware Versions

WCH-LinkUtility v2.8

MRS IDE v2.7

## Variants

| MCU       | Variant            | Description                            |
| --------- | ------------------ | -------------------------------------- |
| CH549     | WCH-Link-R1-1v1    | Swith mode by changing firmware        |
| CH32V305F | WCH-LinkE-R0-1v3   | Swith mode by button or EEPROM setting |
| CH32V203  | WCH-LinkS-CH32V203 |                                        |
| ??        | WCH-LinkB          |                                        |

## References

- Official WCH-Link Homepage(English) \
  <http://www.wch-ic.com/products/WCH-Link.html>
- Official WCH-Link Homepage(Chinese) \
  <https://www.wch.cn/products/WCH-Link.html>

### WCH-OpenOCD

Since WCH updates there firmware to 2.8, the old version of WCH-OpenOCD might be not working.

- <https://github.com/Seneral/riscv-openocd-wch>
- <https://github.com/jiegec/riscv-openocd>
- <https://github.com/kprasadvnsi/riscv-openocd-wch>
