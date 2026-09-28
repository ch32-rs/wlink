# WCH-Link 逆向 + wlink 待办清单 (Backlog)

> 本文档汇总**当前所有未做事项**，分两部分:
> - **Part A**: 与本次反编译 (`WCH-LinkUtility.exe` + `McuCompilerDll.dll`) 相关的待办
> - **Part B**: wlink 仓库本身的缺陷 / 待修复 / 待补充
>
> 每条格式：`[优先级] 任务 — 上下文 / 证据位置 / 估算 / 阻塞原因`
> 优先级： 🔴 高 / 🟡 中 / 🟢 低 / 💤 不做

---

## ✅ 已收割的核心信息 (2026-09-28 ~ 2026-09-29)

这部分**已经完成**, 不再算 TODO；信息沉淀在各文档章节。列在这里只是为总览。

### ✔ 反编译产物 (15 章文档 + 22 blob)

| 类别 | 内容 | 数据 |
|---|---|---|
| **二进制总览** | GUI (3 MB, MFC) + DLL (1.7 MB, transport 层 + 105 export) | [01_overview.md](01_overview.md) |
| **三 transport 层** | RV/HID 默认走 WCHLinkDll (CH375 5 fn ptr) / DAP 走 libusb / ARM-HID 旧模式 | [02_transport.md](02_transport.md) + `_calls.tsv` |
| **完整 cmd byte 表** | 26 4-byte cmd + 5 变长 header cmd + GUI 操作映射 | [14_cmd_complete_reference.md](14_cmd_complete_reference.md) (**SSOT**) |
| **26 个 chip_id → name → capability** | GUI `fcn.0040aee0` + DLL `fcn.100015a0` 双反汇编 | [10_chip_id_table.md](10_chip_id_table.md) |
| **wlink ↔ DLL 一一对照** | 22/24 chip_id 一致 + 5 wlink-only + 9 WCH-only | [11_wlink_vs_dll_chipid.md](11_wlink_vs_dll_chipid.md) |
| **GUI F1..F12 快捷键链路** | virtual_268 accelerator dispatch → 每 fkey 完整 DLL 调用序列 | [13_gui_fkey_handlers.md](13_gui_fkey_handlers.md) |
| **22 个 flash_op blob** | `fcn.10003810` 跳转表 100% 逆清，独立 `.bin` 抽出 | [15_flash_op_blobs.md](15_flash_op_blobs.md) + `reversed/blobs/*.bin` |

### ✔ 7 个 RiscvChip variant 解锁

| 新 chip_id | 对应 blob | 大小 | wlink 能否加 |
|---|---|---|---|
| 0x4b CH585 | `blobs/CH585_new.bin` | 1222 B | ✅ |
| 0xab CH586/7 | `blobs/CH586X_new.bin` | 996 B | ✅ |
| 0x8b CH570/2 | `blobs/CH570_2_new.bin` | 1308 B | ✅ |
| 0x8e CH32M007 | `blobs/CH32M007_8E_new.bin` | 440 B | ✅ |
| 0xc6 CH32H41X_fix | `blobs/CH32H41X_new.bin` | 630 B | ✅ (替换 wlink 旧 618B) |
| 0x86 V317_V660 | `blobs/CH32V317_V660_new.bin` | 440 B | ✅ (edi=0 变体) |
| 0x26 CH664 旧 V205 | `CH664_oldV205_DI_a/b.bin` | 440/460 B | ⚠ (待决定归属) |

### ✔ 与 wlink 已收 blob 一致性

- **IDENTICAL (9)**: V103/CH573/CH569/CH583/V003_ALT/OP8571_ALT/CH643/V00X/CH564 + V317_ALT/V317_DI/CH645
- **Size-pad diff (5)**: V307 (446/448), V205 (446/512), L103 (488/512), CH645_ALT (486/487), CH32H417 (618/630)
  → 建议用 dll blob 替换以获得 "权威" 副本

### ✔ 跳转表 (fcn.10003810) 已 100% 映射

