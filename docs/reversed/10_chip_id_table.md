# 10 — Chip-ID ↔ Chip-Name ↔ Capability 完整映射表

来自两层二进制分析:

| 层 | 位置 | 用途 |
|---|---|---|
| **GUI 层** (WCH-LinkUtility.exe) | `fcn.0040aee0` (2705 行,inline strcmp chain) | 把 combo box 字符串名 → chip_id (bl) |
| **DLL 层** (McuCompilerDll.dll) | `fcn.100015a0` (916 行) | 把 chip_id (dl) + link descriptor (magic) → capability mask (arg_8h, arg_ch) |

## 1. 完整 Name → Chip-ID 表 (GUI)

源自 `F-cn.txt` （反汇编 `fcn.0040aee0`）。一行 GUI 字符串 → 一个 chip_id (PUSH 到 `McuCompiler_SetChipType`):

```
chip_id  0x01  <- CH32V10X
chip_id  0x02  <- CH57X
chip_id  0x03  <- CH565_9           (UI: "CH565/9")
chip_id  0x04  <- CH32F10X          (ARM Cortex-M3)
chip_id  0x06  <- CH32V20X   (same family as V30X/V203)
chip_id  0x06  <- CH32V203
chip_id  0x06  <- CH32V30X
chip_id  0x07  <- CH582_3           (UI: "CH582/3")
chip_id  0x08  <- CH32F20X          (ARM Cortex-M3)
chip_id  0x09  <- CH32V003          ★ SDI 单线代表
chip_id  0x0a  <- CH8571
chip_id  0x0b  <- CH590_1_2         (UI: "CH590/1/2")
chip_id  0x0c  <- CH643
chip_id  0x0d  <- CH32X035_4_3      (X035/4/3)
chip_id  0x26  <- CH32V205_3CCT     (UI: "CH32V205/3CCT")
chip_id  0x26  <- CH32V205
chip_id  0x26  <- CH664
chip_id  0x46  <- CH645
chip_id  0x46  <- CH653
chip_id  0x49  <- CH32L103_M103     (UI: "CH32L103/M103")
chip_id  0x49  <- CH32L103          ← CH32L103 单独 SKU, GUI 显示
chip_id  0x49  <- CH641
chip_id  0x4b  <- CH581_4_5         (UI: "CH581/4/5")
chip_id  0x66  <- CH660
chip_id  0x86  <- CH32V002_4_5_6_7  (UI: "CH32V002/4/5/6/7")
chip_id  0x86  <- CH32M007
chip_id  0x86  <- CH32V006
chip_id  0x86  <- CH564
chip_id  0x86  <- CH32V317
chip_id  0x8b  <- CH570_2           (UI: "CH570/2")
chip_id  0x8e  <- CH32M030
chip_id  0xa6  <- CH32V4X7
chip_id  0xab  <- CH586_7           (UI: "CH586/7")
chip_id  0xc6  <- CH32H41X
chip_id  0xcb  <- CH595_6           (UI: "CH595/6")
chip_id  0xe6  <- CH32X315
chip_id  0xf6  <- USB_General_MCU   (UI: "USB General MCU")  ← IAP USB-IAP
```

## 2. Chip-ID → Capability 表 (DLL)

源自 `_dl-cases.txt`，从 `fcn.100015a0` 提取的 33 个 case。

**有两个能力 mask 写出**:
- **cap1** (`[ecx]`) — flash size + feature 等级 mask
- **cap2** (`[eax]`) — SDI printf 支持 flag (`0x8000000` = 有 SDI print)

**magic mask** 是从 link 读回的 4 字节按位 AND 后的期望值。

| chip_id | 期望 magic (AND mask) | cap1 | cap2 | 推测含义 |
|---|---|---|---|---|
| 0x01 | `0x25000002` | `0x8000` | `0x8000000` | CH32V10X | SDI=Y, 小 flash |
| 0x02 | `0x71000000` + `0x73550000` | `0x30000` | — | CH57X | 48K flash |
| 0x03 | — | `0x70000` | — | CH565/9 | 大 flash |
| 0x04 | `0x20000002` -> `0x8000`; `0x2000000f` -> `0x10000` | （变体） | — | CH32F10X | (F10X 不同子型) |
| 0x05 | 12 个 `0x203x05xx` + 4 个 `0x208x050c` | `0x78000` | — | CH58X 子型 |
| 0x06 | — | (transition) | — | V20X 总入口 (fallthrough) |
| 0x07 | 0x81000000 头或 fallback | `0x30000` + `0x40000` | — | CH582/3 |
| 0x08 | 多个 `0x20xx0508` magic | `0x78000 / 0x38000` | `0x8000000` | CH32V30X / F20X | SDI=Y |
| 0x09 | — | `0x4000` | `0x8000000` | **CH32V003** | SDI=Y, 最小 flash |
| 0x0a | `0x0/0x1000000/0x2000000` | `0x1e000 / 0x3e000 / 0x7e000` | — | CH8571 (V10X multi-variant) |
| 0x0b | `0x91000000` + `0x92000009` | `0x30000 / 0x70000` | — | CH590/1/2 |
| 0x0c | (empty branch) | — | — | CH643 |
| 0x0d | (empty branch) | — | — | X035 子型 |
| 0x0e | — | `0x10000` | `0x8000000` | F10X/F20X | 64KB, SDI=Y |
| 0x0f | — | `0x70000` | — | ? reserved |
| 0x49 | — | `0x4000` | — | L103/M103/641 family | 小 flash |
| 0x46 | — | `0x80000` | `0x8000000` | CH645/653 | 中型, SDI=Y |
| 0x4e | `0xffffe`/`0x510000`/`0x620000` | `0x4000/0x8000/0xf800/0x8000000` | `0x8000000` | X035/4/3 多子型 | SDI=Y |
| 0x4b | — | `0x78000` then `0x70000` | — | CH581/4/5 | 大 flash |
| 0x8e | — | `0x10000` | `0x8000000` | CH32M030 | 64KB, SDI=Y |
| 0x8b | — | `0x3c000` | — | CH570/2 | 小 flash |
| 0xcb | — | `0x3c000` | — | CH595/6 | 小 flash |
| 0xc6 | — | (only cap2) | `0x8000000` | CH32H41X | SDI=Y |
| 0xce | — | `0x40000` | `0x8000000` | CH32V205/V205-3CCT/CH664 | 256K, SDI=Y |
| 0x26 | — | `0x38000` | — | CH32V205/205_3CCT/CH664 | 224K |
| 0x66 'f' | `0x66070000` | `0xf7000` | `0x8000000` | CH660 | ~980K, SDI=Y |
| 0xa6 | — | `0xf8000` | `0x8000000` | CH32V4X7 | 992K, SDI=Y |
| 0xe6 | — | `0x78000` | `0x8000000` | CH32X315 | SDI=Y |
| 0xab | `0x307x0508 / 0x30530508` | `0x70000 / 0xf0000 / 0xf7000 / 0xf800` | `0` (反汇编 mov dword [edx],0) | CH586/7 | SDI=N （之前误标 Y) |
| 0x40 '@' | `0x30500508 / 0x30520508 / 0x305b0508 / 0x305c0508 / 0x30540508 / 0x30300504 / 0x30310504 / 0x30320504 / 0x30330504 / 0x3170b508+` | `0x38000 / 0x78000` | — | **V4 / 高端 sub-variant jump table** |
| 0x86 | — | `0xf8000` | — | V002/4/5/6/7, M007, V006, 564, V317 | 大 flash |

