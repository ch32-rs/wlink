# Handoff — 新芯片家族支持 MR

> **目的**: 把"从 McuCompilerDll.dll 逆向出的新芯片家族"落成一个可提交到 `ch32-rs/wlink` 的 MR。
> **状态**: 数据收集 + 交叉验证已完成；代码尚未开始写。
> **范围说明**: 本文**不涉及 PR #114 的验证**（已明确停止，不再追）。
>
> 证据来源: `reversed/blobs/*.bin`（blob 二进制）、`reversed/F-fcn10003810.txt`（Flash 主流程跳转表）、
> `reversed/F-cn.txt`（GUI 名字→chip_id）、`reversed/F-chip-lookup.txt`（chip_id→capability）、
> `McuCompilerDll.dll`（原始样本，不入库）。

---

## 1. 结论速览（TL;DR）

| # | 事项 | 结论 |
|---|---|---|
| 1 | blob 文件可信度 | ✅ 22/22 从 DLL 按 VA 重新抽取，与原文件**字节完全一致**，可复现 |
| 2 | 真正新增的家族 | **3 个新 chip_id**: `0xab` CH586/7、`0x8b` CH570/2、`0x8e` CH32M030 |
| 3 | 需要换 blob 的已有家族 | **2 个**: `0x4b` CH585（现错挂 CH583）、`0xc6` CH32H41X（618→630） |
| 4 | **不应**新增的家族 | `0x26` CH664/旧 V205 — a/b 两份 blob 都是已有 blob 的重复，无新数据 |
| 5 | 仓库内重复 blob | **2 组**，本次 MR 一并修掉 |
| 6 | 文档需修正 | `15_flash_op_blobs.md` 的 edi 语义 / §4 表 / §5 命名（详见 §5） |
| 7 | 最大未知风险 | 新家族的 `code_flash_start`（0x0 vs 0x08000000）**无硬件无法定论** |
| 8 | 分支 | 必须从 `origin/main` 新开，**不要**用当前 `pr-84`（含 30 个无关 IAP 提交） |

---

## 2. 可直接落地的数据（已逐条验证）

### 2.1 三个新 chip_id

| chip_id | 名称 | blob 文件 | 大小 | DLL VA | 依据 |
|---|---|---|---|---|---|
| `0xab` | CH586/CH587 | `reversed/blobs/CH586X_new.bin` | 996 B | `0x10173038` | `F-fcn10003810.txt` 0x10003f0e/0x10003f21 |
| `0x8b` | CH570/CH572 | `reversed/blobs/CH570_2_new.bin` | 1308 B | `0x101720e0` | `F-fcn10003810.txt` 0x10003f41/0x10003f54 |
| `0x8e` | **CH32M030** | `reversed/blobs/CH32M007_8E_new.bin` | 440 B | `0x10171f28` | `F-fcn10003810.txt` 0x100040bb/0x100040ce |

> ⚠️ `0x8e` 的**文件名是错的**：应叫 `CH32M030`，不是 `CH32M007`。
> 证据: `F-cn.txt` GUI `strcmp("CH32M030")` → `mov bl, 0x8e`（0x0040b5d4→0x0040b609）。
> 而 "CH32M007" 是**另一条** strcmp，命中后 `je 0x40b93e`，`0x40b93e` 是 `mov bl, 0x4e` ⇒ CH32M007 属 `0x4e`(CH32V00X)，与 wlink 现状一致。
> 旁证: `openwch/ch32m030` 仓库存在（电机控制/无线充电），无 `ch32m007` 独立仓库。

### 2.2 两个需要更换 blob 的已有家族

| chip_id | 名称 | wlink 现状 | DLL 正确 blob | 差异性质 |
|---|---|---|---|---|
| `0x4b` | CH585/CH584 | 错挂 `flash_op::CH583`（1326 B） | 1222 B @`0x10172878` | **内容不同**（前 40B 即分叉），非 padding |
| `0xc6` | CH32H41X | 618 B（`3bc899f` 引入，DLL 中查无此串） | 630 B @`0x10172600` | 偏移 8 替换 2B + **偏移 38 插入 12B**，相似度 0.9872 |

### 2.3 仓库内重复的 blob（本次一并修）

`src/flash_op.rs` 中存在**逐字节相同**的常量对：

| 常量 A | 常量 B | 大小 | md5 |
|---|---|---|---|
| `CH32V205` | `CH32L103` | 512 B | `1f5904cfa3c6…` |
| `CH645` | `CH32V317` | 440 B | `896cf21134e2…` |

另外 `CH32V205`(512B) 与 `CH32L103`(512B) 实际上都是 `L103_V205_DI.bin`(488B) + 24×`0xff` 填充，
即 wlink 把 L103 的 blob **复制**给了 V205。DLL 里 `0x0e`/`0xce` 共用 `0x10170938`(488B)，语义上确实是同一份。

