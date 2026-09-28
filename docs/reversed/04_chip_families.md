# 04 — 芯片族 ↔ 模式 ↔ DLL 命令族 对应表

## 1. GUI 显示名 (fcn.0040d180 给 combo)

按 push 顺序写入 combo box 的字符串 (共 ~33 个 family):

```
CH565/9, CH57X, CH582/3, CH590/1/2,
CH32V003, CH32V10X, CH32V20X, CH32V30X,
CH32X035/4/3, CH32L103/M103,
CH641, CH643, CH645, CH653,
CH32V002/4/5/6/7, CH32M007,
CH564, CH32V317, CH581/4/5, CH32M030,
CH570/2, CH32H41X, CH32V205/3CCT, CH595/6,
CH32V4X7, CH32X315, CH586/7,
CH32V203, CH32V205, CH32L103, CH32V006,
CH32F10X, CH32F20X
```

## 2. WCH 内核分类

按目前公开资料和 wlink 已实现的 enum, 芯片按调试 IP 分以下代:

| Family | QingKe 内核 | debug iface | 备注 |
|---|---|---|---|
| CH32V003 | V2 (RISC-V2A) | **1-wire SDI** | 支持单线 SDI |
| CH32V002/4/5/6/7 | V2 | 1-wire SDI |  |
| CH32V006 | V2 | 1-wire SDI | "V006" 是新 SKU |
| CH32V10X | V3 | SWT/2-wire (SWD-derived) |  |
| CH32F10X | (no RV) | ARM Cortex-M3 SWD | 仅 `_DAP` |
| CH32V20X / V208 | V3/V4 | 2-wire |  |
| CH32F20X | (no RV) | ARM Cortex-M3 SWD |  |
| CH32V30X / V31X / V317 | V4F | 2-wire |  |
| CH32V305 | V4 | 2-wire |  |
| CH32X035/4/3 | V4C | 2-wire |  |
| CH32X315 | V4 | 2-wire |  |
| CH32L103 / M103 | V4F | 2-wire | 集成 LF |
| CH32V203 | V3B | 2-wire |  |
| CH32V205 / 3CCT | V4B | 2-wire |  |
| CH32V4X7 | V4 | 2-wire |  |
| CH32H41X | **V5** | 2-wire | 高速 |
| CH32M007 / M030 | V2/V4 | 2-wire |  |
| CH56X / CH57X / CH58X / CH59X | BLE-RV | 2-wire |  exile (BLE) |
| CH64X | RV | 2-wire |  |
| CH641/643/645/653 | RV | 2-wire | ultra low cost |

## 3. 模式 ↔ 命令族

每条 family 对应一个 `McuCompiler_SetChipType(idx)` 索引，DLL 内 chip-table 决定:
- 选 1-wire SDI 还是 2-wire
- 用 `_Ex` family (BLE) 还是 base (CH32V)
- 命令是 HID (RV) 还是 DAP (ARM)

**GUI 的实际选择**取决于用户为 chip 选 GUI 的 family (combo box)，以及当前 link 是 RV 还是 DAP mode:

```
ChipFamily ∧ LinkMode ∈ {RV, DAP, ARM-HID}
        ↓
{base, _Ex, _DAP} 命令变体
```

### 3.1 一些已知映射 (证据 + infer)

| Family | wlink 内部 chip id | 走哪条路径 | 备注 |
|---|---|---|---|
| CH32V003 | 0x00 | RV/HID base | SDI 1-wire |
| CH32V002/4/5/6/7 | 0x10..0x17 | RV/HID base | SDI 1-wire |
| CH32V006 | (同属 0x16) | RV/HID base |  |
| CH32V10X | 0x?? | RV/HID base | 1-wire |
| CH32V20X | 0x1d | RV/HID base | 2-wire |
| CH32V203/205 | 0x?? | RV/HID base | 2-wire |
| CH32V30X/V31X/V317 | 0x3a | RV/HID base | 2-wire |
| CH32X035 | 0x25 | RV/HID base | 2-wire |
| CH32L103/M103 | 0x?? | RV/HID base | 2-wire |
| CH32X315 | 0x?? | RV/HID base | 2-wire |
| CH32V4X7 | 0x?? | RV/HID base | 2-wire |
| CH32H41X | **0x?? (V5)** | RV/HID base | 2-wire |
| CH32M030 | 0x?? | RV/HID base | 2-wire |
| CH32M007 | 0x?? | RV/HID base | 1-wire? |
| CH32F10X | (ARM) | **`_DAP` only** | SWD |
| CH32F20X | (ARM) | **`_DAP` only** | SWD |
| CH58X/59X/57X/56X | 0x82 (CH583) | RV/HID `_Ex` family | BLE |
| CH565/9 | 0x?? | RV/HID `_Ex` |  |
| CH64X (641/643/645/653) | 0x?? | RV/HID | 2-wire |

注: wlink 中的 `0x10..0x17` / `0x1d` / `0x3a` 等是 wlink 自己的 chipid enum,**不是** WCH 的内部 ID。
要拿到 WCH 的官方 chipid 表 (`SetChipType(idx)` 在 DLL 内索引到的表):
- 反编译 `fcn.10009e30` (`McuCompiler_SetChipType`) 看 idx -> chip 参数
- 或抓 USB 包

## 4. 关键函数用 chip id

`McuCompiler_SetChipType_DAP` 接受的就是 GUI combo 的 index。看 `fcn.1000d180` (SetChipType_DAP) 反汇编
看它怎么转换 idx -> cmd 里的 chip-id byte。目前还没做到这段。

## 5. 已知 family 专属特性

| 特性 | 涉及 family | DLL 证据 |
|---|---|---|
| SDI printf | **仅** V2/V4 SDI 芯片 (CH32V003 etc) | `"Current chip type does`t support SDI Printf function!"` |
| 1-wire/2-wire toggle | 多数 RV | `"The current chip does not support 2-wires serial debug mode!"` |
| ARM DAP | CH32F10X/F20X | `_DAP` 后缀 |
| 大 flash  >= 32KB | CH32V30X+ | `McuCompiler_GetFlashSum` (cmd 0xfd050d81) |
| 分区 (Code+Boot+User) | BLE-RV | `McuCompiler_GetROMRAM/SetROMRAM` cmd 0x180200d81 |
| "Version A" 下载禁区 | 旧版 X035 | `"chip of Version A does not support boot area download!"` |
| Boot+User 双区下载 | BLE-RV only in 2-wire | `"chip does not support BOOT  + USER area download in 2-wires mode!"` |
| IAP / USB Bootloader | 大部分 | MRSFunc_* + `SetIAPMode` |

## 6. TODO 待反编译验证

- [ ] `SetChipType` chip-id 映射表 (`fcn.10009e30`)
- [ ] 各 family option-byte 地址和默认 mask
- [ ] BLE-RV 的 `Boot+User` 双 region 起始地址表

这些都需要再拆 DLL；当前证据已足够给 wlink 提供 high-level 参考。
