# 15 — Flash_op Blob 完整提取 ( chip_id → blob )

> **本文档给出 `McuCompilerDll.dll` 里所有内嵌 flash loader blob 的 SSOT (Single Source Of Truth) 表。**
> 22 个 blob 全部提取为独立 `.bin` 文件，可直接给 wlink 用。

## 1. 提取位置与跳转表

| fn | 位置 | 作用 |
|---|---|---|
| `fcn.10003810` | DLL `.text` (3957 行) | **Flash 主流程**;行 487-747 是 `cmp ebx, chip_id; je + push blob_vaddr + push size` 的完整 switch |
| `McuCompiler_Download` | `0x10004790` | 同一 switch 的精简版 (~20 cases) |
| `McuCompiler_DownloadEx` | `0x10005820` | 同上，第三份 |

`fcn.10003810` 被 `MRSFunc_FlashOperationB`/`McuCompiler_Download*` 多次调用，**是常规 flash 路径的 dispatcher**。

证据: `reversed/F-fcn10003810.txt` 反汇编 + `reversed/blobs/` 每 blob 已抽出为独立 `.bin`（仓库根相对路径）。

## 2. 全部 blob 排布表

| blob_vaddr | size | chip_id (ebx) | edi | wlink 现状 |
|---|---|---|---|---|
| `0x1016e830` | 494  | 0x01 | 0x100 | ✅ CH32V103 (identical) |
| `0x1016ea20` | 1102 | 0x02 | 0x100 | ✅ CH573 (identical) |
| `0x1016ee70` | 1156 | 0x03 | 0x100 | ✅ CH569 (identical) |
| `0x1016f320` | 446  | 0x05, 0x06 | 0x80 | ✅ V307 wlink 446 (一致) |
| `0x1016f4e0` | 1326 | 0x07, 0x0b | 0x100 | ✅ CH583/58X (identical) |
| `0x10172878` | 1222 | 0x4b | ? | ❌ **CH585NEW — wlink 缺** |
| `0x10173038` | 996  | 0xab | ? | ❌ **CH586/7 NEW — wlink 缺** |
| `0x101720e0` | 1308 | 0x8b | ? | ❌ **CH570/2 NEW — wlink 缺** |
| `0x101701e0` | 1386 | 0x0a | ebx+0x76 | ✅ OP8571_ALT (identical) |
| `0x10170008` | 466  | 0x09, 0x49 | ? | ✅ V003_ALT (identical) |
| `0x10170750` | 488  | 0x0c, 0x0d | 0x100 | ✅ CH643/L103 (identical) |
| `0x10170938` | 488  | 0x0e, 0xce | 0x100 | ⚠ L103 wlink 512（差异仅 24 字节 0xff 尾部填充，非内容差异） |
| `0x10170b20` | 486  | 0x46 + edi=0 | 0x80 | ✅ CH645_ALT wlink 486 (一致) |
| `0x10170d08` | 440  | 0x46 + edi=1 | 0x80 | ✅ V317/CH645 DI (identical) |
| `0x10170ec0` | 500  | 0x4e | 0x80 | ✅ CH32V00X (identical) |
| `0x101715a0` | 1532 | 0x0f | 0x100 | ✅ CH564 (identical) |
| `0x10171ba0` | 440  | 0x86,0x66,0xa6,0xe6 + edi=0 | 0x80 | ❌ **CH32V317_V660_NEW — wlink 缺** |
| `0x10171d58` | 460  | 0x86,0x66,0xa6,0xe6 + edi=1 | 0x80 | ✅ V317_ALT (identical) |
| `0x10171f28` | 440  | 0x8e | ? | ❌ **CH32M030 NEW — wlink 缺**（0x8e 不是 CH32M007，那是 0x4e） |
| `0x10172600` | 630  | 0xc6 | ? | ❌ **CH32H41X NEW — wlink H417 618B diff 완전 |
| `0x10171ba0` | 440  | 0x26 + edi=0 | 0x80 | ❌ **CH664 旧 V205a NEW — wlink 缺** |
| `0x10173420` | 460  | 0x26 + edi=1 | 0x80 | ❌ **CH664 旧 V205b NEW — wlink 缺** |

