# 02 — USB 传输层

`McuCompilerDll.dll` 不直接控 USB;它把"link"当一包命令字写出去。底层 transport 分两条：

## 1. RV/HID（默认） — 经 CH375/WCH 私有驱动

链路：
```
McuCompilerDll
   ↓ GetProcAddress (运行时装配)
WCHLinkDll.dll   ← 优先
CH375Dll.dll     ← fallback
   ↓ CH375 driver (kernel mode)
USB-Bulk to WCH-Link (VID=1A86 PID=8010, RV mode)
```

### 装配位置

`fcn.1000a850` (helper, 5 个 caller) 是装配器：

```
LoadLibraryA("WCHLinkDll.dll")             ; 0x101799dc
GetProcAddress(m, "CH375GetDeviceDescr") → 0x10179940
GetProcAddress(m, "CH375GetDeviceName")  → 0x10179944
GetProcAddress(m, "CH375OpenDevice")     → 0x10179954
GetProcAddress(m, "CH375CloseDevice")    → 0x10179958
GetProcAddress(m, "CH375SetTimeoutEx")   → 0x10179950
GetProcAddress(m, "CH375ReadEndP")       → 0x1017994c
GetProcAddress(m, "CH375WriteEndP")      → 0x10179948     ★ 写
```

如果 `WCHLinkDll.dll` 找不到 → fallback `CH375Dll.dll` 重新跑同一套 GetProcAddress。

### 全局状态表 (0x10179940 起)

| Vaddr | 含义 |
|---|---|
| `0x10179940` | CH375GetDeviceDescr fn-ptr |
| `0x10179944` | CH375GetDeviceName fn-ptr |
| `0x10179948` | **CH375WriteEndP** fn-ptr (主发) |
| `0x1017994c` | **CH375ReadEndP** fn-ptr (主收) |
| `0x10179950` | CH375SetTimeoutEx fn-ptr |
| `0x10179954` | CH375OpenDevice fn-ptr |
| `0x10179958` | CH375CloseDevice fn-ptr |
| `0x10179994` | 当前 link index (EnumLink 选了第几个) |
| `0x10179998` | 当前已 open 的 device index (CH375 用) |
| `0x101799dc` | `hModule` for WCHLinkDll.dll |
| `0x1016d470` | GS cookie (VS 编译器) |

实际调用 = `call dword [0x10179948]` (写) 或 `call dword [0x1017994c]` (读)。
所有 HID/RV/_Ex 的 McuCompiler_* 走这条路。

### "Kit D" — 函数分类证据

来自 `_calls.tsv`：
```
McuCompiler_CheckQE            -> KERNEL32 WriteFile  (其实就是 CH375WriteEndP)
McuCompiler_ChangeHIDToIAP     -> KERNEL32 WriteFile
McuCompiler_ChangeHIDToDAPLink -> KERNEL32 WriteFile
McuCompiler_ChangeArmToRvLink  -> KERNEL32 WriteFile
McuCompiler_GetArmLinkVersion  -> KERNEL32 WriteFile
```

这些 fn 是 ARM-HID 旧模式 / 模式切换专用，直接用 Windows 文件 I/O（CreateFile+WriteFile），
说明 ARM-HID 是用 HID-class 而不是 CH375 私有驱动。

## 2. DAP — libusb-1.0

`_DAP` 函数走 `libusb_bulk_transfer`：

```
EnumLink_DAP:  libusb_init(&libusb_ctx → 0x101799d8)
               libusb_get_device_list(ctx, &lst → 0x1017995c)
               …cmp descriptor for VID/PID match…
               libusb_open … → 0x101799d4
```

DAP 函数表 (印证 `_calls.tsv`)：
```
McuCompiler_CheckQE_DAP            libusb_bulk_transfer
McuCompiler_CheckReadPro_DAP       libusb_bulk_transfer
McuCompiler_CheckReadPro_Ex_DAP    libusb_bulk_transfer
McuCompiler_ClearCodeFlash_DAP     libusb_bulk_transfer
McuCompiler_ClearCodeFlashB_DAP    libusb_bulk_transfer
McuCompiler_Clear_DAP              libusb_bulk_transfer
McuCompiler_DisableReadPro_DAP     libusb_bulk_transfer
McuCompiler_EnableQE_DAP           libusb_bulk_transfer
McuCompiler_EnableReadPro_DAP      libusb_bulk_transfer
McuCompiler_GetChipInfo_DAP        libusb_bulk_transfer
McuCompiler_GetDeviceMode_DAP      libusb_bulk_transfer
McuCompiler_GetROMRAM_DAP          libusb_bulk_transfer
McuCompiler_Opt33VOut_DAP          libusb_bulk_transfer
McuCompiler_Opt50VOut_DAP          libusb_bulk_transfer
McuCompiler_Reset_DAP              libusb_bulk_transfer
McuCompiler_SetChipType_DAP        libusb_bulk_transfer
McuCompiler_SetDeviceToIAP_DAP     libusb_bulk_transfer
McuCompiler_SetDeviceToRV_DAP      libusb_bulk_transfer
McuCompiler_SetRSTPin_DAP          libusb_bulk_transfer
```

## 3. ARM-HID 旧模式 — KERNEL32 文件 I/O

`McuCompiler_ChangeArmToRvLink / ChangeHIDToDAPLink / ChangeHIDToIAP / GetArmLinkVersion / SearchAndSetArmLink`
用的是标准 Windows：
```
CreateFileA(\\.\HID path) + WriteFile + ReadFile + WaitForSingleObject
```
通过 `SetupDiGetClassDevsA(GUID_HID)` + `HidD_GetAttributes` 枚举。
旧 ARM-HID 模式是个普通 HID-class 设备，不走 CH375 私有驱动。

## 4. 总结一张表

| 模式 | 用什么 | 由谁开 | Win driver 依赖 |
|---|---|---|---|
| RV/HID (default) | `WCHLinkDll.dll`/`CH375Dll.dll` → CH375 driver | `EnumLink`/`OpenDevice` | 是 (CH375/WinUSB) |
| ARM-HID (legacy) | `CreateFile` on `\\?\HID#VID_1A86&PID_55xx` | `SearchArmLink` | 否 (HID class) |
| DAP | `libusb_bulk_transfer` (WinUSB) | `EnumLink_DAP` | 否 (libusb,需要 Zadig 装 WinUSB 驱动) |

## 5. 与 wlink 的差距

`wlink` (Rust):
- 用 `hidapi` 枚举 + `read`/`write` — **HID 抽象跨平台**
- 不支持 CH375 私有驱动 (Linux/macOS 也没有)
- 不支持 ARM-HID 旧模式 (dead feature)
- DAP 通过 `libusb`/原生 WinUSB

CH375 那一层 (`WCHLinkDll.dll` + 5 个 import) 在 macOS/Linux 完全不存在, 所以我们的实现 _必然_
绕过它, 直接通过标准的 USB HID API。**给 wlink 的核心参考 = `McuCompilerDll.dll` 这一层**,
不是 CH375。

## 6. 磁盘证据

- `T-table-func-ptrs.md` — 函数指针布局
- `F-dll-helpers.txt` 中 `FN_INT:1000a850` — 装配代码全文
- `_calls.tsv` — 每个导出函数调用了哪个 import (按 transport 分类的证据)