**建议**: 保留两个具名常量作为别名（可读性），或合并为一个 const + 两条 `get_flash_op` 分支；二选一即可，不要留两份字面量。

### 2.4 已知重复的 blob 文件（不影响功能，但影响"22 个"表述）

| 内容 | 重复的文件 |
|---|---|
| `896cf21134e2…` | `CH32V317_DI.bin` = `CH32V317_V660_new.bin` = `CH664_oldV205_DI_a.bin` |
| `cad449bbcf36…` | `CH643_L103_DI.bin` = `L103_V205_DI.bin` |
| `ad72cb076e4b…` | `CH664_oldV205_DI_b.bin` = `V317_ALT_new.bin` |

⇒ 22 个文件实为 **18 份唯一内容**；相对 wlink 有增量的只有 **5 份**
（`CH585_new` / `CH586X_new` / `CH570_2_new` / `CH32M007_8E_new` / `CH32H41X_new`）。

### 2.5 漏抽的 blob（无信息量，可忽略或补上）

`0x1016fe10`（498 B，`0x09`/`0x49` 的 edi=0 分支，见 `F-fcn10003810.txt` 0x10004227）
**未**导出为 `.bin`。其内容与 wlink 现有 `flash_op::CH32V003` **逐字节相同**，故无新信息。

---

## 3. 新家族的功能参数（从反汇编推导，含校准）

**校准方法**: 用 wlink 已有的两个**已知正确**家族反推 `fcn.10003810` 第二个 switch 的语义，再套到新家族。

`fcn.10003810` 第二 switch（`0x100045b1` 起）结论：

| 别名 | `esi` 含义 | `var_128h` 含义 |
|---|---|---|
| 校准点 1 | `0x09/0x49` → `esi=0x40`(64)、`var_128h=0x400`(1024) | wlink `CH32V003/CH641`: `data_packet_size=64`、`write_pack_size=1024` ✅ 吻合 |
| 校准点 2 | `0x4e` → `esi=0x100`(256)、`var_128h=0x400`(1024) | wlink `CH32V00X`: `data_packet_size=256`、`write_pack_size=1024` ✅ 吻合 |

⇒ `esi = data_packet_size`，`var_128h = write_pack_size`。据此推出新家族：

| chip_id | 名称 | `data_packet_size` | `write_pack_size` | 依据 |
|---|---|---|---|---|
| `0x4b` | CH585/584 | 256 | 4096 | `0x10004625`→`0x1000462a` |
| `0x8b` | CH570/572 | 256 | 4096 | 同上 |
| `0xab` | CH586/587 | 256 | 4096 | 同上 |
| `0x8e` | CH32M030 | **128** | 4096 | `0x1000460f` `lea esi,[ebx-0xe]` ⇒ 0x8e→0x80 |

### 3.1 capability（`fcn.100015a0`）

| chip_id | cap1 | cap2 | SDI |
|---|---|---|---|
| `0x4b` CH585 | `0x78000`（devid≥8）或 `0x70000` | `0` | ❌ |
| `0x8b` CH570/2 | `0x3c000` | `0` | ❌ |
| `0xab` CH586/7 | `0x70000`（低 nibble=0）或 `0xf0000` | `0` | ❌ |
| `0x8e` CH32M030 | `0x10000` | `0x8000000` | ✅ |
| `0xc6` CH32H41X | 动态（`0x78000` 或 `0xf0000`） | `0x8000000` | ✅ |

> ⚠️ **与 `10_chip_id_table.md` 冲突**: 该文档把 `0xab` 标为 `SDI=Y`，
> 但反汇编 `0x10001dd2`–`0x10001e08` 两个分支都写 `mov dword [edx], 0` ⇒ **cap2=0，无 SDI**。
> 以反汇编为准；`0xab` 的 SDI 建议标为"存疑/待实测"，不要直接写 `support_sdi_print()=true`。

### 3.2 `support_*` 建议值（**工程判断，非反汇编直接结论**）

| chip_id | 参照物 | `support_flash_protect` | `support_query_info` | `support_special_erase` | `support_disable_debug` |
|---|---|---|---|---|---|
| `0xab` CH586/7 | CH585（BLE 系） | true | false | false | true |
| `0x8b` CH570/2 | CH585（BLE 系） | true | false | false | true |
| `0x8e` CH32M030 | CH32V00X（非 BLE） | true | true | true | false |

**这些是对齐同族现有行为的推断，不是从 DLL 读出来的**。MR 里必须如实标注，并最好请作者确认。

---

## 4. 最大未知风险：`code_flash_start`

新家族必须回答"代码 flash 起始是 `0x0000_0000` 还是 `0x0800_0000`"。

