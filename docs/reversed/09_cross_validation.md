# 09 — 交叉验证矩阵 (Cross-Validation)

每张表在多个文档/证据文件中重复出现,便于随时对照。  

格式：`<Field>` 在 `<Doc A>` 提到,可在 `<Evidence>` 验证,对应 `<Doc B>`。

## 1. 命令字节 ↔ 函数 ↔ 文档

| Cmd Dword (LE) | byte1:byte0 | byte2 | 函数 | 03 章 | 05 章 | 06 章 | 证据 |
|---|---|---|---|---|---|---|---|
| `0x1010681` | 01:06 | 01 | CheckReadPro | ✅ | ✅ | ✅ | `_cmd-func-map.txt` |
| `0x1010b81` | 01:0b | 01 | Reset | ✅ | | ✅ | 同上 |
| `0x6010b81` | 01:0b | 01 | ResetB | ✅ | | ✅ | 同上 |
| `0x3010681` | 01:06 | 01 | EnableReadPro | ✅ | ✅ | | 同上 |
| `0x2010681` | 01:06 | 01 | DisableReadPro | ✅ | ✅ | | 同上 |
| `0x4010681` | 01:06 | 04 | CheckReadPro_Ex | ✅ | ✅ | | 同上 |
| `0x3080681` | 08:06 | 01 | EnableReadPro_Ex | ✅ | ✅ | | 同上 |
| `0x2080681` | 08:06 | 01 | DisableReadPro_Ex | ✅ | ✅ | | 同上 |
| `0x6010d81` | 01:0d | 01 | CheckQE | ✅ | ✅ | | 同上 |
| `0x7010d81` | 01:0d | 01 | EnableQE | ✅ | ✅ | | 同上 |
| `0xee020d81` | 02:0d | 0xee | SetSDIPrn / SetSDLineMode | ✅ | ✅ | | 同上 |
| `0x1010e81` | 01:0e | 01 | DisableDebug | ✅ | ✅ | | 同上 |
| `0x1010f81` | 01:0f | 01 | SetIAPMode | ✅ | | ✅ | 同上 |
| `0x2010f81` | 01:0f | 02 | GetIAPType | ✅ | | ✅ | 同上 |
| `0x1010281` | 02:01 | 01 | Clear | ✅ | | ✅ | 同上 |
| `0x2010281` | 02:01 | 01 | Download_DAP | ✅ | | ✅ | 同上 |
| `0x5010281` | 02:01 | 05 | Download | ✅ | | ✅ | 同上 |
| `0x7010281` | 02:01 | 07 | Download commit | ✅ | | ✅ | 同上 |
| `0x8020d81` | 02:0d | 08 | ClearCodeFlash | ✅ | | ✅ | 同上 |
| `0xf010d81` | 02:0d | 0x0f | ClearCodeFlashB | ✅ | | ✅ | 同上 |
| `0xff010d81` | 01:0d | 0xff | OptEnd | ✅ | ✅ | | 同上 |
| `0x11050d81` | 05:0d | 01 | SetAccessAddress | ✅ | ✅ | | 同上 |
| `0x12010d81` | 01:0d | 01 | GetAccessAddress | ✅ | ✅ | | 同上 |
| `0x16010d81` | 01:0d | 01 | SetChipType | ✅ | | ✅ | 同上 |
| `0x17010d81` | 01:0d | 01 | GetROMRAM | ✅ | ✅ | | 同上 |
| `0x4010d81` | 01:0d | 04 | GetROMRAM (alt) | ✅ | ✅ | | 同上 |
| `0x18020d81` | 02:0d | 01 | SetROMRAM | ✅ | ✅ | | 同上 |
| `0x5020d81` | 02:0d | 05 | SetROMRAM (idx5) | ✅ | ✅ | | 同上 |
| `0xfd050d81` | 05:0d | 0xfd | GetFlashSum | ✅ | | ✅ | 同上 |
| `0xfc020d81` | 02:0d | 0xfc | SetClearFlashPara | ✅ | | ✅ | 同上 |
| `0x5201ff81` | 01:ff | 5 | ChangeArmToRvLink | ✅ | | ✅ | 同上 |
| `0x6301ff81` | 01:ff | 6 | ChangeDAPToHidLink | ✅ | | ✅ | 同上 |
| `0x7401ff81` | 01:ff | 7 | ChangeHIDToDAPLink | ✅ | | ✅ | 同上 |
| `0xf8040d81`..`0xfb040d81` | 04:0d | 0xF8-FB | DownloadExternal | ✅ | | ✅ | 同上 |

## 2. 全局变量 ↔ 用途 ↔ 证据 (transport)

