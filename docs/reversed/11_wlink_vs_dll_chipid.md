# 11 — wlink ↔ WCH-LinkUtility 完全对照

**结论**： wlink 的 `RiscvChip` enum 和 `chips.rs::chip_id_to_chip_name` **就是** WCH-LinkUtility 的同一套体系。
没有坐标转换问题。

## 1. USB 协议帧确认 (来自 src/commands/control.rs)

`AttachChip` (cmd `0x81 0x0d 0x01 0x02`) 返回 5 字节:

```
bytes[0]     = RiscvChip (family code)         <- 等同 DLL chip_id (GUI push)
bytes[1..5]  = u32 chip_id (big-endian)        <- 等同 DLL magic mask,等同 chips.rs chip_id
```

DLL 的 `fcn.100015a0` 是对这个返回值的解析器:
```
dl       = bytes[0]           (arg chipid)
[esi]    = bytes[1..5]  (= magic mask for capability check)
ecx_out  = capability mask 1 (flash size tier)
eax_out  = capability mask 2 (0x8000000 = SDI flag)
```

## 2. RiscvChip 对照表

| RiscvChip | (hex) | WCH-LinkUtility chip_id | GUI name | Notes |
|---|---|---|---|---|
| `CH32V103` | 0x01 | 0x01 | "CH32V10X" | 一致 |
| `CH57X` | 0x02 | 0x02 | "CH57X" | 一致 |
| `CH56X` | 0x03 | 0x03 | "CH565/9" | 一致 (含 564/565/569; GUI 单独 0x86) |
| `CH32F10X` | 0x04 | 0x04 | "CH32F10X" | 一致 |
| **`CH32V20X`** | **0x05** | **不在 GUI** | — | ⚠ **wlink-only**; GUI 用 0x06 包装 V20X+V203+V30X |
| `CH32V30X` | 0x06 | 0x06 | "CH32V30X" (case-fallthrough) | 一致 |
| `CH582` | 0x07 | 0x07 | "CH582/3" | 一致 |
| `CH32F20X` | 0x08 | 0x08 | "CH32F20X" | 一致 |
| `CH32V003` | 0x09 | 0x09 | "CH32V003" | 一致 |
| `CH8571` | 0x0A | 0x0a | "CH8571" | 一致 |
| `CH59X` | 0x0B | 0x0b | "CH59X" | 一致 |
| `CH643` | 0x0C | 0x0c | "CH643" | 一致 |
| `CH32X035` | 0x0D | 0x0d | "CH32X035/4/3" | 一致 |
| `CH32L103` | 0x0E | 0x0e | "CH32F10X/F20X-cap1=0x10000" (case) | 一致但都共用 _0e_ case |
| **`CH564`** | **0x0F** | **不在 GUI** | — | ⚠ GUI 用 0x86 包装 |
| `CH32V205` | 0xCE | 0xce (GUI 缺) | — | ⚠ **GUI 没收录 0xce** (firmware <v2.22 报 0x05) |
| `CH641` | 0x49 | 0x49 | "CH641" | 一致 |
| `CH585` | 0x4B | 0x4b | "CH581/4/5" | 一致 |
| `CH645` | 0x46 | 0x46 | "CH645/653" | 一致 |
| `CH32V00X` | 0x4E | **不在 GUI** | — | ⚠ GUI 用 0x86 包装 (V002/4/5/6/7) |
| `CH585` | 0x4B | 0x4b | "CH581/4/5" | 一致 |
| `CH32V317` | 0x86 | 0x86 | "CH32V317"+V00X+M007+V006+CH564 | 一致 (GUI 多个名共用) |
| `CH32V4X7` | 0xA6 | 0xa6 | "CH32V4X7" | 一致 |
| `CH32H41X` | 0xC6 | 0xc6 | "CH32H41X" | 一致 |

## 3. wlink-only 一族 (5 个)

