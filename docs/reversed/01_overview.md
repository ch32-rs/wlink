# 01 — WCH-LinkUtility 整体结构

## 1. 二进制组成

```
WCH-LinkUtility.exe     GUI (MFC)       ──┐
                                            │  静态绑定 (import table)
McuCompilerDll.dll      协议/烧录引擎   ◄──┘
   │ LoadLibrary + GetProcAddress 动态绑定 ↓
   ├──► WCHLinkDll.dll   (HID/RV 模式, 优先)
   │     fallback: CH375Dll.dll (CH375 芯片 USB 驱动包装)
   └──► libusb-1.0.dll   (DAP/ARM WinUSB 模式)
```

`wlink` 这个 repo 直接在用户态用 libusb / hidapi，所以 **CH375 那层是完全跳过的**；我们只有
`McuCompilerDll.dll` 这层逻辑可参考。

## 2. DLL 三种模式

每个完整功能有 1~3 个变体，按导出函数名分：

| 后缀 | 模式 | 连接 | 芯片族 |
|---|---|---|---|
| 无后缀 | RV / HID | HID 通过 CH375/WinUSB HID 接口 | CH32V 大部分 + BLE RV |
| `_DAP` | ARM / CMSIS-DAP | WinUSB/libusb | CH32F10X / CH32F20X |
| `_Ex` | BLE-RV 扩展 | 同 HID | CH57X / CH58X / CH59X / CH565 |

字符串证据：
```
"WCH-Link is at RISC-V mode!"
"WCH-Link is at ARM-WINUSB mode!"
"WCH-Link is at ARM-HID mode!"
"Connected RISC-V mode WCH-Link Cnt:%d"
"Failed to connect with WCH-Link!Please affirm link is at RISC-V mode!"
"Only RISC-V mode supports this feature!"             // SDI
```

## 3. 105 个导出函数全列（按功能分组）

### 3.1 链路发现 / 打开

| 函数 | 模式 | 备注 |
|---|---|---|
| `McuCompiler_EnumLink` | RV-HID | 枚举 1A86:8010 设备，dynamic-load WCHLinkDll |
| `McuCompiler_EnumLink_DAP` | DAP | libusb_get_device_list |
| `McuCompiler_SetLinkIndex` |  | 选 link idx 存到 0x10179994 |
| `McuCompiler_OpenDevice` / `CloseDevice` | RV-HID | CH375OpenDevice wrapper |
| `McuCompiler_OpenDevice_DAP` / `CloseDevice_DAP` | DAP | libusb_open |
| `McuCompiler_SearchArmLink` / `CloseArmLink` | ARM-HID | ARM-HID 旧模式 |
| `McuCompiler_SearchAndSetArmLink` | ARM-HID |  |
| `McuCompiler_ChangeHIDToIAP` / `ChangeHIDToDAPLink` | RV→other | 改 link 自身的 firmware mode |
| `McuCompiler_ChangeArmToRvLink` | ARM→RV | 同上 |
| `McuCompiler_ChangeDAPToHidLink` | DAP→HID | libusb |

### 3.2 模式切换 / 设置 link 行为

| 函数 | 用途 |
|---|---|
| `McuCompiler_SetIAPMode` | 切 link 到 IAP（USB 升级 link） |
| `McuCompiler_SetDeviceToIAP_DAP` | DAP 下切到 IAP |
| `McuCompiler_SetDeviceToRV_DAP` | DAP→RV 切换 |
| `McuCompiler_SetArmFlag` | (内部) ARM-HID 标志 |
| `McuCompiler_SetTargetChip` | (内部) 由 GUI chip 名字查找 chip id |
| `McuCompiler_SetChipType` / `_DAP` | 注入当前芯片族 ID |

### 3.3 读 ID / Version

| 函数 | 模式 | 命令字节 |
|---|---|---|
| `McuCompiler_GetChipInfo` / `_DAP` | 通用 | （通过 helper） |
| `McuCompiler_GetDeviceVersion` / `B` / `_DAP` |  |  |
| `McuCompiler_CompareVersion` / `ARM` / `_DAP` | 与最低固件比较 |  |
| `McuCompiler_GetArmLinkVersion` | ARM-HID |  |
| `MRSFunc_GetLinkedMCUID` | RV-HID | 拿 chip UUID |
| `MRSFunc_GetMCUWaferVer` | RV-HID | wafer 芯片版本 |
| `McuCompiler_GetFlashSum` | RV-HID | 总 flash 容量 |
| `McuCompiler_GetROMRAM` / `_DAP` | RV-HID | 分 ROM/RAM 区 |

### 3.4 Read/Write Flash

| 函数 | 模式 |
|---|---|
| `McuCompiler_Clear` / `_DAP` | 全片擦 |
| `McuCompiler_ClearCodeFlash` / `B` / `_DAP` | Code/B sector 擦 |
| `McuCompiler_SetClearFlashPara` | 擦除参数 |
| `McuCompiler_Download` / `Ex` / `External` | 主下载路径 (Ex=最新) |
| `McuCompiler_Download_DAP` / `Ex` | DAP 下载 |
| `McuCompiler_GetHexOffsetAddr` | hex 偏移 |
| `McuCompiler_ReadDevMemory` / `_DAP` | 读回 |
| `McuCompiler_Update` | 完成并校验 |

