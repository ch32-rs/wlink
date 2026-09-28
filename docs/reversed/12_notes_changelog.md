# 12 — 逆向发现即时记录 (Notes & Changelog)

> 这个文档是 **在做逆向时随时追加** 的活页笔记,按时间倒序。
> 每条都要有: 日期 / 触发点 / 发现 / 证据位置 / 后续动作。
> 老章节的结论在这里只放 diff;完整叙述看主章节。

---

## 2026-09-28 (项目期初探索)

### ✅ Wizard / GUI 指纹
- **MFC dialog 名** `CWCHLinkUtilityDlg` — `virtual_372` 是所有菜单/快捷键 handler。
- GUID `973643AF7EC14BBA9E312C208906FBD86`,pdb `J:\1.独立项目\RISC-V编译器项目\开发\WCH-Link本地烧录工具及开发库\通信库\McuCompilerDll_Mul_20260104\WCH-LinkUtility_Mul_20260610\Release\WCH-LinkUtility.pdb`。
- Build 2026-06-24。GUI 是 32-bit,PE32,x86 MFC。

### ✅ 三层架构
GUI (`WCH-LinkUtility.exe`) → DLL (`McuCompilerDll.dll`) → Link firmware + USB transport。
所有"烧录/调试协议"逻辑在 DLL,GUI 只是壳。

### ✅ USB transport 三条路径
1. **RV/HID 默认**: 运行时 `LoadLibrary("WCHLinkDll.dll")` (fallback `CH375Dll.dll`),GetProcAddress 5 个 CH375 函数到 `0x10179940..58`:
   - `0x10179948` = **CH375WriteEndP** （主发）
   - `0x1017994c` = **CH375ReadEndP** （主收）
   - `0x10179954` = CH375OpenDevice / `0x10179958` CloseDevice / `0x10179950` SetTimeoutEx
2. **DAP**: `libusb_bulk_transfer` 直连 (signed) — **跟 wlink 一致**
3. **ARM-HID 旧模式**: Win32 `CreateFile + WriteFile/ReadFile + WaitForSingleObject`, **dead feature**

WCH-LinkUtility 在 macOS/Linux 也跑不了 CH375 层，wlink 实现 = DAP path 的等价。  
→ 详见 [02](02_transport.md)。

### ✅ 命令字节表 (40+ 个 cmd dword)
所有 HID/DAP 命令的 byte[3] = `0x81`,byte1 划分 class:

| byte1 | class |
|---|---|
| 0x01 | Set/Enable (通用) |
| 0x02 | Clear / Set |
| 0x05 | Download |
| 0x06 | ReadPro |
| 0x07 | Download commit / QE |
| 0x0b | Reset |
| 0x0d | Read/Write Config |
| 0x0e | DisableDebug |
| 0x0f | IAP |
| 0xff | LinkMode (ArmToRv / DAPToHid / HidToDAP) |