| RiscvChip | wlink id | DLL 等价 | 备注 |
|---|---|---|---|
| `CH32V20X` | 0x05 | (DLL 把 V20X/V203/V30X 全归到 0x06) | wlink 给 0x05 VB/V4C 单独 ID |
| `CH564` | 0x0f | (GUI 归到 0x86) | wlink 给 RISC-V4J 单独 ID |
| `CH32V00X` (V002/4/5/6/7, M007) | 0x4e | (GUI 归到 0x86) | wlink 给 V00X 单独 ID |
| `CH32V205` | 0xce | (GUI 没收录,需要 v2.22+ fw) | wlink 给 V205/V203 新代单独 ID |
| `CH32L103` | 0x0e | (GUI 也用 0x0e, 但 GUI 实际把 L103 归 0x49) | 一致 |

## 4. WCH-only 一族 (9 个)

| DLL id | GUI 名字 | wlink 状态 |
|---|---|---|
| 0x26 | "CH32V205/3CCT" | ❌ 不支持(0x26 是旧版 V205) |
| 0x66 | "CH660" | ❌ 不支持 |
| 0x8b | "CH570/2" | ❌ 不支持 |
| 0x8e | "CH32M030" | ❌ 不支持 |
| 0xab | "CH586/7" | ❌ 不支持 |
| 0xcb | "CH595/6" | ❌ 不支持 |
| 0xe6 | "CH32X315" | ❌ 不支持 |
| 0xf6 | "USB General MCU" | ❌ IAP-only, wlink 不做 |

## 5. magic mask 对照 (chips.rs ↔ DLL)

DLL `fcn.100015a0` 期望的 magic 都是 wlink `chips.rs` 已经实现的 vendor ID:

| DLL chip_id | DLL 期望 magic (mask) | wlink chips.rs 对应记录 |
|---|---|---|
| 0x01 | `0x25000002` | (chips.rs 无 V103 行,但 V103 DBGMCU ID 待考) |
| 0x02 | `0x71000000` + `0x73550000` | `0x710_00000=>CH571`, `0x730_00000=>CH573` ✓ |
| 0x03 | (无 magic 强匹配) | `0x650_00000=>CH565`, `0x690_00000=>CH569` ✓ |
| 0x04 | `0x20000002 / 0x2000000f` | `0x200_04102=>CH32F103C8T6` 微妙差异 (mask AND 后是 `0x20000002`)|
| 0x05 | (DLL 没单独 case,跳到 V30X) | `0x203xxxxx` (CH32V203 系列) |
| 0x06 | 同 0x05 | `0x303/305/307/317 00508` (CH32V30X 系列) ✓ |
| 0x07 | `0x81000000` 头 | `0x810_00000=>CH581`, `0x820_00000=>CH582`, `0x830_00000=>CH583` ✓ |
| 0x08 | `0x20xx0508` family | `0x208_0050C/1050C/2050C/3050C` (CH32V208 系列) ✓ |
| 0x09 | (宽松) | `0x003_00500/10500/20500/30500` (CH32V003 系列) ✓ |
| 0x0a | `0x0/0x1000000/0x2000000` | CH8571 undocumented |
| 0x0b | `0x91000000 / 0x92000009` | `0x920_00000=>CH592` + `0x910_00000=>CH591` ✓ |
| 0x0c | (empty) | `0x643_00601..40601` (CH643W/Q/L/U) ✓ |
| 0x0d | (empty) | `0x035_E0601/10601/60601/A0601/B0601` (X035 variants) ✓ |
| 0x0e | (无 mask) | `0x103_00700/10700/A0700/B0700` (CH32L103 子型) ✓ |
| 0x26 | (无 mask) | (`chips.rs` 没收录 — 旧版 V205/CH664 待加) |
| 0x46 | (无 mask) | (`chips.rs` 没收录 CH645/653 型号) |
| 0x49 | (无 mask) | `0x641_00500/10500/50500/60500` (CH641F/D/U/P) ✓ |
| 0x4b | (无 mask) | `0x930_00000=>CH585` (从 CH581/4/5 找 0x810/0x930 mask) |
| 0x4e | `0x510000 / 0x620000` | `0x002_00600..`, `0x004_00600..`, `0x005_10600`, `0x006_10600`, `0x007_20800` ✓ |
| 0x66 | `0x66070000` | (chips.rs 未收录) |
| 0x86 | (无 mask) | `0x317_0B508/3B508/5B508` (CH32V317 系列) + `0x6xx_00600..` (V00X) ✓ |
| 0x8b | (无 mask) | (chips.rs 未收录 CH570/2) |
| 0x8e | (无 mask) | (chips.rs 未收录 CH32M030) |
| 0xa6 | (无 mask) | `0x4670_0000..4675 0502` (CH32V407/V467 系列) ✓ |
| 0xab | `0x307x0508 / 0x30530508` | (重叠 0x30X, chips.rs 已有 V30X 行) + (CH586/7 未收) |
| 0xc6 | (无 mask) | `0x415/416/417 0050D` (CH32H41X) ✓ |
| 0xcb | (无 mask) | (chips.rs 未收录 CH595/6) |
| 0xe6 | (无 mask) | (chips.rs 未收录 CH32X315) |

