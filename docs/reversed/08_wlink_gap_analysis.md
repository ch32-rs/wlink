# 08 — wlink 尚未覆盖 (WCH 实现独有) 的功能

按"用户能用上"优先级从高到低排。每行末尾标注 wlink 现在能不能通过手发命令字节实现。

## 1. 一定有用户价值

### 1.1. QE bit 操作
| | |
|---|---|
| 含义 | SPI/QSPI flash Quad-Enable bit |
| WCH 实现 | `EnableQE` (cmd `0x7010d81`), `CheckQE` (`0x6010d81`), `EnableQE_DAP` |
| 命令 | 已反汇编 |
| **wlink 状态** | ❌ 未实现 |
| **新增建议** | `wlink qe [--enable/--disable]` — 一行 cmd |

### 1.2 ResetB (硬复位)
| | |
|---|---|
| 含义 | `0x6010b81` — 不同方式复位 (推测: NRST pin 触发, 而不是软复位) |
| WCH | `McuCompiler_ResetB` |
| **wlink** | ❌ |
| 建议 | `wlink reset --hard` |

### 1.3 SDI 1-wire / 2-wire explicit toggle
| | |
|---|---|
| 含义 | SDI (serial debug interface) 的 wire 数 |
| WCH | `SetSDLineMode` / `SetTwolineLowSpeed` (cmd `0xee020d81`) |
| **wlink** | 部分 — 自动检测; 没有用户命令 |
| 建议 | `wlink --wire 1|2`, `wlink set-sdi-mode` |

### 1.4 Access Address + ReadDevMemory
| | |
|---|---|
| 含义 | 把 chip 的"访问地址"设为 任何 region, 再读 |
| WCH | `SetAccessAddress` (`0x11050d81`), `ReadDevMemory` |
| **wlink** | ✅ 已实现 raw read, ❌ 没 access-addr 抽象 |
| 建议 | 增加 `wlink --access-addr X read <len>` |

### 1.5 ROM/RAM 分区 (BLE)
| | |
|---|---|
| 含义 | BLE 芯片 (CH58x etc) 才算有用 — flash 分 Code/Boot/User |
| WCH | `GetROMRAM` (`0x4010d81`/`0x17010d81`), `SetROMRAM` (`0x180200d81` / `0x5020d81`) |
| **wlink** | ❌ |
| 建议 | `wlink romram --show` / `--set code=X,boot=Y,user=Z` |

### 1.6 EncryptData / DisEncrypt
| | |
|---|---|
| 含义 | 数据加密/解密 (烧录前在 PC 侧处理) |
| WCH | `McuCompiler_EncryptData` / `McuCompiler_DisEncrypt` |
| **wlink** | ❌ |
| 状态 | 不确定算法 (可能 XOR/AES), 需深逆向 |

### 1.7 SetMCUConfigValue (直接写 option-value)
| | |
|---|---|
| 含义 | raw 写 option byte (替代 OptEnd 原子操作) |
| WCH | `McuCompiler_SetMCUConfigValue` |
| **wlink** | ❌ |
| 建议 | `wlink write-cfg <dword>` — 高级用户 |

### 1.8 OptEnd (commit 后的 atomic 写)
| | |
|---|---|
| 含义 | Option-bytes 写入提交 (cmd `0xff010d81`) |
| WCH | `McuCompiler_OptEnd` |
| **wlink** | ❌ (隐式) |
| 建议 | `wlink opt-end` 或整合到上面的 `--write-option` 流 |

### 1.9 电压选择 (Link 输出)
| | |
|---|---|
| 含义 | WCH-Link 给 chip 供电 — 3.3V 或 5V |
| WCH | `Opt33VOut` / `Opt50VOut` |
| **wlink** | ❌ |
| 建议 | `wlink vcc 33|50` |

### 1.10 RST pin 控制
| | |
|---|---|
| 含义 | assert/deassert chip 的 NRST |
| WCH | `SetRSTPin` / `SetRSTPin_DAP` |
| **wlink** | ❌ (通过 hidapi 写 raw cmd 应该可以) |
| 建议 | `wlink rst high|low|pulse` |

### 1.11 Chip 模型独立 (CH32H41x 等新型号的 chip id)
| | |
|---|---|
| 含义 | 新 chip family 加表时 |
| WCH | `SetChipType` (cmd `0x16010d81`) + DLL 表 |
| **wlink** | ✅ 已有 chip-id 表,可持续加 |
| 建议 | 每次 WCH 发布新 chip,从 DLL 反汇编抓 chip-id |

