# 03 — 完整命令字节表 (历史文档)

> ⚠ **已被 [14_cmd_complete_reference.md](14_cmd_complete_reference.md) 取代**。
> 本章保留作历史归档,新查询去 14 章。

每个 McuCompiler_* 调用,把 4 字节命令 (little-endian dword) 通过 USB 报文头写到 link。
模式: `<LSB..MSB> = [cmd_minor][cmd_major][sub_cmd][0x81]`
所有命令的 byte[3] = `0x81` (WCH 报文 magic / 7-bit 协议标志)。

下面是按 cmd_byte2:byte1 分类的完整查询表。

数据来自 `_cmd-func-map.txt`,从全部 105 个导出函数的反汇编中 grep `mov dword [var_*], 0x...81`
采集得到;`0x100...` 地址类常量已过滤。

## 命令字 (byte1:byte0, byte3=0x81)

| byte2 | byte1:byte0 | 命令 (LE dword) | fn | 含义 |
|---|---|---|---|---|
| `0x81` | 0x01:0x06 | `0x1010681` | `CheckReadPro_DAP` | **查询读保护 (DAP)** |
| `0x81` | 0x02:0x01 | `0x1010281` | `Clear` / `Clear_DAP` | **整片擦除** |
| `0x81` | 0x02:0x01 | `0x2010281` | `Download_DAP` / `Download_DAPEx` | **DAP 下载开始** |
| `0x81` | 0x01:0x02 | `0x5010281` | `Download`/`DownloadEx`/`DownloadExternal` | **HID 下载开始 (cmd=5, sub=1)** |
| `0x81` | 0x02:0x01 | `0x7010281` | `Clear` / `Download*` | **擦除/下载 commit (cmd=7)** |
| `0x81` | 0x01:0x06 | `0x2010681` | `DisableReadPro` | **关读保护** |
| `0x81` | 0x01:0x06 | `0x3010681` | `EnableReadPro` | **开读保护** |
| `0x81` | 0x08:0x06 | `0x2080681` | `DisableReadPro_Ex` / `DisableReadPro_DAP` | **关 RDPro (Ex/DAP)**|
| `0x81` | 0x08:0x06 | `0x3080681` | `EnableReadPro_Ex` / `EnableReadPro_DAP` | **开 RDPro (Ex/DAP)**|
| `0x81` | 0x01:0x06 | `0x4010681` | `CheckReadPro_Ex` / `CheckReadPro_Ex_DAP` | **查询 RDPro (Ex)**|
| `0x81` | 0x01:0x0b | `0x1010b81` | `Reset` | **软件复位** |
| `0x81` | 0x01:0x0b | `0x6010b81` | `ResetB` | **复位-B (硬复位)** |
| `0x81` | 0x01:0x0d | `0x1010d81` | `GetArmLinkVersion` | 查 ARM link 版本 |
| `0x81` | 0x01:0x0d | `0x6010d81` | `CheckQE` / `CheckQE_DAP` | **查询 QE bit (Quad-Enable)**|
| `0x81` | 0x01:0x0d | `0x7010d81` | `EnableQE` / `EnableQE_DAP` | **写 QE bit** |
| `0x81` | 0x01:0x0d | `0x2010d81` | `Clear` / `SetChipType` / `MRSFunc_GetLinkedMCUID` | 通用"协商 + chip-id" |
| `0x81` | 0x01:0x0d | `0x4010d81` | `GetROMRAM` | **查 ROM/RAM 布局** |
| `0x81` | 0x01:0x0d | `0x11050d81` | `SetAccessAddress` / `_DAP` (5字节) | **设 access addr** |
| `0x81` | 0x01:0x0d | `0x12010d81` | `GetAccessAddress` / `_DAP` | **读 back** |
| `0x81` | 0x01:0x0d | `0x16010d81` | `SetChipType` | **设 chip id** |
| `0x81` | 0x01:0x0d | `0x17010d81` | `GetROMRAM` | 读 ROM/RAM 配置 (alt) |
| `0x81` | 0x02:0x0d | `0x18020d81` | `SetROMRAM` | **写 ROM/RAM 配置** |
| `0x81` | 0x02:0x0d | `0x5020d81` | `SetROMRAM` | (sub-id 5) |
| `0x81` | 0x02:0x0d | `0x8020d81` | `ClearCodeFlash` / `_DAP` | **擦 Code Flash** |
| `0x81` | 0x02:0x0d | `0xee020d81` | `SetSDIPrn` / `SetSDLineMode` | **SDI printf / 1-2wire 模式** |
| `0x81` | 0x02:0x0d | `0xf010d81` | `ClearCodeFlashB` / `_DAP` | **擦 Code Flash B (仓页)** |
| `0x81` | 0x04:0x0d | `0xf8040d81` | `DownloadExternal` | **写外部 flash cmd=F8** |
| `0x81` | 0x04:0x0d | `0xf9040d81` | `DownloadExternal` | F9 |
| `0x81` | 0x04:0x0d | `0xfa040d81` | `DownloadExternal` | FA |
| `0x81` | 0x04:0x0d | `0xfb040d81` | `DownloadExternal` | FB |
| `0x81` | 0x02:0x0d | `0xfc020d81` | `SetClearFlashPara` | **设擦除参数** |
| `0x81` | 0x05:0x0d | `0xfd050d81` | `GetFlashSum` | **查总 flash 大小** |
| `0x81` | 0x01:0x0d | `0xff010d81` | `OptEnd` | **Option-bytes 写完成 commit** |
| `0x81` | 0x01:0x0e | `0x1010e81` | `DisableDebug` | **关调试口** |
| `0x81` | 0x01:0x0f | `0x1010f81` | `SetIAPMode` / `SetDeviceToIAP_DAP` / `ChangeHIDToIAP` | **切 IAP 模式** |
| `0x81` | 0x02:0x0f | `0x2010f81` | `GetIAPType` / `GetDeviceMode(_DAP)` | **查 IAP/mode** |
| `0x81` | 0x01:0xff | `0x5201ff81` | `ChangeArmToRvLink` / `SearchAndSetArmLink` / `SetDeviceToRV_DAP` | **mode=RV (5)** |
| `0x81` | 0x01:0xff | `0x6301ff81` | `ChangeDAPToHidLink` | **mode=DAP→HID (6)** |
| `0x81` | 0x01:0xff | `0x7401ff81` | `ChangeHIDToDAPLink` | **mode=HID→DAP (7)** |
| - | - | `0x0803f000` | `DownloadEx` | pseudo-cmd,在 DownloadEx data path (非 `0x81`-尾) |

