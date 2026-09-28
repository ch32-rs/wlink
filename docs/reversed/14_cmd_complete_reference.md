# 14 — USB 命令字节完整参考 (Canonical Reference)

> **本文档是 WCH-Link 命令字节的最终参考**。
> 整合自 03 章 (`_cmd-func-map.txt`) + 13 章 (GUI context) + 5 个 helper 反汇编补充。
> **03 / 13 章 后续仅作历史文档保留**,如需查询**以此为准**。

## 1. 命令帧结构

**Header 2-byte (LE word) + sub_id 1-byte + args**

```
byte[0]     = 0x81                    (WCH magic,恒为 0x81)
byte[1]     = sub-cmd-class           (0x01..0xff)
byte[2]     = sub-id                  (sub-cmd 内 sub-id)
byte[3..]   = args (length varies)
```

也有 **变长命令**: header word 直接 `0x81` 后跟 cmd-byte(像 `0x1181` = LE of [0x81, 0x11]):
- `0x1181` = cmd-class 0x11 (GetChipInfo), 后跟 chip_id (1 byte) + arg (1 byte)
- `0x381` = cmd-class 0x03 (ReadDevMemory), 后跟 8 byte args

sub-cmd-class 划分:

| byte[1] | class | 用途 |
|---|---|---|
| 0x01 | Set/Enable | 通用配置 |
| 0x02 | Clear / Reset | 清空 / 复位 |
| 0x03 | Read | 读 flash/memory |
| 0x05 | Download | 写 flash |
| 0x06 | ReadPro | 读保护位 |
| 0x0b | System Control | Reset 等 |
| 0x0d | Config | 配置 / 状态读取 |
| 0x0e | Debug | 调试口 |
| 0x0f | IAP | Bootloader |
| 0x11 | ChipInfo | (变长 header word) |
| 0xff | LinkMode | Link 自身模式切换 |

## 2. 全部 cmd dword (header-4-byte form)

按 chip_id / GUI 操作 / DLL fn 三个维度查询。

### 2.1 主 26 个 (GUI 常用)

| cmd dword (LE) | 字节 (81 sub-class sub-id) | fn | GUI 操作 | chip_family support |
|---|---|---|---|---|
| `0x1010281` | 81 02 01 01 | `Clear` / `Clear_DAP` | (GUI: hidden) | all |
| `0x2010281` | 81 02 02 01 | `Download_DAP` / `Download_DAPEx` | (F10 Program) | DAP |
| `0x5010281` | 81 02 05 01 | `Download`/`DownloadEx`/`DownloadExternal` | (F10 Program) | RV/HID |
| `0x7010281` | 81 02 07 01 | (commit variant) | 写入后续 commit | RV/HID |
| `0x1010681` | 81 06 01 01 | `CheckReadPro_DAP` | (F5 Query RP) | DAP |
| `0x2010681` | 81 06 02 01 | `DisableReadPro` | (F7 Disable RP) | RV/HID base |
| `0x3010681` | 81 06 03 01 | `EnableReadPro` | (F6 Enable RP) | RV/HID base |
| `0x1010b81` | 81 0b 01 01 | `Reset` | (F12 Reset - soft) | RV/HID base |
| `0x6010b81` | 81 0b 06 01 | `ResetB` | (F12 Reset - hard) | RV/HID base |
| `0x2010d81` | 81 0d 02 01 | `Clear` / `SetChipType` / `GetLinkedMCUID` | (combo change) | RV/HID base |
| `0x4010d81` | 81 0d 04 01 | `GetROMRAM` | (BLE only) | BLE-RV |
| `0x6010d81` | 81 0d 06 01 | `CheckQE` / `CheckQE_DAP` | (GUI hidden) | 全部 |
| `0x7010d81` | 81 0d 07 01 | `EnableQE` / `EnableQE_DAP` | (GUI hidden) | 全部 |
| `0x16010d81` | 81 0d 16 01 | `SetChipType` | (combo change) | RV/HID base |
| `0x17010d81` | 81 0d 17 01 | `GetROMRAM` (alt) | (BLE) | BLE-RV |
| `0x11050d81` | 81 0d 11 05 | `SetAccessAddress` | (GUI hidden) | 全部 |
| `0x12010d81` | 81 0d 12 01 | `GetAccessAddress` | (GUI hidden) | 全部 |
| `0x18020d81` | 81 0d 18 02 | `SetROMRAM` | (BLE) | BLE-RV |
| `0x5020d81` | 81 0d 50 02 | `SetROMRAM` (idx5) | (BLE) | BLE-RV |
| `0x8020d81` | 81 0d 08 02 | `ClearCodeFlash` | (F9 Erase) | RV/HID base |
| `0xf010d81` | 81 0d f0 01 | `ClearCodeFlashB` | (F9 Erase alt) | RV/HID base |
| `0xee020d81` | 81 0d ee 02 | `SetSDIPrn` / `SetSDLineMode` | (1-wire/SDI printf) | RV/HID SDI |
| `0xff010d81` | 81 0d ff 01 | `OptEnd` | (隐含) | 全部 |
| `0xfc020d81` | 81 0d fc 02 | `SetClearFlashPara` | (GUI hidden) | 全部 |
| `0xfd050d81` | 81 0d fd 05 | `GetFlashSum` | (GUI hidden) | 全部 |
| `0x1010e81` | 81 0e 01 01 | `DisableDebug` | (GUI: 关调试口) | 全部 |
| `0x1010f81` | 81 0f 01 01 | `SetIAPMode` / `ChangeHIDToIAP` / `SetDeviceToIAP_DAP` | (IAP 进入) | 全部 |
| `0x2010f81` | 81 0f 02 01 | `GetIAPType` / `GetDeviceMode(_DAP)` | (GUI hidden) | 全部 |
| `0x5201ff81` | 81 ff 52 01 | `ChangeArmToRvLink` / `SetDeviceToRV_DAP` / `SearchAndSetArmLink` | (Link mode=RV) | link mode |
| `0x6301ff81` | 81 ff 63 01 | `ChangeDAPToHidLink` | (Link mode=HID) | link mode |
| `0x7401ff81` | 81 ff 74 01 | `ChangeHIDToDAPLink` | (Link mode=DAP) | link mode |
| `0x3080681` | 81 06 08 03 | `EnableReadPro_Ex` / `EnableReadPro_DAP` | (F6 BLE-RV/DAP) | BLE / DAP |
| `0x2080681` | 81 06 08 02 | `DisableReadPro_Ex` / `DisableReadPro_DAP` | (F7 BLE-RV/DAP) | BLE / DAP |
| `0x4010681` | 81 06 04 01 | `CheckReadPro_Ex` / `CheckReadPro_Ex_DAP` | (F5 BLE-RV/DAP) | BLE / DAP |
| `0xf8040d81` | 81 0d f8 04 | `DownloadExternal` (cmd F8) | (外置 SPI BLE) | BLE |
| `0xf9040d81` | 81 0d f9 04 | `DownloadExternal` (cmd F9) | (外置 SPI BLE) | BLE |
| `0xfa040d81` | 81 0d fa 04 | `DownloadExternal` (cmd FA) | (外置 SPI BLE) | BLE |
| `0xfb040d81` | 81 0d fb 04 | `DownloadExternal` (cmd FB) | (外置 SPI BLE) | BLE |