注:
- chip_id `0x40` 实际是 sub-selector — 它的 case 内部又根据 magic 跳到一组共用 body。从指令看是 `cmp dl, 0x40; je 0x10001f37` 等，**实际不是独立 chip-id**，是 case-table 优化。
- chip_id `0x86` 反汇编里是大批量共享 (V002-V317 都到同一 case), 实际是 **wlink 中 QingKe-V2 全家族**。

## 3. Capability Mask 解释

| cap1 值 | bits | 估计含义 |
|---|---|---|
| `0x4000` | 14 | ~16K flash |
| `0x8000` | 15 | ~32K flash |
| `0x10000` | 16 | ~64K flash |
| `0x1e000` | 13-16 | ~120K |
| `0x38000` | 15-17 | ~224K (CH581 印证) |
| `0x3c000` | 14-17 | ~240K (CH595/570 印证) |
| `0x3e000` | 13-17 | ~248K |
| `0x40000` | 18 | ~256K (V205) |
| `0x70000` | 16-18 | ~448K (V30X/565) |
| `0x78000` | 15-18 | ~480K (V30X) |
| `0x7e000` | 13-18 | ~504K |
| `0x80000` | 19 | ~512K (CH645/653) |
| `0xf7000` | 12-19 | ~960K |
| `0xf8000` | 15-19 | ~992K (V4X7/V317) |

cap1 估算公式：`(cap1 >> 12) * 4 KiB` = flash size
(CH32V003: `0x4000 >> 12 = 4` → 16K ✓, CH32V20X `0x10000 >> 12 = 16` → 64K ✓)

| cap2 值 | 含义 |
|---|---|
| `0x8000000` | SDI printf SUPPORT (bit 27) |
| (absent) | 无 SDI print |

## 4. 与 wlink 对照

wlink/src/chip.rs 用的 chip-id 是 QingKe 内核 descriptor 直读 (0x00=v003, 0x10=v002, ...)

WCH-LinkUtility 用的 chip-id 是 GUI 字符串场景下的 fn 内部 ID.

**这两套完全不一样**:
- wlink: 通过 USB 命令从 chip 读 self-descriptor,把读到的 byte 当 chip-id
- WCH-LinkUtility: 用户从 GUI combo 选 family → 字符串 → chip_id

## 5. 实用 — 用 chip_id 给 wlink 加 `--wch-id N`

```
chip_id → actual flash size:
   9 (V003)          →  16 KiB
   1 (V10X)          →  32 KiB  (但 V103=64K, 需 magic)
   6 (V20X/V203/V30X)→ 64/64/448 KiB 
   ...

所以 wlink 可以给用户加 `--wch-id` 复用 WCH 的 family 概念
```

但 wlink 已经直接读 chipid + flash size, **不需要**这个表。

新挖的价值点：
- GUI 提供了**同一个 chip_id 覆盖多个 GUI 字符串** (e.g. 0x26 涵盖 CH664+CH32V205+V205-3CCT) — 这告诉我们**wlink 给相同 flash size + debug 特性的 chip 共享同一程序**就可以了。

## 6. 备注

`fcn.0040aee0` 没列出 `CH32V205/3CCT` 单独的 chip_id (同 0x26) — GUI 界面显示是一样的，但实际 chip_id 仍 0x26。

chip_id **0x40 不是真 id** — 是 case-table jump，它的 case 又按 magic 分到 0x38000/0x78000 共享 body。
可以认为：**V30X/V31X/V4X7 是 chip_id 0x40 的子分支；但 GUI 不会把 0x40 push**，它会 push 更具体的 id 0xa6 (V4X7) / 0x86 (V317) 等。