wlink 现有规律（`src/lib.rs::code_flash_start`）:
- `0x0` — BLE/无线系: `CH56X` `CH57X` `CH582` `CH585` `CH59X` `CH8571`
- `0x0800_0000` — 其余全部

**尝试过的旁证及其局限（重要，避免后人重复踩坑）:**
- openwch linker script 的 `FLASH ORIGIN` **不能**作为判据:
  `ch570` = `0x00000000`、`ch32m030` = `0x00000000`、`ch32v003` = `0x00000000`、`ch32v307` = `0x00000000` —
  但 wlink 对 CH32V003/CH32V307 用的是 `0x0800_0000`。说明 `.ld` 里的 ORIGIN 是**镜像加载地址**，与 wlink 的 `code_flash_start` 不是一回事。
- blob 内也扫不到可判别的 flash base 常量（`lui 0x8000` 模式在 CH583 等已知家族里同样为 0 次命中）。

**因此:**
- `CH570/CH572`、`CH586/CH587` 是 2.4GHz 无线/BLE 系 ⇒ **倾向 `0x0`**（与 CH585 同族），置信度中。
- `CH32M030` 是电机控制/无线充电 MCU，非 BLE ⇒ **倾向 `0x0800_0000`**，置信度中。
- **需要一块真实硬件 + `wlink flash` 实测确认**。这是本 MR 唯一无法在离线环境闭环的点。

---

## 5. 必须先修正的文档错误

`15_flash_op_blobs.md` 目前被当作 SSOT，但有以下硬伤，MR 里应一并修（否则代码注释会继承错误）:

| 位置 | 问题 | 事实 |
|---|---|---|
| §2 / §6.5 / §9 | edi 语义三处自相矛盾，且出现"512B chunk" | edi 只是传输块长，反汇编中**只取 128/256** 两值（`0x100042c1` `cmp eax, edi`、`0x10004308`）。且 0x46/0x86 两支 edi **都是 0x80=128**，与"新/旧链路"说法不符 |
| §2 注 | "edi=1 通常带 ALT blob" | 反汇编中 edi 是 **per-case blob 变体选择器**，不是 `if old_link_fw {ALT}` 开关；§9 的"wlink 应加 alt 分支"属未证推断 |
| §4 | "V307 dll 446 / wlink 448" | ❌ wlink `CH32V307` 就是 **446 B 且完全一致** |
| §4 | "CH645_ALT dll 486 / wlink 487" | ❌ wlink `CH645_ALT` 就是 **486 B 且完全一致** |
| §4 | "V205 dll 446 / wlink 512" | ❌ 数据有误；0x0e/0xce 走 488 B 那份 |
| §4 | "L103 488 / wlink 512" | ✅ 属实，但差异仅尾部 24×`0xff` 填充 |
| §4 | "CH32H417 630 / wlink 618" | ✅ 唯一真实**内容性**差异 |
| §5 | "7 个全新捕获" | 实为 **5 个**有增量（另 2 个是重复内容） |
| §8 | "9 个新 RiscvChip variant" | 真正新 chip_id 只有 **3 个**（0xab/0x8b/0x8e）；0x4b/0xc6 是换 blob |
| §3 | `CH32V00X (0x04e)` | 应为 `0x4e` |
| §6 | 路径 `wlink/reversed/blobs/` | 仓库内实际为 `reversed/blobs/` |

### 5.1 其它流程风险

`docs/reversed/`、`reversed/`、`McuCompilerDll.dll` 在 git 中**全部 untracked**（`git ls-files` 为空）。
所谓 SSOT 目前只存在于本地工作区，一次 `git clean -fdx` 即全部丢失。

---

## 6. MR 执行计划

### 6.1 分支与基线

```
基线:    origin/main (42a5c3c "Add ch32v4x7 support (#117)")
新分支:  feat/new-chip-families   ← 必须从 origin/main 新开
fork:    andelf/wlink (已存在, parent = ch32-rs/wlink, gh 已登录 andelf)
```

> ⚠️ **不要用当前 `pr-84` 分支**。它相对 `origin/main` 多出 30 个提交
> （`git log origin/main..pr-84`），内容全是 IAP / upgrade / ch375 相关，与本 MR 无关。
> `pr-84` 的 `-- src/` 差异应为空（其额外改动集中在 `probe.rs`/`usb_device.rs`/`commands/mod.rs`）。

### 6.2 提交顺序（建议 3 个 commit，便于 review）