### 2.2 变长命令 (header-word + args) — NEW

| header word (LE) | args 长度 | fn | 说明 |
|---|---|---|---|
| `0x1181` | 1 byte chip_id (bl) + 1 byte (0x01) | `GetChipInfo(_DAP)` | 返回 4-byte vendor chip_id + args |
| `0x81 0x03 0x08` | `chip_id + args` | `ReadDevMemory` (cmd 1) | 读 flash数据,长 0xb |
| `0x81 0x02 0x0c` | args | `ReadDevMemory` (cmd 2) | read cmd 2,长 0xc |
| `0xd81 + al + 0x01` | al = `0x09/0x0a` | `Opt33VOut` / `Opt50VOut` | 3.3V=0x0a, 5V=0x09 (weird swap)|
| `0xd81 + al + 0x01` | al = `0x13/0x14/0x15` | `SetRSTPin` | 0=no-op, 1=assert (low), >1=deassert (high) |

确认 `fcn.100015a0` 在每条 cmd 后被调一次 — **说明 GUI/DLL 都依赖一条 cmd 的返回值决定后续 capability**。

## 3. GUI 操作 → cmd dword 完整链 (主 family)

### 3.1 主流程 (Download)

```
1. GUI: 选 chip                -> SetChipType (cmd 0x16010d81 + chip_id) 
2. GUI: Connect (F2)           -> OpenDevice (无 cmd) + SetTargetChip (无 cmd)
3. GUI: Query Info (F3)        -> GetChipInfo(_DAP) (cmd 0x1181 + chip_id, header-word form)
4. GUI: Execute (F4)           -> 主流程:
     - SetClearFlashPara       -> cmd 0xfc020d81
     - ClearCodeFlashB         -> cmd 0xf010d81 (or ClearCodeFlash = 0x8020d81)
     - DownloadEx              -> cmd 0x5010281 (start) + data + 0x7010281 (commit)
     - ReadDevMemory (verify)  -> cmd 0x81 03 08 (read 1) + 0x81 02 0c (read 2)
     - SetMCUConfigValue       -> 命令字节 DLL 反汇编里没显示; 直接调 write cmd (via AccessAddress)
     - OptEnd                  -> cmd 0xff010d81
     - EnableReadPro (opt)     -> cmd 0x3010681 (RV base) / 0x3080681 (BLE/DAP)
     - Update                  -> (数据同步)
     - Reset / ResetB          -> cmd 0x1010b81 / 0x6010b81
     - CloseDevice
5. GUI: F12 Reset              -> Reset 或 ResetB 直接调
6. GUI: F5 Query RDPro         -> CheckReadPro (cmd 0x1010681 base) / CheckReadPro_Ex (0x4010681 BLE) / _DAP
```