```
Flash 主流程 = 22 个 case, 每个 case: cmp ebx,chip_id; je <branch>; push size; push blob_vaddr
edi 含义: FlashOperation 第 1 个 args
  edi=0x100=新链路 (256B chunk + cmd feature)
  edi=0x80 =老链路 (128B chunk)
edi=0/1 = 同 chip_id 下 variant sub-branch (e.g., 0x46 / 0x86 / 0x26)
```

### ✔ 关键 fixed addresses (DLL 模块全局)

```
0x1016d470  GS cookie
0x10179940..58  CH375 func ptr table (5 fn)
0x10179964..70  last chip descriptor (capability 缓存)
0x10179980     link mode byte ({0x02, 0x12, 0x05, 0x85})
0x10179984     link fw version (minor)
0x1017998c     link fw version (major)
0x10179994     selected link index
0x10179998     opened device index (CH375)
0x101799dc     WCHLinkDll hModule
0x10169990...  chip_id → capability 缓存表
```

---

## Part A — 反编译待办 (剩)

### A1. 命令字节覆盖度补全

- [ ] 🟡 **USB 抓包实测** (usbmon / Wireshark USBPcap)
  - 触发: 给每个 cmd dword 验证线上实测
  - 证据: `docs/reversed/14_cmd_complete_reference.md` 已有反汇编列表
  - 阻塞: 需要物理 WCH-Link + 一个 chip
  - 估算: 1 天 (做 5-10 个主流 family)

- [ ] 🟡 **`SetMCUConfigValue` cmd dword**
  - 未在 `_cmd-func-map.txt` 出现 (wrapper fn 内嵌)
  - 需要反汇编 `fcn.<inner>` (helper fn)
  - 估算: 30 分钟

- [ ] 🟢 **`GetDeviceVersion` (non-B) cmd dword**
  - 反汇编 `fcn.100013e0` 即可
  - 估算: 10 分钟

- [ ] 🟢 **`CompareVersion` / `CompareARMVersion` cmd dword**
  - 估算: 30 分钟

- [ ] 🟢 **`ChangeArmToRvLink` / `SearchArmLink` / `SearchAndSetArmLink` cmd 完整 byte 序列**
  - 已抓到 `0x5201ff81` 但完整 frame (含 args) 未挖
  - 估算: 1 小时

### A2. Chip family 完整性

- [ ] 🟡 **`fcn.100015a0` 中 dl=0x40 / 0x0c / 0x0d 子 case 的具体 body**
  - 这三个 case 是 jump-table sub-selector (empty branch),实际行为共享其它 case 的 body
  - 需要更细的反汇编 (r2 pdf 不足以看 shared code)
  - 估算: 2 小时
  - 收益: 把 capability mask 填到 chip_id 0x0c (CH643) / 0x0d (X035) / 0x40 (V30X family sub)

- [ ] 🟡 **`fcn.0040aee0` 早期条件 (`jne 0x40b968`) 的语义**
  - 11 个 GUI 字符串 (CH32V203/V205/V002_4_5_6_7/M007/L103_M103/L103/V006/564/V317) 在该条件下被 skip
  - 它们走哪条 path 决定 chip_id?是 ARM-HID 模式专用还是另一组 chip-family?
  - 证据: `F-cn.txt` line 260-262,`0x40b10b test eax, eax; je 0x40b968`
  - 估算: 2 小时

- [ ] 🟢 **`SetChipType` 全部 `cmp ebx, X` 分支**
  - 验证 wlink-only chip_id (0x05 V20X / 0x0f CH564 / 0x4e V00X / 0xce V205) 是否被 DLL 接
  - 证据: `F-SetChipType.txt`
  - 估算: 30 分钟

### A3. 数据加密 (低 ROI)

- [ ] 💤 **`McuCompiler_EncryptData` / `McuCompiler_DisEncrypt` 算法**
  - 用途未明，可能 in-flash 加密或协议加密
  - 反汇编这两个 fn 提取算法 (XOR / AES / stream cipher)
  - 估算: 1 天
  - 决策: **不做** — 除非发现实际应用场景

### A4. 链接模式 / 状态机