**Commit 1 — 基础设施**
- 新增 `scripts/blob2rust.py`: `reversed/blobs/*.bin` → Rust `pub const [u8; N]`，带 VA/来源注释。
- 决策并执行 blob 的入库方式:
  - ✅ 建议入库 `reversed/blobs/*.bin`（合计 **16.7 KB**，可接受）
  - ❌ **不要**入库 `McuCompilerDll.dll`（1.6 MB，第三方二进制，含 WCH PDB 路径）
  - ❌ **不要**入库整个 `reversed/`（5.4 MB r2 转储）；如需保留证据，只挑 `F-fcn10003810.txt` 等关键片段
  - 在 `.gitignore` 中显式排除 `McuCompilerDll.dll`、`reversed/F-*.txt` 等

**Commit 2 — flash_op 数据**
- 用 `scripts/blob2rust.py` 生成 5 个新常量:
  `CH585`(1222)、`CH586`(996)、`CH570`(1308)、`CH32M030`(440)、`CH32H417`(630，替换 618)
- **修重复**: 消除 `CH32V205`≡`CH32L103`、`CH645`≡`CH32V317` 两份字面量
- 常量命名与注释写清 chip_id + DLL VA + size

**Commit 3 — 接线 + 文档**
- `src/lib.rs`:
  - `enum RiscvChip` 加 `CH586 = 0xAB`、`CH570 = 0x8B`、`CH32M030 = 0x8E`
  - `try_from_u8`: `0xAB`/`0x8B`/`0x8E` 三个分支
  - `get_flash_op`: 新家族指向新常量；`CH585` 从 `CH583` 改指新 `CH585`；`CH32H41X` 指向 630 B 新 blob
  - `data_packet_size`: `CH32M030 => 128`
  - `write_pack_size`: 新家族默认 4096（无需改）
  - `code_flash_start`: 按 §4 的**倾向**先写，注释标 `FIXME(needs hardware)`
  - `ValueEnum`（`value_variants`/`to_possible_value`/`from_str`）+ `support_*` 系列按 §3.2
- `src/chips.rs`: 补新家族的 vendor chip_id（**注意**: 需 datasheet，见 §6.3）
- `CHANGELOG.md` Unreleased / `README.md` 支持列表
- 修正 `15_flash_op_blobs.md`（§5 表所列各项）
- 单测: 新 chip_id 的 `try_from_u8` 往返 + blob 长度断言（现有全仓仅 5 个 `#[test]`）

### 6.3 明确不在本 MR 范围

| 项 | 原因 |
|---|---|
| `0x26` CH664 / 旧 V205 | a/b blob 均为已有 blob 重复，无新数据；归属待定 |
| `src/chips.rs` 的 9 个 vendor chip_id | 需 datasheet 确定型号名（T6/U6/P6/R6/F8/G8…），无据不盲加 |
| `CH32L103 = 0x0E` 与 F10X/F20X 冲突 | 需 USB 抓包，阻塞于硬件 |
| PR #114 相关验证 | **已按要求停止** |
| 加密算法 / IAP 流程 / ARM-HID | BACKLOG 已标 💤 不做 |

---

## 7. 待办清单（按顺序）

- [ ] 从 `origin/main` 新建 `feat/new-chip-families`
- [ ] 写 `scripts/blob2rust.py`
- [ ] 决定并落实 blob 入库范围 + `.gitignore`
- [ ] 生成 5 个新 flash_op 常量（含 CH32H417 换 630 B）
- [ ] 修 `flash_op.rs` 内 2 组重复常量
- [ ] `lib.rs` 全量接线（enum / try_from_u8 / get_flash_op / 参数 / ValueEnum / support_*）
- [ ] 补测 + `cargo fmt` + `cargo clippy` + `cargo test`
- [ ] 修 `15_flash_op_blobs.md` 与 `10_chip_id_table.md` 的 0xab SDI 行
- [ ] 更新 CHANGELOG / README
- [ ] 推 fork → 对 `ch32-rs/wlink:main` 开 PR，正文注明:
      - 数据来源是 WCH-LinkUtility 自带 `McuCompilerDll.dll` 的逆向
      - §3.2 的 `support_*` 是推断
      - `code_flash_start` 待硬件确认
- [ ] 硬件到手后实测 `code_flash_start` 与 SDI，回收 §4/§3.1 的 FIXME

---

## 8. 交叉引用

- 跳转表原始反汇编: [`reversed/F-fcn10003810.txt`](../../reversed/F-fcn10003810.txt)
- GUI 名字→chip_id: [`reversed/F-cn.txt`](../../reversed/F-cn.txt)
- chip_id→capability: [`reversed/F-chip-lookup.txt`](../../reversed/F-chip-lookup.txt)
- blob 二进制: [`reversed/blobs/`](../../reversed/blobs/)（22 个 `.bin`，18 份唯一内容）
- 待修正的 SSOT 文档: [15_flash_op_blobs.md](15_flash_op_blobs.md)
- chip_id 表: [10_chip_id_table.md](10_chip_id_table.md)
- 总待办: [BACKLOG.md](BACKLOG.md)
- 本文件: `docs/reversed/handoff.md`
