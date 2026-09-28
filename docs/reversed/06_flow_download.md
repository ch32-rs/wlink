# 06 — 完整 Download / Flash 流程

基于:
- 命令字节表 (03 章)
- GUI 字符串证据 (E-strings.txt)
- DLL 函数反汇编

## 1. 总览状态机

```
                 ┌─────────────────────────┐
                 │  link 上电 / attach      │
                 └───────────┬─────────────┘
                             │ EnumLink
                             ▼
                 ┌─────────────────────────┐
                 │  link 枚举完成           │
                 │  (1A86:8010 RV HID)     │
                 └───────────┬─────────────┘
                             │ SetLinkIndex + OpenDevice
                             ▼
                 ┌─────────────────────────┐
                 │  SetChipType(family_idx)│
                 └───────────┬─────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
   ┌─────────┐         ┌──────────┐         ┌─────────┐
   │ query   │         │ option   │         │ download │
   │ info    │         │ bytes    │         │ / flash  │
   └─────────┘         └──────────┘         └─────────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │ OptEnd + CloseDevice
                             ▼
                      ┌─────────────┐
                      │ Reset chip  │
                      └─────────────┘
```

## 2. Query / Info 阶段

DLL calls:
```
EnumLink                  # 找到 WCH-Link (RV mode), 1A86:8010
SetLinkIndex(0)           # 选第一个
OpenDevice                # CH375OpenDevice / 关联句柄
SetChipType(family_idx)   # cmd 0x16010d81 + byte4 idx
GetFlashSum               # cmd 0xfd050d81 -> 总字节 (BLE 用)
GetROMRAM                 # cmd 0x17010d81 -> 分区表
GetChipInfo               # UID + version
GetDeviceVersionB         # link 固件版本
CheckReadPro              # cmd 0x1010681 -> 看 RDP
CheckQE                   # cmd 0x6010d81
GetAccessAddress          # cmd 0x12010d81
```

GUI 上的"Begin to get chip UID & flash size..." / "Begin to query chip code-protect..."
对应这阶段。

错误分支 (GUI 文案):
- `"Failed to connect with WCH-Link!Please affirm link is at RISC-V mode!"` — link 不响应
- `"Failed,chip model mismatch!"` — chip-id 与 SetChipType 不匹配
- `"Failed!Please confirm:1.Is ... correct; 2.Is the chip in single-wire mode; 3.Is the reset pin ... connected"` — 物理连接问题

## 3. Option-bytes 配置阶段

(详见 05 章)

```
SetAccessAddress(addr)         ; cmd 0x11050d81
ReadDevMemory(len)             ; 读当前 option
SetMCUConfigValue(dword)       ; 写新值
OptEnd                         ; cmd 0xff010d81
```

或原子操作:
```
EnableReadPro / EnableQE / DisableDebug / SetSDIPrn / SetSDLineMode
```
每个内部: write cfg + commit。

## 4. Download / Flash 阶段

整个序列:

```
1. SetClearFlashPara(rate, size)   ; cmd 0xfc020d81 — 擦除参数
2. ClearCodeFlashB                 ; cmd 0xf010d81   — 擦 CODE 区
   ClearCodeFlash                  ; cmd 0x8020d81   — 或擦 CODE+USER 区
   Clear                           ; cmd 0x1010281 / 0x2010d81 / 0x5010281 / 0x7010281 — 全擦
3. Download[Ex]                    ; cmd 0x5010281  (B WAIT)
                                    + data 传输
                                    + cmd 0x7010281 (COMMIT) per chunk
4. DownloadExternal                ; cmd 0xf8040d81 / 0xf9040d81 / 0xfa040d81 / 0xfb040d81
                                    用于外置 SPI flash (BLE 部分)
5. GetHexOffsetAddr                ; hex 起始地址
6. ReadDevMemory + 校验            ; 回读程序
7. Update                          ; 完成 commit
```

`sanity check` (文本证据):
```
"The download address of this chip needs to be aligned in [%d] bytes!"
"Failed,the target file size exceeds the maximum flash capacity of the chip!"
"Failed,chip of Version A does not support boot area download!"
"Failed,The current chip does not support BOOT  + USER area download in 2-wires mode!"
"Failed,The current chip doesn't support this function!"
```

## 5. 完成 + Reset

```
Opt33VOut / Opt50VOut   (可选)   — link 输电压
DisableDebug            (可选)   — 关调试口安全提示
EnableReadPro           (可选)   — 上读保护
Reset / ResetB          — 复位 chip
CloseDevice / CloseArmLink
```

警告文案:
```
"The debug interface is enable now,there is risk of code leakage,
 please disable debug interface and enable code protect after download.
 Go on current operation?"
```

## 6. SDI Printf 监听模式

特殊路径 (不烧录):
```
EnumLink + OpenDevice + SetChipType
SetSDIPrn(enable)              ; cmd 0xee020d81
reset (start MCU)
loop: read SDI output from link
SetSDIPrn(disable)
```

凭据:
```
"Only WCH-LinkE support SDI Printf function!"
"Begin to enable SDI printf..."
"Begin to disable SDI printf..."
"Current chip type does`t support SDI Printf function!"
```

## 7. 模式切换 (Link firmware mode change)

特殊命令族 (`0x..01ff81`),把 link 自身的 firmware mode 改变:

```
0x5201ff81   ChangeArmToRvLink            ARM-HID  -> RV
0x6301ff81   ChangeDAPToHidLink           DAP(WinUSB) -> HID
0x7401ff81   ChangeHIDToDAPLink           HID(RV) -> DAP(WinUSB)
0x1010f81    SetIAPMode                   link 进入 IAP 模式 (USB bootloader)
```

切换后 **当前 USB 设备会消失,新 mode 重新枚举**, DLL 必须从 EnumLink 重新走。

## 8. wlink 实现已覆盖的阶段

| Step | wlink 状态 |
|---|---|
| Link 枚举 + attach | ✅ (`wlink -v`) |
| SetChipType (chip 锁) | ✅ |
| Clear / Erase | ✅ (`--erase-before-write` is implicit) | | Download (hex/bin) | ✅ |
| Verify (回读) | ✅ |
| Reset | ✅ |
| ReadMem | ✅ |
| SDI printf | ✅ (#118, #120) |

未覆盖: 看 [08 章](08_wlink_gap_analysis.md)。