## 命令分组

按 byte2 (sub-op) 看:

- **byte2=0x01** — 主操作 (most commands)
- **byte2=0x02** — 扩展/带参 (Ex, DAP 版本, SDI 配置)
- **byte2=0x04** — DownloadExternal 的 4 个 sub-cmd (F8..FB)
- **byte2=0x05** — SetROMRAM/GetFlashSum (带 size 参数)

按 byte1 (cmd-class) 看:

| byte1 | class |
|---|---|
| 0x01 | Set/Enable (通用) |
| 0x02 | Clear / Set (RV/HID) |
| 0x05 | Download |
| 0x07 | Download-commit |
| 0x06 | ReadPro / QE |
| 0x0b | Reset |
| 0x0d | 读 / 写配置 |
| 0x0e | 关调试 |
| 0x0f | IAP mode |
| 0xff | Link mode change (RV/DAP/ARM 切换) |

## 与 wlink 的对应

`wlink/src/operations.rs` / `flash_op.rs` 已实现的:

| wlink 行为 | 对应命令 | 状态 |
|---|---|---|
| `attach + Clear` | 0x01 02 01 / 02 02 0d (sub 8) | ✅ |
| `flash` | 0x05 01 02 + 0x07 01 02 | ✅ |
| `Reset` | 0x01 01 0b | ✅ |
| `DisableDebug` (wlink invoke "exit") | 0x01 01 0e | ✅ |
| `Enable RDP` / `Disable RDP` | 0x03/0x02 01 06 | ✅ |
| SDI 1-wire/2-wire 选择 (内部) | 0xee 02 0d | ✅ (wlink/src/commands.rs writes) |

`wlink` 尚未覆盖的 → [08 章](08_wlink_gap_analysis.md)。

## 备注

* 命令总长 4 字节, 后跟任意 args/数据 (按 sub-cmd 决定)。
* Response: link 用同样头 + data 写回。DownloadEx 帧的 `0x0803f000` 是类似 swd 状态字。
* 解析命令字节的代码： each McuCompiler_* push 4-16 字节到 stack, 然后 `call [0x10179948]` (CH375WriteEndP),
  期望 link 同步 ack. 具体帧 bundle (header length + tail 校验) 在 DLL 内不显示,
  **要拿实际帧需要 USB 抓包**或读 WCH 官方 SDK C 头文件。