| Vaddr | 含义 | 02 章 | 证据 (`F-dll-helpers.txt`) |
|---|---|---|---|
| `0x10179940` | CH375GetDeviceDescr fn | ✅ | ✅ |
| `0x10179944` | CH375GetDeviceName | ✅ | ✅ |
| `0x10179948` | **CH375WriteEndP** | ✅ | ✅ |
| `0x1017994c` | **CH375ReadEndP** | ✅ | ✅ |
| `0x10179950` | CH375SetTimeoutEx | ✅ | ✅ |
| `0x10179954` | CH375OpenDevice | ✅ | ✅ |
| `0x10179958` | CH375CloseDevice | ✅ | ✅ |
| `0x10179994` | selected link idx | ✅ | (`EnumLink` 写) |
| `0x10179998` | opened device idx | ✅ | (`OpenDevice` 写) |
| `0x101799d4` | DAP libusb_device_handle* | ✅ | (`EnumLink_DAP`) |
| `0x101799d8` | DAP libusb_context* | ✅ | 同上 |
| `0x101799dc` | WCHLinkDll.dll hModule | ✅ | ✅ (`fcn.1000a850`) |
| `0x1016d470` | GS cookie | — | (VS runtime) |

## 3. USB VID / PID ↔ 证据

| 值 | 上下文 | 证据 |
|---|---|---|
| `1A86` | WCH VID | `F-dll-all-funcs.txt` 在 EnumLink `cmp word [var_114h], 0x1a86` |
| `8010` | WCH-Link RV-HID PID | 同上, `cmp word [var_112h], 0x8010` |
| `0x0a` | 最多枚举 10 个 device | EnumLink 内 `cmp edi, 0xa` |

## 4. GUI chip 字符串 ↔ 文档

| Combo String | 04 章 | GUI 反汇编 (`F-chiplist-init.txt`) |
|---|---|---|
| `CH565/9` | ✅ | `push str.CH565_9` |
| `CH57X` | ✅ | `push str.CH57X` |
| `CH582/3` | ✅ | `push str.CH582_3` |
| `CH590/1/2` | ✅ | `push str.CH590_1_2` |
| `CH32V003` | ✅ | `push str.CH32V003` |
| `CH32V002/4/5/6/7` | ✅ | `push str.CH32V002_4_5_6_7` |
| `CH32V006` | ✅ | `push str.CH32V006` |
| `CH32F10X` / `CH32F20X` | ✅ | (gui 反汇编最后一对) |
| `CH32X035/4/3` / `CH32L103/M103` | ✅ | 同上 |
| `CH32V30X/V10X/V20X/V006/V002` | ✅ | 同上 |
| `CH32V4X7` / `CH32X315` | ✅ | 同上 |
| `CH32H41X` | ✅ | 同上 |
| `CH32V317` / `CH32M030` | ✅ | 同上 |
| `CH641/643/645/653` | ✅ | 同上 |
| `CH581/4/5/6` / `CH595/6` | ✅ | 同上 |
| ... | | |

## 5. Transport 选择 ↔ 函数 (按 `_calls.tsv`)

证据 `_calls.tsv` (生成), 02 章使用。

```
KERNEL32 WriteFile (HID path)         <-  ARM-HID legacy 模式
libusb_bulk_transfer (DAP path)       <-  ARM / DAP 模式
CH375WriteEndP (RV path)              <-  RV/HID (default) 模式, 通过 [0x10179948]
```

## 6. 已知 chip-id ↔ family

wlink 用 chip-id enum, WCH 用 combo idx。需要从 `SetChipType` 反编译拿映射。  
(尚未做 — 但接口都是 `SetChipType(idx)`。)

## 7. 错误文案 → 路径

每个错误字符串对应 1 个 sanity check 分支:
| 文案 | 出现函数/路径 |
|---|---|
| `"chip of Version A does not support boot area download!"` | Download path, BLE only |
| `"BOOT  + USER area download in 2-wires mode!"` | BLE + 2-wire |
| `"only WCH-LinkE"` | SDI printf gating |
| `"Only RISC-V mode supports this feature!"` | SDI printf / 1-wire path |
| `"Failed to set IAP-Mode... Err:%d"` | SetIAPMode return |
| `"The current chip does not support 2-wires serial debug mode!"` | SetSDLineMode |
| `"This debugger does not support switching 1 or 2 wires"` | SetSDLineMode |

可在 `E-strings.txt` grep 验证。

## 8. 待办 (verify gap)

- [ ] 反汇编 `SetChipType`/`SetChipType_DAP` 拿 chip-id ↔ idx 表
- [ ] 反汇编 `SetMCUConfigValue` 拿 option-address 表
- [ ] 反汇编 `EncryptData`/`DisEncrypt` 提取加解密算法
- [ ] USB 抓包确认 cmd 总长 (cmd + args + 校验)
- [ ] 找 option-byte default mask per family (Option 默认 bit) — datasheet 对照
- [ ] `0xee020d81` SetSDIPrn vs SetSDLineMode 的 data payload 区分