- [ ] 🟡 **`0x10179980` link mode byte 语义**
  - 多个 fn 先 `cmp dword [0x10179980], {0x02, 0x12, 0x05, 0x85}` 做 sanity check
  - 这个值什么时候设置、什么含义?
  - 需要反汇编 `EnumLink` / `OpenDevice` 中 `mov dword [0x10179980], X` 的赋值点
  - 估算: 1 小时

- [ ] 🟢 **`0x10179994` link index 与 `0x10179998` opened device index 关系**
  - `EnumLink` 的 device 循环 (`cmp edi, 0xa`) 已看到
  - 两个全局的写入时机未追
  - 估算: 30 分钟

### A5. 加密 / 安全

- [ ] 🟢 **GUI `wchlink.wcfg` 配置文件 schema**
  - `Firmware_Link/wchlink.wcfg` 是 link 自更新的 config
  - 拿一个真实文件 (或 mock) 看 schema
  - 估算: 1 小时,但需要文件样本

- [ ] 💤 **PDB 路径泄漏 / 反水印**
  - 二进制嵌 `J:\1.独立项目\...McuCompilerDll_Mul_20260610\Release\WCH-LinkUtility.pdb`
  - 仅作情报收集，没有 wlink 价值
  - 决策: **不做**

### A6. IAP / 模式切换

- [ ] 💤 **完整 IAP 流程反汇编**
  - 用户用 WCH `isp50x` 工具或 `WCHISPTool` 即可
  - 决策: **不做** — 已有现成工具

### A7. BLE-RV `_Ex` 子族协议差异

- [ ] 🟡 **`_Ex` 命令字节与 base 的具体 data 差异**
  - cmd dword 已抓 (`0x4010681`/`0x3080681` 等), 但后续 data frame 区别未挖
  - 影响: BLE 子族 (`CH58X/CH59X/CH565/CH564`) 的细节
  - 估算: 3 小时

---

## Part B — wlink 仓库缺陷 / 待修复 / 待补充

### B0. 已解锁的 wlink feature (2026-09-29 via blob 提取）

> 来自 `fcn.10003810` 跳转表 + `reversed/blobs/*.bin` 提取 — 给 Part B2 的核心 blocker 全部破解。

- [x] 🔴 **新 RiscvChip variant 全套实现** (5 variants, unblock 已得数据） — **已在新芯片家族 MR 完成**（新 chip_id 3 个: 0xab CH586/7, 0x8b CH570/2, 0x8e CH32M030；0x4b CH585 与 0xc6 CH32H41X 是 blob 替换）
  - **CH585 (0x4b)** — blob `reversed/blobs/CH585_new.bin` (1222B)
  - **CH586/7 (0xab)** — blob `reversed/blobs/CH586X_new.bin` (996B)
  - **CH570/2 (0x8b)** — blob `reversed/blobs/CH570_2_new.bin` (1308B)
  - **CH32M030 (0x8e)** — blob `reversed/blobs/CH32M007_8E_new.bin` (440B)（文件名误名，芯片为 CH32M030）
  - **CH32H41X_fix (0xc6)** — blob `reversed/blobs/CH32H41X_new.bin` (630B, 替换 wlink 旧 618B)
  - 每个要做： enum + `from_str`/`try_from_u8` + `code_flash_start`/`data_packet_size`/`write_pack_size`/`get_flash_op`/`support_*`/`do_post_init`
  - 遗留： `code_flash_start` 与部分 `support_*` 标 FIXME(needs hardware)

- [x] 🟡 **从已提取 dll blob 生成 Rust `pub const` 数组** （用 python script) — **已完成**: `scripts/blob2rust.py`
  - 目的： 直接把 `blobs/*.bin` → `src/flash_op.rs` 风格的 `[u8; N]` 数组

- [x] 🟡 **用 dll blob 替换 wlink 已有的 5 个 size-pad diff** — **已核实**: V307/V645_ALT 本就一致（旧记录有误）；CH32H417→630B 已替换；仅剩 L103 尾部 24×0xff 填充（非功能性，保留）
  - V307 (446 vs 448) / V205 (446 vs 512) / L103 (488 vs 512) / CH645_ALT (486 vs 487) / CH32H417 (618 vs 630)

