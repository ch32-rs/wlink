# 13 — GUI 快捷键 → 完整调用链

> ⚠ §9 配置字段 → cmd 字节表已被 [14_cmd_complete_reference.md](14_cmd_complete_reference.md) 替代。
> 本章保留快捷键 handler 链路信息 （独有价值），字节查询去 14 章。

每个 F-key (Alt+F1..F12 + A) 对应一个用户操作，每操作触发一段 DLL 调用。
本表把 **GUI 文案** / **F-key handler 函数** / **DLL 调用 sequence** 拉通。

## 1. 快捷键分布 (来自 virtual_268 handler + GUI 字符串)

| Hotkey | UI Label | fkey handler | 主要 DLL 调用 |
|---|---|---|---|
| Alt+F1 | Choose the file to be downloaded | `fcn.0040c5f0` | (`GetOpenFileNameA`) 仅选文件 |
| Alt+F2 | Connect WCH-Link | `fcn.0041bd90` | (link enum/open) |
| Alt+F3 | Query Chip Info | `fcn.0041b450` | GetChipInfo |
| Alt+F4 | Execute Checked Operation | `fcn.0041ba10` | (CheckedOp 主流程,CreateThread) |
| Alt+F5 | Query Read-Write-Protect Status | `fcn.0041b490` | CheckReadPro(_Ex) |
| Alt+F6 | **Enable Read-Protect** | `fcn.0041b500` → `fcn.00416ed0` | EnableReadPro* + Reset |
| Alt+F7 | **Disable Read-Protect** | `fcn.0041b570` | DisableReadPro* + Reset |
| Alt+F8 | Read Chip Flash | `fcn.0041bda0` | ReadDevMemory |
| **F9** | **Erase Chip** | `fcn.0041b5e0` | (完整 flash 序列,DAP) |
| **F10**| **Program**     | `fcn.0041b790` (CreateThread) | (下载) |
| **F11**| **Verify**      | `fcn.0041b7c0` (CreateThread) | (Read+比较) |
| **F12**| **Reset**       | `fcn.0041b7f0` | SetChipType_DAP（重连) + Reset |
| Alt+A  | (special action) | `fcn.0041bdd0` | 未知 |

## 2. F9 (Erase Chip) — `fcn.0041b5e0` 反汇编看调用序列

```
sanity: 全局 0x69c73c/0x69c708/0x69c720 (read-protect 状态)
main:
  fcn.0040c540   ; show progress UI
  fcn.004162b0   ; cleanup log
  fcn.0040c540   ; show progress
  ...
  fcn.0040f940   ; error popup helper
  fcn.0042d4ae   ; locale util
  call edi       ; SendMessageA 联动
  fcn.0040aee0   ; name→chip_id (combo idx)
  fcn.00402360   ; string helper
  McuCompiler_SetChipType_DAP  <- 锁 chip
  ...
  fcn.00417a10   ; chip-specific erase impl
  fcn.0040c540   ; ui update
```

注： F9 实际**用 DAP 而不是 RV/HID** — 因为 ARM 模式工作的 DAP 是默认 flash path。
**wlink 的等价**: `wlink erase` → 走 RV/HID 的 `McuCompiler_ClearCodeFlash*`,cmd 一样 (`0x8020d81` / `0xf010d81`)。

## 3. F6 (Enable Read-Protect) 反汇编 sequence (F-one.txt 全套)

```
[fcn.0041b500 = F6 entry]
  sanity check:
    0x69c73c == 0  ? -> jump error
    0x69c708 == 0  ? "External Memory does not support this operation!"
    0x69c720 == 0  ? -> jump skip
    0x69c6fc == 0  ? -> jump main
  main fcn.00416ed0:
    OpenDevice
    SetTargetChip(chip_id, mode)
    McuCompiler_GetChipInfo_DAP
    McuCompiler_EnableReadPro_DAP    <- write RDPro
    McuCompiler_Reset_DAP             <- hard reset to apply
    McuCompiler_CloseDevice
    display "Succeed,the Read-protect is Enable, and WP is ..."
  fallback fcn.00416a60:
    (another branch, CheckReadPro_Ex_DAP)
  tail fcn.004129f0: rename UI button
```

**证据**: `F-one.txt` 调用 sequence + `F-fkeys.txt` 行号。

## 4. F7 (Disable Read-Protect) — `fcn.0041b570`

只 1 个 helper (`fcn.00417200`)。后续 chain 同 F6 但 DisableReadPro。

## 5. F3 (Query Chip Info) — `fcn.0041b450`

仅调 `fcn.00402360` (string util)，它的具体实现在每个 chip-family 不同 message loop。GetChipInfo(_DAP)。

## 6. F4 (Execute Checked Operation) — `fcn.0041ba10`

```
  call fcn.0040fce0   ; helper
  CreateThread        ; async
  CloseHandle
```

`fcn.0040fce0` 是 **主烧录流程**,CreateThread 异步。它内部分别调用 **OpenDevice/SetChipType/GetChipInfo/CheckReadPro/Clear/Download/Update/OptEnd/EnableDebug/Reset** 全套。

## 7. 通用 sanity-check 全局变量

GUI 用一堆 `0x69cXXX` 全局 byte 作为 cross-handler state:

