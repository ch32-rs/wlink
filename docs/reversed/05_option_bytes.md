# 05 — 配置位 (Option Bytes) 处理

WCH 的 option bytes 是各芯片族在 flash 末尾的若干字节,决定 read-protect / write-protect / debug-enable /
SDI printf / QE 等硬件行为。**`WCH-LinkUtility` 通过这一组命令字节读写**;具体 bit 含义因芯片族而异。

## 1. 用户视角 (GUI 文案)

GUI 暴露给用户 5 类配置位:

| 类别 | 显示字符串 | 触发函数 |
|---|---|---|
| **Read-Protect** | `"Enable MCU Code Read-Protect"` / `"Disable MCU Code Read-Protect"` | `Enable/DisableReadPro*` |
| **Write-Protect** | (复合在 Read-Protect 反馈内 — GUI 不单独暴露按钮,只在做全片擦 / 修改 option 时一起报) | (内部) |
| **SDI Printf** | `"Enable SDI Printf"` / `"Disable SDI Printf"` | `McuCompiler_SetSDIPrn` |
| **1-wire / 2-wire** | `"1-wire serial debug interface"` / `"2-wires serial debug interface"` | `SetSDLineMode` / `SetTwolineLowSpeed` |
| **关闭调试口** | (警告字符串) | `McuCompiler_DisableDebug` |

反馈只 4 种组合 (RD × WP):
```
"Succeed,the Read-protect and Write-Protect is Enable!"       // 11
"Succeed,the Read-protect is Disable,and Write-Protect is Enable!"   // 01
"Succeed,the Read-protect is Enable,and Write-Protect is Disable!"   // 10
"Succeed,the Read-protect and Write-Protect is Disable!"      // 00
```

`QueryRProtect` 当前 RD 状态; Enable/DisableReadPro 同时改 RD 和 WP。

## 2. 命令 → 配置位的映射

| 操作 | cmd dword | byte2 | byte1:byte0 |
|---|---|---|---|
| Query ReadPro (RV/HID base) | `0x1010681` | 0x01 | 0x0106 |
| Query ReadPro (Ex/DAP) | `0x4010681` | 0x01 | 0x0106 (sub 4) |
| Enable RDPro (RV/HID base) | `0x3010681` | 0x01 | 0x0106 (sub 3) |
| Disable RDPro (RV/HID base) | `0x2010681` | 0x01 | 0x0106 (sub 2) |
| Enable RDPro (Ex/DAP) | `0x3080681` | 0x01 | 0x0806 |
| Disable RDPro (Ex/DAP) | `0x2080681` | 0x01 | 0x0806 |
| Query QE (RV/HID base + DAP) | `0x6010d81` | 0x01 | 0x010d (sub 6) |
| Enable QE | `0x7010d81` | 0x01 | 0x010d (sub 7) |
| Disable Debug (关调试口) | `0x1010e81` | 0x01 | 0x010e |
| SDI printf + 1/2-wire | `0xee020d81` | 0x02 | 0xee0d |
| Set access address | `0x11050d81` | 0x01 | 0x110d (5 bytes!) |
| Get access address | `0x12010d81` | 0x01 | 0x120d |
| Set ROM/RAM part | `0x18020d81` / `0x5020d81` | 0x02 | 0x180d |
| OptEnd (commit) | `0xff010d81` | 0x01 | 0xff0d |

观察:
- **`0x06` byte0 是 RDPro 类**, **`0x0d` byte0 是配置/模式类**, **`0x0e` byte0 是关调试口**, **`0x0f` byte0 是 IAP**。
- `0xee` 是 SDI 专用 (sub-id 还没细分 printf vs wire-mode 是分 frame 后续 byte 决定)。
- `SetMCUConfigValue` 是另一个 **direct 写 option-value** 的路径 (没在我们抓的 cmd 列表里,可能用通用 path)。

## 3. 选项字的写入时机

伪流程 (反汇编 + 字符串证据):