**edi 含义（修正）**: edi 是 FlashOperation 第 1 个 args，是 **chunk 长度选择器（128 或 256），不是 512**。反汇编中 edi 只取 `0x80`(128) / `0x100`(256) 两值（`0x100042c1` `cmp eax, edi`、`0x10004308`）。`edi=0/1` 是 **同一 chip_id 下 per-case blob 变体选择器**（like V003 vs V003_ALT），不是「if old_link_fw then ALT」开关。注意 0x46/0x86 两支 edi **都是 0x80=128**。

~~**edi==1 通常带 ALT blob**~~ — 已删：该说法属未证推断，见 §9 修正。

## 3. wlink 已有的 (9 个，直接复用)

```
CH32V103 (0x01)        ✅ IDENTICAL to DLL
CH573    (0x02)        ✅ IDENTICAL
CH569    (0x03)        ✅ IDENTICAL
CH583/58X (0x07/0x0b)  ✅ IDENTICAL
OP8571_ALT (0x0a)      ✅ IDENTICAL
V003_ALT   (0x09/0x49) ✅ IDENTICAL
CH643/L103 (0x0c/0x0d) ✅ IDENTICAL
CH32V00X   (0x4e)      ✅ IDENTICAL
CH564      (0x0f)      ✅ IDENTICAL
V317_ALT   (0x86...)   ✅ IDENTICAL (= V317_ALT_new)
V317_DI    (0x46)      ✅ IDENTICAL (= CH645/V317)
```

## 4. wlink 待补 (size 微差, 仍是同一 blob 改 padding)

```
CH32V307     dll 446 / wlink 446 — 一致（旧记录「wlink 448」有误）
CH32V205     0x0e/0xce 都走 488 B 这份（wlink 里填充至 512 B；旧记录「dll 446」有误）
CH32L103     dll 488 / wlink 512 — 属实，差异仅尾部 24×0xff 填充，非内容差异
CH645_ALT    dll 486 / wlink 486 — 一致（旧记录「wlink 487」有误）
CH32H417     dll 630 / wlink 618 (12B diff) — 唯一真实内容性差异（已在本 MR 修复）
```

## 5. wlink 完全缺失的 (7 个捕获，其中 5 个有增量 — **本次新提取**)

```
CH585        0x4b   NEW @0x10172878 (1222B)
CH586/7      0xab   NEW @0x10173038 (996B)
CH570/2      0x8b   NEW @0x101720e0 (1308B)
CH32M030     0x8e   NEW @0x10171f28 (440B)   （文件名 CH32M007_8E_new.bin 是误名）
CH32H41X     0xc6   NEW @0x10172600 (630B) (>= wlink H417 618B)
CH664 a      0x26   NEW @0x10171ba0 (440B) (edi=0)  （与已有 blob 重复，无增量）
CH664 b      0x26   NEW @0x10173420 (460B) (edi=1)  （与已有 blob 重复，无增量）
```

加上我已有但尺寸不对的:
```
CH32V307            在 dll @0x1016f320 (446B) 给 V30X/V20X/0x06
CH32L103            在 dll @0x10170938 (488B) 给 L103
CH645_ALT           在 dll @0x10170b20 (486B) 给 645+ALT
```

## 6. 提取文件路径

```
reversed/blobs/
  CH32V103.bin               494 B
  CH573.bin                  1102 B
  CH569.bin                  1156 B
  CH32V307_V30X.bin           446 B
  CH583_58X.bin              1326 B
  CH585_new.bin              1222 B     ← NEW
  CH586X_new.bin              996 B     ← NEW
  CH570_2_new.bin            1308 B     ← NEW
  OP8571_ALT.bin             1386 B
  CH32V003_ALT_L103_DI.bin    466 B
  CH643_L103_DI.bin           488 B
  L103_V205_DI.bin            488 B
  CH645_ALT_DI.bin            486 B
  CH32V317_DI.bin             440 B
  CH32V00X.bin                500 B
  CH564.bin                  1532 B
  CH32V317_V660_new.bin       440 B     ← NEW
  V317_ALT_new.bin            460 B
  CH32M007_8E_new.bin         440 B     ← NEW
  CH32H41X_new.bin            630 B     ← NEW
  CH664_oldV205_DI_a.bin      440 B     ← NEW (重 0x10171ba0)
  CH664_oldV205_DI_b.bin      460 B     ← NEW
```

## 6.5 edi=0 / edi=1 sub-branch (V46/V86/Va6/Ve6/V26 共享 chip_id 的 variant 选择)