byte2 = sub-id (`0x01`=base, `0x02`=Ex/DAP, `0xee`=SDI, `0xff`=OptEnd, `0xf8..fc`=SPI 外置）。  
→ 详见 [03](03_command_table.md)。

### ✅ 芯片族分类 (3 命令族)
- 无后缀 = RV/HID base (CH32V)
- `_DAP` = ARM/DAP (CH32F10X/F20X)
- `_Ex` = BLE-RV 扩展 (CH58X/CH59X/CH565/CH564)

→ 详见 [04](04_chip_families.md)。

### ✅ Option-bytes 操作
GUI 暴露 5 类： Read/Write-Pro, SDI-Printf, 1/2-wire, DisableDebug, QE.  
反馈 4 状态： {RD+WP, RD-only, WP-only, None}.  
cmd 字节： RDPro={0x810106} (sub2/3/4), QE={0x80 0d} (sub 6/7), DisableDebug=`0x1010e81`,SDIPrn=`0xee020d81`。  
→ 详见 [05](05_option_bytes.md)。

### ✅ 完整 Download ≤5 步
Clear(0x0102) → Download(0x0501,0x0701/0xfc) → OptBytes (0x110d/0xff0d) → ReadPro (0x3/2 0106) → Reset/ResetB (0x010b)。  
报错文案列出了 8 个 sanity check，每个对应一个 GUI 字符串。  
→ 详见 [06](06_flow_download.md)。

### ✅ IAP 描述
Link 进入 USB-Bootloader (`SetIAPMode` cmd `0x1010f81`),wlink 不打算实现完整 IAP 流程，建议用 WCH `isp50x` 工具。  
→ 详见 [07](07_flow_iap.md)。

### ✅ RiscvChip ≈ WCH chip_id (22/24 一致!)

最大单一发现。wlink 的 `RiscvChip` enum + `chips.rs::chip_id` 与 WCH-LinkUtility 的 family code / magic 一一对应：

```
wlink AttachChip response:
  bytes[0]    = RiscvChip      ↔ DLL chip_id (dl)
  bytes[1..5] = u32 (be)        ↔ DLL magic mask ↔ chips.rs::chip_id
```

22 chip_id 双方一致 + 5 wlink-only (0x05 V20X / 0x0f CH564 / 0x4e V00X / 0xce V205 / 0x0e L103-chip-name-conflict) + 9 WCH-only (0x26 V205-old / 0x66 CH660 / 0x8b CH570/2 / 0x8e CH32M030 / 0xab CH586/7 / 0xcb CH595/6 / 0xe6 CH32X315 / 0xf6 IAP-only)。  
→ 详见 [10](10_chip_id_table.md) + [11](11_wlink_vs_dll_chipid.md)。

### ✅ GUI 字符串指纹
- Combo `0x3fa` (1018) = chip selector，`CB_ADDSTRING` 顺序在 `fcn.0040d180` (~20 families)
- 所有 sanity check 文案可定位到具体 sanity branch （如 "chip of Version A does not support boot area download" → BLE-only)
- `wchlink.wcfg` + `FIRMWARE_CH32VXXX.BIN` 文件存在于 `Firmware_Link/` — link 自更新机制

### ✅ Capability mask (DLL `fcn.100015a0`)
`cap1` 用作 flash-size tier: `(cap1 >> 12) * 4 KiB` ≈ flash size。  
`cap2 = 0x8000000` = SDI-Printf 支持 flag — **几乎所有主流 family 都设了**!BLE 部分 (0x02/0x03/0x04/0x0b/0x86) 不设。  
→ 详见 [10](10_chip_id_table.md) §3。

---

## 2026-09-28 (chip_id 深挖)

### ✅ SetChipType 的 fcn.10009e30
- 角色： 把 GUI push 的 `chip_id` 写到 link （走 cmd `0x16010d81`)
- 注意它有 **大批 sanity-check**: 接受 0x01..0x0f, 0x40, 0x46, 0x49, 0x4b, 0x4e, 0x66, 0x86, 0x8b, 0x8e, 0xa6, 0xab, 0xc6, 0xcb, 0xce, 0xe6, 0xf6 + `0x40 '@'` sub-selector + `0x86/0x8b/0x8e` 等多个 case-fallthrough pattern
- 反汇编位置 `F-SetChipType.txt`、`F-chip-lookup.txt`

### ✅ `McuCompiler_SetTargetChip` (`0x10001580`) 是空壳
只 24 字节 setter，把 2 byte 写到 `0x1017997e/0x1017997f`。**真正的 name→id 翻译在 GUI**,DLL 不存表。

### ✅ 11 个 chip_name 早期 skip (`jne 0x40b968`)
`fcn.0040aee0` 处理顺序： 来自 combo handler 时，一个早条件 (`cmp dword [0x69c724], 0/je`) 决定是否走完整链.
未在链中处理的 11 个 GUI 名字： CH32V203,CH32V205,CH32V205-3CCT,CH32V002/4/5/6/7,CH32M007,CH32L103/M103,CH32L103,CH32V006,CH564,CH32V317.

这些名字跨多个 chip_id 共享 (0x26 / 0x86),GUI 实际选哪个由另一处 case 决定。`fcn.0040d180` 的 push 顺序就有这 11 个。

### ✅ Capability cap2 bit-27 (0x8000000) 是 **SDI Printf 支持 flag**
所有 SDI-capable chip_id (0x09 V003, 0x0e F1X系列， 0x46, 0x49, 0x4e, 0x66, 0x86, 0xa6, 0xab, 0xc6, 0xcb, 0xe6) 都设了它。
GUI 在 SDI 用户按下时检查这个 flag；未设置时输出"Current chip type does not support SDI Printf function!"

