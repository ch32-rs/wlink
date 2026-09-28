# WCH-LinkUtility 逆向 — 索引

> 本文档是这个目录下所有交叉验证证据的总入口。
> 目的是给 `wlink` (这个仓库) 提供一个 "WCH 官方工具这样做" 的对照本，便于随时核查我们
> 没覆盖到的流程、Configuration 位处理、错误处理等。

## 样本与产物清单

| 路径 | 用途 | 大小 |
|---|---|---|
| `reverse-skill/WCH-LinkUtility.exe` | GUI 前端 | 3.0 MB |
| `wlink/McuCompilerDll.dll` | **协议/烧录引擎 (核心)** | 1.7 MB |
| `wlink/reversed/` | 全部证据产物 | — |
| `wlink/docs/reversed/` | 人类可读的分析文档 | — |

这两份二进制是同一版（2026-06-24 build, version v69.x, PDB 路径 `J:\1.独立项目\RISC-V编译器项目\...\McuCompilerDll_Mul_20260610\`）。  
注：`McuCompilerDll.dll` 在 GUI 同目录由 GUI 通过 import table 静态链接进来；它是 **32-bit** MFC DLL。**它需要旁边的 `WCHLinkDll.dll`（或退回 `CH375Dll.dll`）作 HID 传输**，以及 `libusb-1.0.dll` 作 DAP 传输。

## 文档目录

| # | 文档 | 内容 |
|---|---|---|
| 01 | [01_overview.md](01_overview.md) | 整体程序结构 + 三种工作模式 + GUI→DLL 调用图 |
| 02 | [02_transport.md](02_transport.md) | USB HID / libusb 传输层；函数指针表；CH375 hook 位置 |
| 03 | [03_command_table.md](03_command_table.md) | 完整 HID/DAP **命令字节** ↔ 函数 ↔ GPU 操作 |
| 04 | [04_chip_families.md](04_chip_families.md) | 芯片族 ↔ 模式 ↔ 命令族 选型表 |
| 05 | [05_option_bytes.md](05_option_bytes.md) | Read-Pro / QE / SDI / RST / IAP 配置位各 bit 的处理 |
| 06 | [06_flow_download.md](06_flow_download.md) | 完整下载 Flash 流程步骤 + sanity check 文案 |
| 07 | [07_flow_iap.md](07_flow_iap.md) | IAP / USB bootloader 模式切换链 |
| 08 | [08_wlink_gap_analysis.md](08_wlink_gap_analysis.md) | **wlink 没覆盖到的部分** —— 我们的 TODO |
| 09 | [09_cross_validation.md](09_cross_validation.md) | 全部证据的 cross-reference 矩阵 — 查询入口 |
| 10 | [10_chip_id_table.md](10_chip_id_table.md) | **完整 chip_id ↔ chip 名 ↔ capability** 映射表 (GUI-level 36 个名字 / DLL-level 33 个 chip_id) |
| 11 | [11_wlink_vs_dll_chipid.md](11_wlink_vs_dll_chipid.md) | **wlink ↔ DLL 完整对照** (22/24 一致,5 个 wlink-only,9 个 WCH-only,magic = chips.rs chip_id) |
| 12 | [12_notes_changelog.md](12_notes_changelog.md) | **活页笔记** — 后续随时 push 新发现到这里,按时间倒序,带证据位置 |
| 13 | [13_gui_fkey_handlers.md](13_gui_fkey_handlers.md) | GUI F1..F12 快捷键完整调用链 → DLL API → cmd dword (历史文档,§9 see 14) |
| 14 | [14_cmd_complete_reference.md](14_cmd_complete_reference.md) | **cmd byte 最终参考 (SSOT)** — 整合 03 + 13 + helper 反汇编 |
| 15 | [15_flash_op_blobs.md](15_flash_op_blobs.md) | **flash_op blob 完整提取** — 22 blob 全部从 `fcn.10003810` 跳转表抽出,7 个 RiscvChip variant 解锁 |
| —  | [BACKLOG.md](BACKLOG.md) | **待办清单** — Part A 反编译待办 / Part B wlink 仓库缺陷待补充 |

## 证据文件（`wlink/reversed/`）

`E-*` 是 rabin2 输出（可以重跑），`F-*` 是反汇编切片，`T-*` 是我手工整理的表。

| 文件 | 来源命令 | 含义 |
|---|---|---|
| `E-imports.txt` | `rabin2 -i WCH-LinkUtility.exe` | GUI 855 个导入 |
| `E-imports-compilerdll.txt` | 同上但只剩 McuCompiler_* | GUI 看到的 72 个 DLL 入口 |
| `E-strings.txt` / `E-strings-ascii.txt` | `rabin2 -zz / -z` | GUI 全部字符串，26k |
| `E-dll-exports.txt` | `rabin2 -E McuCompilerDll.dll` | DLL **105 个**导出符号 |
| `E-dll-imports.txt` | `rabin2 -i` | DLL 看到的系统调用；证明 libusb-1.0.dll 在传输 |
| `E-dll-strings.txt` | `rabin2 -zz` | DLL 全部字符串（基本是 MFC 框架噪音） |
| `E-dll-sections.txt` | `rabin2 -S` | DLL 5 个区段（.text / .rdata / .data / .rsrc / .reloc） |
| `F-dll-all-funcs.txt` | r2 batch (`_batch.r2`) | DLL 全部 105 个导出函数的反汇编 |
| `F-dll-helpers.txt` | r2 (`_internal.r2`) | DLL 内部 helper（包括 CH375 加载器 fcn.1000a850） |
| `F-key-funcs.txt` | awk 切片 | 关键 12 个协议的精剪 |
| `F-chiplist-init.txt` | r2 | GUI fcn.0040d180 — 芯片族 list 喂 combo |
| `F-optionbytes-*` | r2 切片 | GUI option-bytes 显示路径 |
| `T-table-func-ptrs.md` | 手工 | DLL transport-layer 函数指针布局表 |
| `T-chip-id-table.txt` | 手工 | chip_id ↔ name ↔ flash size ↔ SDI 综合表 |
| `F-cn.txt` | r2 (`fcn.0040aee0`) | GUI name→chip_id strcmp chain (1275 行) |
| `F-chip-lookup.txt` | r2 (`fcn.100015a0`) | DLL chip_id→capability (916 行) |
| `_dl-cases.txt` | awk | 33 个 chip_id 的 capability 输出 |
| `_cmd-bytes.txt` / `_cmd-func-map.txt` | awk 提取 | DLL 命令字节魔数 + 调用点 |

## 关键发现速览

1. **三模式架构**：每条 GUI 操作有三种实现（无后缀 = RV/HID、`_DAP` = ARM/WinUSB、`_Ex` = BLE RV 扩展）。
2. **真正的传输是 CH375**：DLL 没用 Windows 原生 USB HID 类，而是加载 `WCHLinkDll.dll`（fallback `CH375Dll.dll`），GetProcAddress 5 个 CH375* 符号。**`wlink` 走 libusb / 原生 USB，需要重实现这一层 ↔ 08 章**。
3. **DAP 直连 libusb**：`_DAP` 函数 import `libusb_bulk_transfer` / `libusb_get_device_list`。
4. **VID:PID = 1A86:8010** 是 RV/HID 模式的 link 描述符。
5. **命令字节表已导出**：每个核心 McuCompiler_* 函数都构造 4-8 字节 `0xXXXXXXXX81 / 0xfXXX` 命令字，详见 [03_command_table.md](03_command_table.md)。
6. **未被 wlink 覆盖** 的高级语义（QE mem type, SDI Printf, ARM 链接等）详见 [08_wlink_gap_analysis.md](08_wlink_gap_analysis.md)。
7. **RiscvChip ≈ WCH chip_id** — wlink 的 `RiscvChip` enum 与 WCH-LinkUtility 的 family code 100% 同源 ([11](11_wlink_vs_dll_chipid.md))

## 后续维护

- **新发现**: 追加到 [12_notes_changelog.md](12_notes_changelog.md),时间倒序,带证据位置
- **不要重写主章节** — 主章节只放稳定结论;变动内容 (TODO/新版 chip/未证实 推测) 都进 12 章
- **TODO 跟踪**: 12 章末尾按主题 (A/B/C/D/E/F/G) 维护,做完一条勾一条

## 最后

我**没有**修改任何源代码。所有 `.txt` / `T-*` 都是 r2/rabin2 离线分析产物，删除 `wlink/reversed/` 即可完全清理。