| Address | 含义 (推测) | 用法 |
|---|---|---|
| `0x69c73c` | link opened flag | F6/F7/etc 都先 `cmp 0` |
| `0x69c708` | "External Memory support" flag | F6 special-case |
| `0x69c720` | re-entrancy lock (0=ok, 1=busy) | 所有 handler wrap |
| `0x69c6fc` | "chip selected" flag | 用户是否选了 chip |
| `0x69a330` | GS cookie | _security_check_cookie |
| `0x69c5f8` | firmware path buffer | 烧录文件路径 (260 bytes) |
| `0x69c5f6` | link mode byte | RV=1/ARM-DAP=? |
| `0x69c5f7` | link mode flag2 | |
| `0x69c700` | fopen result | |

(See F-one.txt for `0x69c700` used in combo handler; `0x69c73c/0x69c708/0x69c720/0x69c6fc` from F6 entry)

## 8. 与 wlink 对应

| Hotkey | wlink equivalent | 状态 |
|---|---|---|
| F1 Choose file | (implicit in `wlink flash <path>` args) | ✅ |
| F2 Connect | `wlink probe` / `wlink -v` (auto) | ✅ |
| F3 Query Info | `wlink info` | ✅ |
| F4 Execute | `wlink flash` | ✅ |
| F5 Query RP | (implicit in `wlink info`?) | ⚠️ 待加独立命令 |
| F6 Enable RP | (没有等价) | ❌ |
| F7 Disable RP | (没有等价) | ❌ |
| F8 Read Flash | `wlink dump` | ✅ |
| F9 Erase | `wlink erase` | ✅ |
| F10 Program | `wlink flash` | ✅ |
| F11 Verify | `wlink verify --flash <file>` (隐含) | ✅ |
| F12 Reset | `wlink reset` | ✅ |
| Alt+A | (none / not impl) | — |

## 9. 配置字段 → cmd 字节表 (新发现总结)

GUI "Enable MCU Code Read-Protect" 选项 (组合菜单） 触发（在 fcn.00416a60 链）:

```
1. OpenDevice                     (枚举,无 cmd)
2. SetTargetChip(chip_id, mode)   (无 cmd, setter 只存 dll 全局)
3. SetChipType_DAP(idx, &buf, 1)  → DAP cmd dword (DLL cmd 序列)
4. GetChipInfo_DAP                → ? 
5. EnableReadPro_DAP              → 0x3080681
6. Reset_DAP                       → DAP cmd (DAP-libusb)
7. OptEnd (隐含)                    → 0xff010d81
8. CloseDevice                    (清理)
```

每个 GUI 配置字段对 cmd 字节都可推导:
- **选 chip** (combo) → SetChipType(idx) → hid cmd `0x16010d81 + chip_idx`
- **wire mode** (1/2 wire) → SetSDLineMode → hid cmd `0xee020d81 + mode_byte`
- **低速** → SetTwolineLowSpeed → 同 cmd 不同 data
- **query RP** → CheckReadPro* → `0x1010681/0x4010681`
- **enable RP** → EnableReadPro* → `0x3010681/0x3080681`
- **disable RP** → DisableReadPro* → `0x2010681/0x2080681`
- **QE** → EnableQE/CheckQE → `0x7010d81/0x6010d81`
- **SDI printf** → SetSDIPrn → `0xee020d81`
- **3.3V / 5V out** → Opt33VOut/Opt50VOut → cmd 字节 (DLL 待挖)
- **RST pin** → SetRSTPin → cmd 字节 (DLL 待挖)
- **IAP mode** → SetIAPMode → `0x1010f81`
- **chip info** → GetChipInfo* → cmd 字节 (DLL 待挖)
- **read flash** → ReadDevMemory* → cmd 字节 (DLL 待挖)
- **erase** → Clear/ClearCodeFlash(B) → `0x1010281/0x2010281/0xf010d81`
- **flash** → DownloadEx/DownloadExternal → `0x5010281/0x7010281/0xf8-0xfb..0d81`
- **verify** → ReadDevMemory → 同上
- **reset** → Reset/ResetB → `0x1010b81/0x6010b81`
- **OptEnd** → OptEnd → `0xff010d81`
- **chip-id set** → SetChipType* → `0x16010d81`
- **access addr** → SetAccessAddress → `0x11050d81`
- **Flash sum** → GetFlashSum → `0xfd050d81`
- **clear flash para** → SetClearFlashPara → `0xfc020d81`
- **ROM/RAM config** → GetROMRAM/SetROMRAM → `0x17010d81/0x180200d81/0x5020d81/0x4010d81`

注： `Opt33VOut`/`Opt50VOut`/`SetRSTPin`/`GetChipInfo`/`ReadDevMemory` 这几个因为 `McuCompiler_` fn 是 wrapper，实际 cmd dword 在 helper 内，需要再反汇编一层。标 TODO 待挖。

## 10. 下一步 (creation of evidence)

到目前为止 GUI 反汇编度足够：**每个菜单/fkey 能追到 DLL API,DLL API 能追到 cmd dword**,不必再 developer 反汇编。

剩下没拿到的 cmd dword:
- Opt33VOut / Opt50VOut (link 输出电压控制)
- SetRSTPin / SetRSTPin_DAP
- GetChipInfo (具体 cmd,数据格式)
- GetDeviceVersionB (link firmware 详版号)
- ReadDevMemory / ReadDevMemory_DAP
- GetDeviceMode / GetDeviceMode_DAP

→ 一条 r2 命令就拿到，在 12 章 TODO 加上。