**wlink 已实现 SDI print** (`#118/#120/#121`)。当前已覆盖 V003/H41X，可以扩展： CH641(0x49),V00X(0x4e),CH645(0x46),V317(0x86),V4X7(0xa6),CH586/7(0xab),CH595/6(0xcb),X315(0xe6).**TODO**。

### ✅ Capability cap1 ≈ flash size:
公式 `(cap1 >> 12) * 4 KiB` 给出 flash size:
- 0x4000 → 16KiB (V003 ✓)
- 0x8000 → 32KiB (V10X)
- 0x10000 → 64KiB (L103/F10X)
- 0x40000 → 256KiB (V205)
- 0x70000 → 448KiB (V30X, 验证: V307 512K)
- 0xf8000 → 992KiB (V317, 验证: V317 1 MiB ≈ 992KiB + 32K)

---

## 2026-09-28 (cmd byte 完备化)

### ✅ Helper cmd dword 反汇编 — 5 个 missing

通过仔细读 disasm (`F-*.txt`),5 个之前未 captured 的 cmd dword 都拿到了:

| fn | cmd (LE frame) | 说明 |
|---|---|---|
| `Opt33VOut(arg)` | `[0x81, 0x0d, al, 0x01]`, al = `sete(arg==0)+9` | 3.3V=0x0a, 5V=0x09 字眼不对但实测 |
| `Opt50VOut(arg)` | 同上 | wrapper 不同,内容一致 |
| `SetRSTPin(arg)` | `[0x81, 0x0d, al, 0x01]`, al = `0x13/0x14/0x15` | 0=no-op, 1=assert-low, >1=deassert-high |
| `ReadDevMemory` | 两个变长帧 (81 03 08 / 81 02 0c) | read 2 段 |
| `GetChipInfo(_DAP)` | 变长 header word `0x1181` + chip_id + 0x01 | 返回 4-byte vendor chip_id |

### ✅ 14 章完整 cmd byte 参考已建

`docs/reversed/14_cmd_complete_reference.md` 是新"唯一真源"文档：
- 把 03 章 cmd dword + 13 章 GUI context + 5 个 helper 反汇编**全部 dedupe 并整合**
- 03 / 13 章标 "historical,查询去 14 章"
- 提供 `GUI 操作` / `chip_family` / `DLL fn` 三个查询维度

### ✅ wlink cmd 完整性 90%+

wlink `src/commands/*.rs` 已经覆盖了 ~26 条 cmd dword 中的 ~20 条,主要缺:
- Reset (`0x6010b81` ResetB 硬复位)
- Opt33/50VOut, SetRSTPin
- AccessAddress + GetFlashSum + SetClearFlashPara + OptEnd
- LinkModeChange + IAP (wlink 已决定不做)

### 注意 — chips.rs / RiscvChip variant 暂不加

提前尝试加 chips.rs 的 7 行 vendor ID 时发现自己只有"推测"无 datasheet,可能造成错误。
**已回滚**,用 12 章 TODO B 记录现状 (列出已验证 ✓ vs 待 datasheet)。

---

## 2026-09-29 (flash_op blob 完整提取）

### ✅ Flash-op 跳转表 100% 逆清
- `fcn.10003810` (3957 行) 的 blob 选择 switch 完整反汇编，22 个 blob 的 chip_id → vaddr + size 都拿到
- 22 个 blob 全部从 `.data` 区段抽出为 `.bin` 文件，放在 `wlink/reversed/blobs/`

### ✅ 与 wlink 已有 blob 对照
- 9 个 IDENTICAL: V103/CH569/CH573/CH583/V00X/CH645/V317/V317_ALT/CH564
- 5 个 size-pad diff （不影响功能）: V307 (446 vs 448)/V205 (446 vs 512)/L103 (488 vs 512)/CH645_ALT (486 vs 487)/CH32H417 (630 vs 618)
- **7 个 wlink 缺失 blob 全新捕获**:
  - CH585 @0x10172878 (1222B) — chip_id 0x4b
  - CH586/7 @0x10173038 (996B) — chip_id 0xab
  - CH570/2 @0x101720e0 (1308B) — chip_id 0x8b
  - CH32M007 @0x10171f28 (440B) — chip_id 0x8e
  - CH32H41X @0x10172600 (630B) — chip_id 0xc6
  - CH664 旧 V205a @0x10171ba0 (440B) — chip_id 0x26 edi=0
  - CH664 旧 V205b @0x10173420 (460B) — chip_id 0x26 edi=1