## 6. 结论 / wlink TODO

### 已 cover 的 (nearly 100%):
- `RiscvChip` family codes 22 个 ✓
- `chips.rs` vendor chip_id: 20+ 子型号 ✓

### 待补 (按用户实用性):
1. **wlink-only ID 的 magic test 缺失**:  
   - 0x05 (V20X) — DLL 实际没区分，GUI 叫 V20X 但 wlink 的 attach protocol 回 0x05 实测有效。可能 DLL 在 0x05 case fall-through 到 0x06.**wlink 跟 GUI 一致即可**
   - 0x0f (CH564) — wlink 实测可用;GUI 不直接支持
   - 0x4e (V00X) — DLL 0x4e 没 case；fall-through。**等 USB 抓包验证**

2. **WCH-only ID 待 wlink 实现** (作为 RiscvChip 新 enum variant):
   - 0x26 CH664 / 旧版 V205
   - 0x66 CH660 (大 flash 高容量)
   - 0x8b CH570/2
   - 0x8e CH32M030 (V4 motor)
   - 0xab CH586/7 (BLE V4 sub-series)
   - 0xcb CH595/6
   - 0xe6 CH32X315
   - 0xf6 USB General MCU (IAP-only,不做)

3. **chips.rs 待扩充** (vendor chip_id,**不影响 flash**,只影响 chip 显示):
   - CH645/653 ID mask
   - CH664 ID mask
   - CH660 ID mask
   - CH570/2 ID mask
   - CH32M030 ID mask
   - CH595/6 ID mask
   - CH32X315 ID mask

### 注意

DLL 也对 GUI 图层对 `_Ex` family 有处理; `_Ex` 是 BLE 子族用不同协议版 (cmd `0x4xx` vs `0x1xx`)
wlink 不区分 BLE-RV vs 普通 RV 的 attachment version, wlink 的 BLE support 实际上调
 `_Ex` cmd bytes (0x4010681/0x3080681)。 已 cover。

## 7. 验证证据

| 文件 | 用途 |
|---|---|
| `src/lib.rs` RiscvChip | wlink family codes |
| `src/chips.rs` chip_id_to_chip_name | wlink vendor chip_id |
| `src/commands/control.rs` AttachChipResponse | USB 协议确认 (bytes[0]=family, bytes[1..5]=vendor) |
| `docs/reversed/10_chip_id_table.md` | WCH-LinkUtility 完整 name→id→capability |
| `wlink/reversed/T-chip-id-table.txt` | WCH 数据汇总表 |
| `wlink/reversed/_dl-cases.txt` | 33 个 chip_id 的 capability 提取 |
