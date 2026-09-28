# 07 — IAP / USB Bootloader 流程

IAP = In-Application Programming, 给一个 chip (或 link 自身) 切到 USB 模式,由 PC 直接送 firmware。

## 1. 两类 IAP

### A. Chip 进 USB-Bootloader
- chip 自身有 factory USB bootloader (CH32V103, X035, V003 等支持)
- 由 `McuCompiler_SetIAPMode` (cmd `0x1010f81`) 通知 link 把 chip 重启到 USB 模式
- link 通过 RST 控制 chip,启动后 chip 自己暴露为 USB-DFU / WCH 私有 IAP device
- GUI 单独再枚举这个 USB device 来 download

### B. Link 自身 IAP
- Link 固件升级也用 IAP
- `McuCompiler_ChangeHIDToIAP` (cmd `0x1010f81`) 让 WCH-Link 自己进 IAP 模式
- USB 重新枚举为 IAP device
- PC 给 link 送新固件

## 2. chip-IAP 流程 (主用例)

GUI 文案:
```
"Set IAP mode..."
"USB FS Download" / "USB HS Download" / "IAP模式"
"Failed to set IAP-Mode WCH-Link!Err:%d"
"Failed:Can`t set to IAP!"
"Succeed to entering IAP mode!"             (estimate; 类似)
```

代码路径:
```
1. GUI: 用户点 "Set IAP mode" 或 ChipMode -> IAP
2. McuCompiler_SetIAPMode         # cmd 0x1010f81 -> link
3. link 给 chip 做 hard reset 同时拉起 bootloader
4. PC 应看到新 USB 设备 (VID:PID 是 WCH IAP-specific)
5. McuCompiler_GetIAPType         # cmd 0x2010f81 (query)
6. 这边走另一路径 (用户态 USB-IAP 通信,不再过 WCH-Link)
7. Download via USB (FS/HS)
8. 完成 → MCU reset → 跑用户代码
```

## 3. 模式切换矩阵

DLL 提供 4 个 mode-change cmd (byte2=0x01, byte1=0xff):

| cmd | 函数 | 效果 |
|---|---|---|
| `0x5201ff81` | `ChangeArmToRvLink` | ARM-HID → RV-HID |
| `0x6301ff81` | `ChangeDAPToHidLink` | DAP-WinUSB → RV-HID |
| `0x7401ff81` | `ChangeHIDToDAPLink` | RV-HID → DAP-WinUSB |
| `0x1010f81` | `SetIAPMode` / `ChangeHIDToIAP` / `SetDeviceToIAP_DAP` | → IAP 模式 |

加 `0x2010f81` (`GetIAPType` / `GetDeviceMode*`) 用来 query 当前 mode。

## 4. wlink 当前对 IAP 的处理

`wlink/src/main.rs` 提到 `--enable-sdi-print` 之类,没有显式 IAP 命令。
**wlink 没实现 chip-IAP 流程** (也没必要 — 用户可以直接用 WCH 提供的 `isp50x` 工具):
- 我们自己开 USB-IAP 通信代价大 (要 libusb 完整 re-attach 流程)
- 通常烧录直接 WCH-Link 就够

## 5. 备注

- WCH 把 IAP 当作 **link firmware mode** + **chip boot mode** 的并集,一个命令触发两种行为
- `GetDeviceMode(_DAP)` 返回当前 link 处于 [RV / ARM / ARM-HID / IAP] 哪个 — 在 GUI 显示用
- 文档: https://www.wch.cn/downloads/WCH_LinkIAP_ZIP.html (WCH-Link 自身 IAP 升级说明)

## 6. 果子

如果用户问"如何独立给 chip 用 USB 烧录",wlink 不应该实现 IAP,
而是建议用 `OpenOCD` / `isp50x` / `WCHISPTool`。