```
0x46: edi=0 -> 486B CH645_ALT    edi=1 -> 440B CH645 (main)
0x86: edi=0 -> 440B CH32V317_V660 edi=1 -> 460B V317_ALT
0x66 / 0xa6 / 0xe6: fall-through 到同一个 0x86 case (共享 blob)
0x26: edi=0 -> 440B CH664_a (0x10171ba0) edi=1 -> 460B CH664_b (0x10173420)
```

`edi` 是 FlashOperation 的第 1 个 args，是 chunk 长度选择器（128 或 256），不是 512；同时 `edi=0/1` 也是 per-case blob 变体选择器（like V003 vs V003_ALT），并非「新旧链路」开关。wlink 现有 V003 vs V003_ALT 即后者模式。

## 7. 如何加进 wlink

给每个 blob 写 `wlink/src/flash_op.rs` 同格式代码。例子 (CH585_new):

```rust
/// For CH585 (BLE V4 sub-series).
/// Captured from WCH-LinkUtility McuCompilerDll @ 0x10172878.
// 0x4b
pub const CH585_NEW: [u8; 1222] = [
    0x01, 0x11, 0x02, 0xce, ...   // 从 blobs/CH585_new.bin 抽 byte
];
```

生成命令 (python):

```python
import hashlib
with open('reversed/blobs/CH585_new.bin','rb') as f: blob = f.read()
hexbytes = ', '.join(f'0x{b:02x}' for b in blob)
# 每 16 byte 一行缩进 4
lines = [', '.join(f'0x{b:02x}' for b in blob[i:i+16]) for i in range(0, len(blob), 16)]
body = ',\n    '.join(lines)
print(f'''/// For CH585
// 0x4b
pub const CH585_NEW: [u8; {len(blob)}] = [
    {body}
];''')
```

## 8. RiscvChip variant 现在能加 (B2 解锁)

| 新 RiscvChip | chip_id | blob | flash_op 已得 |
|---|---|---|---|
| `CH585` | 0x4b | `CH585` (1222B) | ✅ |
| `CH586` | 0xab | `CH586` (996B) | ✅ |
| `CH570` | 0x8b | `CH570` (1308B) | ✅ |
| `CH570B` | 0x8b | 共享 | ✅ |
| `CH32M030` | 0x8e | `CH32M030` (440B) | ✅ |
| `CH32H41X` | 0xc6 | `CH32H41X` (630B) | ✅ (替换 618B wlink H417) |
| `CH664` (旧 V205) | 0x26 | `CH664_OLDV205_A/B` | ✅ |

加上 chip_name → chip_id (`from_str`) + `try_from_u8` + 主 flash_op match + `code_flash_start`/`data_packet_size`/`write_pack_size`,3 个新 chip_id (0xab/0x8b/0x8e) 完整加入（0x4b 与 0xc6 是 blob 替换，不是新 chip_id）。

## 9. edi=0 / edi=1 的语义 (V003 / V645 / V664 共享 chip_id)

```
0x46: edi=0 -> 486B CH645_ALT blob    edi=1 -> 440B CH645 blob
0x86: edi=0 -> 440B CH32V317_V660     edi=1 -> 460B V317_ALT  
0x66: edi=0 -> ?                      edi=1 -> (上同,0x86 fall-through 共用)
0xa6: edi=0 -> (same as 0x86 edi=0)   edi=1 -> (same as 0x86 edi=1)
0xe6: edi=0 -> (上同)                  edi=1 -> (上同)
0x26: edi=0 -> 440B CH664 a           edi=1 -> 460B CH664 b (0x10173420)
```

`edi` 是 FlashOperation 的第 1 个 args，是 per-case blob 变体选择器 + chunk 长度（128/256）选择器；「通常 edi=1 走新链路 + ALT blob」属未证推断，0x46/0x86 两支 edi 都是 0x80=128。

wlink 应该在 `get_flash_op` 加 `alt` 变体场景的判断仍属未证推断（见 §6.5 修正）；现有 wlink 已分了 (e.g. `CH32V003` vs `CH32V003_ALT`),机制相同只是尚未参数化。

## 10. 12 章 changelog 引用

```
- 2026-09-29: fcn.10003810 完整反汇编,跳转表全映射 (chip_id → blob_vaddr+size),22 个 blob 提取到 blobs/*.bin,7 个 family blob 捕获（其中 5 个有增量）。
- wlink RiscvChip variant 解锁: CH585(blob 替换)/CH586/CH570/CH32M030(误名 CH32M007)/CH32H41X(blob 替换)/CH664(重复，未加).
```