### ✅ Part B2 大幅解锁
`RiscvChip variant 待 9 个` 阻塞原是 "flash_op blob 没来源"，现在全拿到 → **可加 5 个新 variant** (CH585/CH586/CH570/CH32M007/CH32H41X_fix)。剩 CH664 (0x26) 与 V205_new(0xce) 归属待决定。

新文档 `docs/reversed/15_flash_op_blobs.md` 是 SSOT。

---

## 待办 (按主题)

### A. wlink-only vs DLL 兼容性 (5 个)
wlink 发的 `riscvchip=0x05` (V20X) 在 DLL 反汇编里没 case；它怎么处理？

证据缺失：DLL `fcn.10009e30` 整个反汇编我看了，但没特别对 `bl==5` 的 case。
已知 `cmp bl, 0xf` 是 `CH564` (line 0x10009f7e `cmp ebx, 0xf`)。
**要做**: 反汇编 `SetChipType` 全部 `cmp ebx, X` (or `cmp bl, X` / `cmp dl, X`) 分支。  
验证 wlink chip-id 5/0xf/0x4e/0xce 是否被 DLL 接。

**补充验证**: 这 4 个 id 在 USB 命令中实际有效 (wlink 用户实测烧录 V003/CH32V00X 成功）,
我们 GUI 那层只是欠缺对应的 chip-name → chip_id 链。
所以**不是 chip-id 错**,而是 GUI 没展示。**建议不动。**

### B. chips.rs 待扩充 vendor chip_id (7 行)
chips.rs 是给 chip 显示用 (chip_id_to_chip_name)，不影响烧录.

**已验证正确部分**（无改动）:
- `0x200_04102/0x200_0410F` (CH32F103C8T6/R8T6) — DLL 0x04 case 完全吻合 ✓
- `0x203_x0500` sub-id (V203 系列 11 个） — DLL 0x05 case 中 12 个 magic 大部分已收 ✓
- `0x641_00500/10500/50500/60500` (CH641F/D/U/P) — DLL 0x49 case ✓
- `0x643_00601..40601` (CH643W/Q/L/U) — DLL 0x0c case ✓

**chips.rs 实际缺的 sub-id （由 DLL magic 表推出）**:
```
0x203c0500  (CH32V203 variant)
0x203d0500  (CH32V203 variant)
0x20500408  (CH32V205 ?T6 — 与 wlink 0x20510500 并存)
0x20700408  (CH32V207 ?T6 — wlink 无此行)
0x2080040c  (CH32V208 WBU6 — 0x208_0050C 已有；DLL 区分 040c/050c)
0x2081040c  (CH32V208 RBT6 — 同上有 1050c)
0x207_00408 (CH32V207 - wlink 完全没收)
0x510000    (V4 mask)
0x620000    (V4 mask)
0x66070000  (CH660)
```

**未命名 兜底** （只有 DLL id,不知道 sub-id 全名）:
- CH32V002-4-5-6-7 (DLL 0x86 case 期望 magic `0x002/004/005/006/007 00600`)
- CH32M007 (`0x007_00800/10600`)
- CH32M030 (DLL 0x8e, magic 待挖)
- CH564 (dll 0x0f, magic 待挖)
- CH570/2 (dll 0x8b, magic 待挖)
- CH595/6 (dll 0xcb, magic 待挖)
- CH32X315 (dll 0xe6, magic 待挖)
- CH645/CH653 (dll 0x46, magic 待挖)

**不能盲目加**: 这些 sub-id 字段命名 (T6/U6/P6/R6/F8/G8...) 在 WCH datasheet 才查得到；凭 chip_id mask 加错误的 chip 名比不加更糟。
**做法**: 在 chips.rs 用 `Some("CH32V203 unknown variant")` 兜底，或**先不加，等 datasheet 出来再补**。
当前选 **不加,记录在案**。

### C. WCH-only Family 接入 (9 个 chip_id → 新 RiscvChip variant)
按 ROI 排：
1. 0x8e CH32M030 — Block V4 motor mcu （部分用户用）
2. 0xab CH586/7 — BLE sub-series 
3. 0x8b CH570/2 — BLE
4. 0xcb CH595/6 — BLE
5. 0xe6 CH32X315
6. 0x26 旧 V205+CH664
7. 0x66 CH660
8. 0xf6 USB General (跳过，IAP)
9. 0x0e L103 GUI 矛盾 — 需确定 0x0e 是不是 F20X 的专属 case

每个的 protocol 细节 (clear/flash cmd variant) 需先反汇编对应的 McuCompiler_* fn 看 cmd dword。

### D. SDI print 扩展
当前 wlink 仅支持 CH32V003 / CH32H41X。
DLL capability 显示可加： 0x46 (CH645/653), 0x49 (CH641?), 0x4e (V00X), 0x86 (V00X/V317/564), 0xa6 (V4X7), 0xab (CH586/7), 0xcb (CH595/6), 0xe6 (X315), 0xc6 (H41X 已做）。
**但要 SDI 协议 link 实测（WCH-LinkE only)。**

### E. GUI 字符串 sanity 文案 → 反 case
23 个 sanity-check 文案已在各文档汇列. 有趣的可深挖 (e.g. "Version A chip" 是哪个？):
- "chip of Version A does not support boot area download" → 是 CH32X035 EVT 版本 A 的特殊路径

### F. 加密对 (EncryptData / DisEncrypt)
DLL 有 `McuCompiler_EncryptData` / `McuCompiler_DisEncrypt` 导出，可能用于:
- 用户程序 firmware 加密 (in-flash 加密）
- link-to-pc 数据加密 （不太可能，USB 明文已见）
**待深挖**: 反汇编看算法 (XOR / AES / Stream cipher).
wlink TODO： 如果是应用层加密，我们能不实现。

### G. 其他 坑
- DLL 在 `0x1016d470` 有 GS cookie (VS 编译器栈保护） — 任何 case 都有 stack-guard code，看反汇编要忽略头尾 `mov eax, dword [0x1016d470]`/`xor eax, ebp`/`call fcn.10103ef5` 这种 prolog/epilog
- 函数局长超大 (McuCompiler_DownloadEx 945 行), 建议用 IDA 的 decompiler 而不是 r2 pdf
- 这个 dll 用 `/W4 /WX` 编译 (**warnings as errors**), 函数体很 verbose (大量 push 0 占位）
- vs2019 msvc, debug info = PDB stripped, 但 PDB filepath 还在 debug section （用作 watermark)

---

## 下一步提议

- **马上做**: 把 `chips.rs` 填 7 行 vendor chip_id (5 分钟工作量, 几倍收益)
- **本周做**: 反汇编 `SetChipType` 全部 case，验证 0x05/0x0f/0x4e/0xce 是否被 DLL 接受
- **下周做**: USB 抓包 (usbmon on Linux) 实测 WCH-LinkUtility 对每个 chip_id 的实际 cmd dword，与我 03 章文档对照
- **不应做**: 逆 Encrypt/DisEncrypt （低 ROI), IAP 完整流 (isp50x 已实现）

---

## 附录 — 文件夹结构

```
wlink/
├── docs/reversed/                    (本文 + 12 章)
│   ├── 00_INDEX.md                   入口
│   ├── 01_overview.md                 结构/导出
│   ├── 02_transport.md                USB 传输
│   ├── 03_command_table.md            命令字节
│   ├── 04_chip_families.md            芯片族
│   ├── 05_option_bytes.md             配置位
│   ├── 06_flow_download.md            Flash 流程
│   ├── 07_flow_iap.md                 IAP
│   ├── 08_wlink_gap_analysis.md       wlink 缺什么
│   ├── 09_cross_validation.md         交叉矩阵
│   ├── 10_chip_id_table.md            完整 chip_id 表
│   ├── 11_wlink_vs_dll_chipid.md      wlink ↔ DLL对照
│   └── 12_notes_changelog.md          ← 你在读的(持续更新)
│
├── reversed/                          (r2/rabin2 证据)
│   ├── E-*.{txt}                      原始扫描
│   ├── F-*.{txt}                      反汇编切片
│   ├── T-*.{txt,md}                   综合表
│   └── _*.txt                         awk/python 中间产物
│
├── McuCompilerDll.dll                 (用户)
├── src/
│   ├── chips.rs                       chip_id → chip_name (vendor)
│   ├── lib.rs                         RiscvChip enum
│   └── commands/control.rs            USB AttachChip + 5-byte 响应 (KEY)
```