## 2. 用户价值中等

### 2.1 SetClearFlashPara
| | |
|---|---|
| 含义 | 擦除时序 / 速率参数 |
| WCH | cmd `0xfc020d81` |
| **wlink** | 用默认值 | 建议 | `wlink config --clear-para` |

### 2.2 GetHexOffsetAddr
| | |
|---|---|
| 含义 | hex 文件的 base addr |
| WCH | `McuCompiler_GetHexOffsetAddr` |
| **wlink** | 我们 parse hex 自己定 |
| 建议 | 用户用不到 |

### 2.3 GetFlashSum
| | |
|---|---|
| 含义 | chip 总 flash 容量 query |
| WCH | cmd `0xfd050d81` |
| **wlink** | 由 chip-id 查表 |
| 建议 | 可补上,用户 --info 时显示 |

### 2.4 GetDeviceVersionB
| | |
|---|---|
| 含义 | link 固件 version-B (细微版) |
| WCH | `McuCompiler_GetDeviceVersionB` |
| **wlink** | 已查 device version,可补 |

### 2.5 GetLinkedMCUID (UUID)
| | |
|---|---|
| 含义 | 单个 chip 唯一 ID |
| WCH | `MRSFunc_GetLinkedMCUID` (cmd `0x2010d81` sub) |
| **wlink** | ❌ |
| 建议 | `wlink info --uuid` |

### 2.6 MRSFunc_* (MounRiver Studio 入口)
WCH 的 IDE 集成层。17 个 fn 都是薄 wrapper。`wlink` 作为独立 CLI 不需要这层。

## 3. 不值得做

### 3.1 ARM-HID 旧模式
| | |
|---|---|
| 含义 | WCH-Link 的旧 ARM-only 模式 (HID-class) |
| WCH | `McLLink` + `SearchArmLink` + `SearchAndSetArmLink` + `GetArmLinkVersion` |
| 命令 | `0x5201ff81` (ChangeArmToRvLink) |
| Literally 没人用了 — 跳过 |

### 3.2 IAP (chip 通过 USB bootloader)
跳到 [07 章](07_flow_iap.md)。`wlink` 已经够用,不推荐在 wlink 里做 USB-IAP 另一半。

### 3.3 GUI 文案 / 报警逻辑
所有 MFC 对话框字符串、`"Please disable read-protect first!"` 之类 — 都是 presentation 层
和在 DLL 里的 `Enable/DisableReset / OptEnd / disable_debug / enable_debug / set_sdi_printint` 当 chip family 是一-wire & 有多 wire 【源】 [03 章命令字](03_command_table.md)

## 4. 决策矩阵

wlink 已是 "subset of WCH-LinkUtility"。下面是建议的"下一个功能 issue 列表":

```
Tier 1 (PR 拿用户):
  - `wlink qe --enable/--disable`           // 1 cmd
  - `wlink reset --hard`                     // 1 cmd (ResetB)
  - `wlink romram --show` (+ --set)          // 2 cmd
  - `wlink --wire 1|2` / `sdi --1|2`         // 已部分
  - `wlink vcc 33|50`                        // 1 cmd (每个)
  - `wlink rst high|low|pulse`               // 1 cmd

Tier 2 (用户 group 少):
  - `wlink --access-addr <A> read <L>`      // 2 cmd
  - `wlink --uuid`                           // 1 cmd
  - `wlink opt-end`                          // 1 cmd

Tier 3 (深逆向才做):
  - EncryptData/DisEncrypt (算法待挖)
  - IAP 流程 (建议外包给 isp50x)
  - ARM-HID legacy (跳过)
```

## 5. 拿命令字节对照表

去 [03 章命令字节表](03_command_table.md) 看每个功能的具体 cmd,
所有"Tier 1"功能都已知 cmd,**实现成本大约每个 30 行 Rust**。

## 6. 衡量优先级

真实用户中最常见的:

1. SDI printf (✅ 已有)
2. Read-Pro toggle (✅ 已有)
3. Flash write (✅)
4. **QE bit (BLE 用户)** —> 值得做
5. **硬 reset** (chip wedging 时救场) —> 值得做
6. **1-wire / 2-wire explicit** —> Debug 救场
7. **VCC 3V3/5V** —> 给用户接 5V 芯片时有用

Tier 1 这 6 个做完,wlink 就覆盖率就 ≥ 90% 用户场景。