### 3.2 用户配置字段 (菜单 hidden)

| 用户操作 | fn | cmd dword |
|---|---|---|
| Choose 1-wire (SDI) | SetSDLineMode(0) | `0xee020d81` + 0 |
| Choose 2-wire (SWD) | SetSDLineMode(1) | `0xee020d81` + 1 |
| Two-line low speed on | SetTwolineLowSpeed(1) | `0xee020d81` + 1 (同 cmd) |
| Enable SDI printf | SetSDIPrn(1) | `0xee020d81` + 1 |
| Disable SDI printf | SetSDIPrn(0) | `0xee020d81` + 0 |
| Assert RST pin | SetRSTPin(1) | `0xd81 0x14 01` |
| Deassert RST pin | SetRSTPin(2) | `0xd81 0x15 01` |
| No-op RST | SetRSTPin(0) | `0xd81 0x13 01` |
| 3.3V output | Opt33VOut(0) or Opt33VOut_DAP(0) | `0xd81 0x0a 01` |
| 5V output | Opt50VOut(0) | `0xd81 0x09 01` |
| Set IAP mode (link) | SetIAPMode | `0x1010f81` |
| Disable debug | DisableDebug | `0x1010e81` |
| Enable QE | EnableQE | `0x7010d81` |
| Check QE | CheckQE | `0x6010d81` |
| Disable read-protect | DisableReadPro (_Ex/DAP) | `0x2010681` / `0x2080681` |
| Enable read-protect | EnableReadPro (_Ex/DAP) | `0x3010681` / `0x3080681` |
| Check read-protect | CheckReadPro (_Ex/DAP) | `0x1010681` / `0x4010681` |

## 4. wlink 现状

### 已实现的 cmd dword (与 DLL 一致)

```
AttachChip            81 0d 01 02    GetChipInfo
SetSpeed              81 0d 01 06    SetSpeed  
GetChipInfo           81 0d 01 04    (wlink 用 GetChipInfo::V1/V2)
QueryInfo             81 0d 01 ??    wlink 未实现完整
Erase                 81 02 01 01    Clear
Download/flash        81 02 05 01    Download
Verify (回读)          81 03 08 ...
Reset                 81 0b 01 01    Reset
Enable-ReadPro        81 06 03 01    EnableReadPro
Disable-ReadPro       81 06 02 01    DisableReadPro
Query-ReadPro         81 06 01 01    CheckReadPro
SDI printf on/off     81 0d ee 02    SetSDIPrn
enable-debug (RISKY!)                  应 = ?
disable-debug         81 0e 01 01    DisableDebug
```

### wlink 已实现的 cmd 与 WCH-LinkUtility 完全匹配 (grep 自 src/commands/mod.rs)

→ 见 `src/commands/control.rs`, `src/commands/mod.rs`. wlink 在 `RawCommand::<0x0d>` 路径下能发 0x0d-sub-cmd 系列;已经在用 0x0d ee 02 (`SDI print support commit #118`)。

### wlink 未实现的 cmd dword (gap)

```
- 0x1010b81 (Reset alt)                  wlink 重置已用 0x0d 系列
- 0x6010b81 (ResetB)                     wlink 没有 reset --hard
- 0xd81 0x0a/0x09/13/14/15 (Opt33/50VOut, SetRSTPin)
- 0x11050d81 (SetAccessAddress)          wlink 有 read 不能 access-addr
- 0x12010d81 (GetAccessAddress)          wlink 没抽象
- 0xfd050d81 (GetFlashSum)               wlink 没这个 query
- 0xfc020d81 (SetClearFlashPara)         wlink 用默认参数
- 0xff010d81 (OptEnd)                    wlink 没专用 opt-end
- LinkModeChange (0x52/0x63/0x74 ff 81)  wlink 不做 mode change
- IAP (0x1010f81)                        wlink 不做 IAP
- ReadDevMemory 变长 header              wlink 用 raw cmd 直发 81 03 ...
```

## 5. 与文档 03 / 13 的差异 (dedupe)

03 章主要列了基础 cmd-dword 类 (4-byte pattern),13 章 GUI 视角那边按用户操作分组,有**少量重叠**。
本 14 章把它们**对齐 + 补充**,作为 single source of truth。

后续如新增 cmd byte 发现,添加到本 14 章;03 / 13 章的历史记录可以暂时不动。

## 6. 验证证据

- `_cmd-func-map.txt` 4-byte cmd 提取
- `F-Opt33VOut.txt` / `F-Opt50VOut.txt` / `F-SetRSTPin.txt` — 5V/3.3V/RST cmd (header word 0xd81 + sub)
- `F-ReadDevMemory.txt` — read 变长 cmd 帧 (81 03 08 / 81 02 0c)
- `F-GetChipInfo.txt` / `F-GetChipInfo_DAP.txt` — GetChipInfo 变长 header (0x1181)
- `F-SetChipType.txt` — SetChipType cmd 0x2010d81/0x16010d81