```
1. GUI: 用户点 "Enable Read-Protect"
2. → EnableReadProEx (BLE) 或 EnableReadPro (RV base) 或 EnableReadPro_DAP
3. → 写 option-byte 命令 + sleep
4. → OptEnd (cmd 0xff0d) 提交
5. → link 给 chip 一个 reset (使 option 生效)
6. → 屏幕上弹 "Succeed, the Read-protect and Write-Protect is Enable!"
```

## 4. ReadDevMemory + SetAccessAddress 用于读 option byte

```
GetAccessAddress(_DAP) -> 0x12010d81   ; 读当前访问地址
SetAccessAddress(_DAP) -> 0x11050d81 + 4-byte addr  ; 设访问地址
McuCompiler_ReadDevMemory(_DAP)   ->    ; 读 数据
```

读 option-byte 时把 access addr 设到 option-byte 区 (各 family 不同),然后 ReadDevMemory:
- **该地址不在 wlink 中暴露** - 我们的 read 是 raw flash 读
- WCH 通过 access addr 把不同 chip 的特殊区 (option/system/info) 统一抽象

## 5. SDI Printf 特殊 - 仅 WCH-LinkE

证据字符串:
```
"Only WCH-LinkE support SDI Printf function!"
"Current chip type does`t support SDI Printf function!"
```

GUI 在按下 SDI printf 按钮时检查:
1. Link 必须是 **WCH-LinkE** 型号 (HW 检查)
2. Chip family 必须支持 SDI 输出 (V2/V4) — 不支持 2-wire-only 的 chip

DLL cmd `0xee020d81` 既用于 SetSDIPrn 也用于 SetSDLineMode。可能后续 data byte 区分。

## 6. 1-wire / 2-wire 切换

DLL 有两个函数:
- `McuCompiler_SetSDLineMode` — 选 1-wire (SDI) 或 2-wire (SWD)
- `McuCompiler_SetTwolineLowSpeed` — **two-line 低速** mode (低速 SDI FALLBACK?)

GUI 文案里说 "1-wire serial debug interface" 即 SDI, "2-wires" 即 SWD。
CMD 字节还不知 (`_cmd-func-map.txt` 没有 SetTwolineLowSpeed, 应该走通用帧)。

## 7. 恢复流程 (fail-safe)

GUI 在 disable-debug 之前会警告:
```
"The debug interface is enable now,there is risk of code leakage,
 please disable debug interface and enable code protect after download.
 Go on current operation?"
```
这是 **enable debug -> 危险** 的提示,不是 disable 提示。

## 8. wlink 已实现的 (git log 看)

| 功能 | wlink 子命令 |
|---|---|
| Read-Pro query/toggle | `wlink protect [--enable/--disable]` |
| SDI printf enable/disable | **wlink SDI print support** (#118/#120) |
| Reset | `wlink reset` |
| 1-wire (auto) | link 探测 |
| Flash Write/Erase | `wlink flash` |

## 9. wlink 尚未覆盖的 (跳到 08 章)

- ❌ **QE (Quad-Enable)** 操作 - SPI/QSPI flash 加速
- ❌ **Access Address + ReadDevMemory** (option/system flash 读)
- ❌ **SetMCUConfigValue** 直接位写
- ❌ **SetROMRAM** 分区设置 (Code+Boot+User, BLE 用)
- ❌ **ResetB** 硬复位 (有些 chip 需要)
- ❌ **`McuCompiler_Update`** - 校验完成后 finalize
- ❌ **EncryptData/DisEncrypt**
- ❌ ARM-HID 旧模式 (dead feature)

## 10. 安全模型注记

WCH 把 option-byte 看作 atomic 写: 通过 OptEnd 提交,而且要 chip reset 才生效。
所以**完整 sequence = 写 option → OptEnd → 给 chip 复位**。

`wlink` 当前 `wlink protect --enable` 简化成 1 cmd。要严格重放官方协议:
```
wlink write_option_bytes <value>
wlink opt_end          # 提交
wlink reset            # 生效
```

如果用户忘了 reset,option 不生效。后续可加 sanity check。