### 3.5 保护位（Option Bytes）

| 函数 | 命令字节 | 说明 |
|---|---|---|
| `McuCompiler_CheckReadPro` / `_Ex` / `_DAP` | 0x1010681 / 0x4010681 | 查询 |
| `McuCompiler_EnableReadPro` | 0x3010681 | 启用 |
| `McuCompiler_DisableReadPro` | 0x2010681 | 关闭 |
| `McuCompiler_CheckQE` / `_DAP` | 0x6010d81 | 查询 QE |
| `McuCompiler_EnableQE` / `_DAP` | 0x7010d81 | 启用 QE |
| `McuCompiler_SetAccessAddress` / `_DAP` | — | 访问地址 option |
| `McuCompiler_SetMCUConfigValue` | — | 直接写选项字 |
| `McuCompiler_OptEnd` | — | 完成 option 写 |
| `MRSFunc_EnableRProtect` / `DisableRProtect` | — | MRS 包装 |
| `MRSFunc_QueryRProtect` | — |  |

### 3.6 调试 / 接口控制

| 函数 | 命令字节 | 说明 |
|---|---|---|
| `McuCompiler_DisableDebug` | 0x1010e81 | 关调试口 |
| `McuCompiler_SetSDIPrn` | 0xee020d81 | **SDI printf on/off** |
| `McuCompiler_SetSDLineMode` | 0xee020d81 | SDI 1-wire/2-wire |
| `McuCompiler_SetTwolineLowSpeed` | — | 低速模式 |
| `McuCompiler_SetRSTPin` / `_DAP` | — | RST 引脚电平 |
| `MRSFunc_DisableDbgInterface` | — | 包装 DisableDebug |
| `MRSFunc_DisablePowerOut` | — | 关 link 输出电 |

### 3.7 电压/辅助

| 函数 | 说明 |
|---|---|
| `McuCompiler_Opt33VOut` / `_DAP` | Link 输出 3.3V |
| `McuCompiler_Opt50VOut` / `_DAP` | Link 输出 5V |

### 3.8 加解密（**新发现**）

| 函数 | 用途 |
|---|---|
| `McuCompiler_EncryptData` | 数据加密 (未确认 AES 还是简单编码) |
| `McuCompiler_DisEncrypt` | 解密 |

`wlink` 没用到，可能是某些型号烧录前 data-in-flash 加密。

### 3.9 MRSFunc_* （MounRiver Studio 集成接口）

17 个 MRSFunc_, 都是上述 McuCompiler_* 的薄 wrapper。
MRS (官方 IDE) 通过这条窄入口调用完整流程。

### 3.10 Reset

| 函数 | 命令字节 | 说明 |
|---|---|---|
| `McuCompiler_Reset` | 0x1010b81 | 普通 reset |
| `McuCompiler_ResetB` | 0x6010b81 | "B" (硬 reset? 待验证) |
| `McuCompiler_Reset_DAP` | — (libusb) | DAP 下 reset |

## 4. GUI 启动流程

```
CWCHLinkUtilityApp.InitInstance
└─► CWCHLinkUtilityDlg 创建
    ├─ OnInitDialog
    │  └─ fcn.0040d180 = "fill chip combo":
    │      SendMessage(hwnd_combo=1018, CB_ADDSTRING, 0, "CH565/9")
    │      ... 20 个 family 字符串依次 push
    ├─ fcn.0040aee0 = "fill config-area sublist"
    │      SendMessage(hwnd, CB_ADDSTRING, ..., "Operation" / "User Option Bytes..." / etc)
    └─ virtual_372 ← 主菜单 handler
        通过 switch(menu_id) 分派到上述 McuCompiler_*。
```

## 5. 总图：GUI 用户操作 → DLL 调用

```
┌────────────────────────────────────────────────────────────────┐
│ 用户菜单             │ GUI handler            │ DLL 调用       │
├────────────────────────────────────────────────────────────────┤
│ 选 chip family    │ fcn.0040d180 (init)   │ SetTargetChip    │
│ 选 link           │ virtual_372 case      │ EnumLink / SetLinkIndex │
│ Open              │ 〃                     │ OpenDevice        │
│ Query Info        │ 〃                     │ GetChipInfo       │
│ Reset             │ 〃                     │ Reset / ResetB    │
│ Erase             │ 〃                     │ ClearCodeFlashB   │
│ Download          │ 〃                     │ DownloadEx        │
│ Verify            │ 〃                     │ ReadDevMemory     │
│ Option: RDPro on  │ 〃                     │ EnableReadPro[_Ex][_DAP] │
│ Option: QE on     │ 〃                     │ EnableQE[_DAP]    │
│ Option: SDI print │ 〃                     │ SetSDIPrn         │
│ Option: 1/2-wire  │ 〃                     │ SetSDLineMode/SetTwolineLowSpeed │
│ IAP               │ 〃                     │ SetIAPMode        │
│ Volts             │ 〃                     │ Opt33VOut / Opt50VOut │
└────────────────────────────────────────────────────────────────┘
```

详见 [03_command_table.md](03_command_table.md) 把命令字节也挂上。