- [x] 🟡 **dedupe CH32V205≡CH32L103 / CH645≡CH32V317 重复常量** — **已在新芯片家族 MR 完成**（`pub use` 别名）

### B1. 命令覆盖度 (对照 14 章)

- [ ] 🔴 **`wlink reset --hard`** (ResetB cmd `0x6010b81`)
  - 当前 wlink 只有软 reset
  - 硬复位在 chip wedging 时救场必备
  - 实现位置: `src/commands/control.rs` 已有 `Reset`,加 `ResetB` variant
  - 估算: 30 分钟

- [ ] 🟡 **`wlink rst high|low|pulse`** (SetRSTPin)
  - cmd `[0x81, 0x0d, al, 0x01]`, al = `0x13/0x14/0x15`
  - 反汇编已得 (`F-SetRSTPin.txt`)
  - 估算: 1 小时 (含 CLI)

- [ ] 🟡 **`wlink vcc 33|50`** (Opt33VOut / Opt50VOut)
  - cmd `[0x81, 0x0d, al, 0x01]`, al = `0x0a` (3.3V) / `0x09` (5V)
  - 反汇编已得 (`F-Opt33VOut.txt`)
  - 估算: 1 小时 (含 CLI)

- [ ] 🟡 **`wlink --access-addr <A> read <L>`** (SetAccessAddress + ReadDevMemory)
  - cmd `0x11050d81` (set) + `0x81 03 08 / 0x81 02 0c` (read)
  - 反汇编已得 (`F-ReadDevMemory.txt`)
  - 估算: 2 小时

- [ ] 🟢 **`wlink opt-end`** (OptEnd cmd `0xff010d81`)
  - 让用户能单独触发 option-bytes commit
  - 估算: 30 分钟

- [ ] 🟢 **`wlink qe --enable/--disable`** (EnableQE / CheckQE)
  - cmd `0x7010d81` / `0x6010d81`
  - 实现位置: `src/commands/control.rs` 加新 cmd
  - 估算: 1 小时

- [ ] 🟢 **`wlink info --uuid`** (GetLinkedMCUID)
  - cmd `0x2010d81` (SetChipType) 后跟 query
  - 估算: 1 小时

- [ ] 🟢 **`wlink --wire 1|2`** (SetSDLineMode / SetTwolineLowSpeed)
  - cmd `0xee020d81` + sub-byte
  - 已部分支持 (auto-detect)，加 explicit flag
  - 估算: 1 小时

- [ ] 🟢 **`wlink flash --romram code=X,boot=Y,user=Z`** (SetROMRAM)
  - cmd `0x180200d81` / `0x5020d81`
  - BLE-RV only
  - 估算: 2 小时

### B2. Chip family 扩展

- [ ] 🟡 **chips.rs 加 7 行 vendor chip_id** (详见 12 章 TODO B)
  - 已验证 ✓: F103 (2 行） / V203 (11 行） / CH641 (4 行） / CH643 (4 行）
  - 待 datasheet: `0x203c0500`/`0x203d0500`/`0x20500408`/`0x20700408`/`0x2080040c`/`0x2081040c`/`0x510000`/`0x620000`/`0x66070000`
  - 阻塞: 需要 datasheet 确定 chip 名 (T6/U6/P6/R6/F8/G8...)
  - 决策: **暂不盲目加**,等 datasheet

- [x] 🟡 **RiscvChip 新 variant** — **blob 已全部捕获** (2026-09-29)
  - 22 blob 全部提取自 `fcn.10003810` Flash 主流程 jump table → `wlink/reversed/blobs/*.bin`
  - **可加 5 个新 variant**: CH585 (0x4b) / CH586 (0xab) / CH570 (0x8b) / CH32M007 (0x8e) / CH32H41X_fix (0xc6)
  - 决策待定: CH664 (0x26) 与旧 V205 归属 (`CH664_OLDV205_A/B`)
  - 数据见 [15_flash_op_blobs.md](15_flash_op_blobs.md)

- [ ] 🟢 **RiscvChip::CH32L103 chip_id 冲突**
  - wlink `CH32L103 = 0x0E` 但 GUI 中 0x0e 对应 F10X/F20X 也成立
  - 12 章 TODO A 已记录，验证 USB 抓包
  - 阻塞: 需要硬件

### B3. SDI print 扩展

- [ ] 🟡 **SDI print 支持更多 family**
  - 当前 wlink 支持 CH32V003 / CH32H41X (#118, #120)
  - DLL capability 显示可扩展: 0x46 (CH645/653), 0x49 (CH641), 0x4e (V00X), 0x86 (V00X/V317/564), 0xa6 (V4X7), 0xab (CH586/7), 0xcb (CH595/6), 0xe6 (X315), 0xc6 (H41X 已做)
  - 阻塞: 需要 SDI-capable WCH-LinkE + 实测
  - 估算: 每个 family 1 小时

### B4. 错误处理 / UX

- [ ] 🟡 **`wlink attach` 给 "芯片已加密/读保护" 错误的可读提示**
  - 当前错误码可能不直观
  - GUI 文案参考: `"Please disable Read-protect before read chip flash!"`
  - 估算: 1 小时

- [ ] 🟢 **`wlink --verbose` 显示当前 link 处于哪种 mode (RV/DAP/ARM-HID)**
  - 需查 `0x10179980` link mode byte (反编译 A4 章)
  - 估算: 30 分钟

### B5. 文档 / 测试

- [ ] 🟡 **`docs/reversed/` 持续维护**
  - 每次 wlink 改 flash 行为，检查 cmd byte 是否对应 WCH-LinkUtility 一致
  - 12 章活页笔记继续 push

- [ ] 🟢 **USB 抓包脚本 (`scripts/capture-usb.sh`)**
  - 便于开发者实测 cmd byte
  - 估算: 1 小时

- [ ] 🟢 **cmd byte 单元测试**
  - 把 14 章 cmd byte 表做成 Rust 常量 + test
  - 估算: 2 小时

### B6. 已知不打算做

- [ ] 💤 **IAP 完整流程** — 用 `isp50x` / `WCHISPTool`
- [ ] 💤 **ARM-HID 旧模式** — dead feature
- [ ] 💤 **EncryptData/DisEncrypt** — 算法未明，无应用场景
- [ ] 💤 **CH375 私有驱动兼容** — macOS/Linux 不存在，wlink 用 hidapi 即可

---

## 优先级建议 (Top 5 下一个 sprint)

按"用户能感知"的影响排序:

1. 🔴 **`wlink reset --hard`** — 修 brick 必备
2. 🟢→🔴 **新 RiscvChip variant ×5** (blob 已抓，只剩 Rust 接入) — **✅ 已完成**（新芯片家族 MR: 3 新 chip_id + 2 blob 替换）— **数据已全，工作量小回报大**
3. 🟡 **`wlink rst high|low|pulse`** — 配合 reset --hard 用
4. 🟡 **`wlink --access-addr <A> read <L>`** — 调试 option bytes
5. 🟡 **替换 5 个 size-pad diff 用 dll blob** — 提升 blob 一致性 — **✅ 基本完成**（仅剩 L103 尾部填充，非功能性）

**低 ROI 推后**: SDI print 扩展 / VCC 电控 / EncryptData — 等硬件验证或用户需求.

---

## 交叉引用

- 反编译产物: `wlink/reversed/` (E-*, F-*, T-*, _*)
- Blob 二进制: `wlink/reversed/blobs/` (22 个 .bin,7 个 new)
- 文档: `wlink/docs/reversed/` (00..15 + BACKLOG)
- wlink 主代码: `wlink/src/`
- 本文件位置: `wlink/docs/reversed/BACKLOG.md`

---

## 更新规则

- 完成一条 → 删除该条 (或打勾移到 "已完成" 区)
- 新发现 → 加到对应 Part (A 反编译 / B wlink)
- 每次大改动 → 同步更新 `12_notes_changelog.md`
