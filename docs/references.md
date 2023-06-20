# References

## Real Shot

![real shot](https://web.archive.org/web/20230613102346im_/https://www.wch.cn/uploads/image/20221230/1672381416120803.png)

## Feature matrix

| Feature                         |  WCH-Link  | WCH-LinkE  | WCH-LinkW  | WCH-DAPLink |
| ------------------------------- | :--------: | :--------: | :--------: | :---------: |
| RISC-V mode                     |  &check;   |  &check;   |  &check;   |             |
| ARM-SWD mode (HID device)       |            |            |            |   &check;   |
| ARM-SWD mode (WinUSB device)    |  &check;   |  &check;   |  &check;   |   &check;   |
| ARM-JTAG mode (HID device)      |            |            |            |   &check;   |
| ARM-JTAG mode (WinUSB device)   |            |  &check;   |  &check;   |   &check;   |
| ModeS button                    |            |  &check;   |  &check;   |   &check;   |
| DFU via 2-wire                  |  &check;   |            |            |             |
| DFU via serial-port             |  &check;   |            |            |             |
| DFU via USB                     |  &check;   |            |  &check;   |   &check;   |
| Power Supply (3.3v)             |  &check;   |  &check;   |  &check;   |   &check;   |
| Power Supply (5.0v)             |  &check;   |  &check;   |  &check;   |   &check;   |
| Power Supply (Configurable)     |            |  &check;   |  &check;   |   &check;   |
| USB 2.0 to JTAG                 |            |  &check;   |            |             |
| Wireless mode                   |            |            |  &check;   |             |
| Download via [MounRiver Studio] |  &check;   |  &check;   |  &check;   |   &check;   |
| Download via [WCH-LinkUtility]  |  &check;   |  &check;   |  &check;   |             |
| Download via [Keil]             | &ge; v5.25 | &ge; v5.25 | &ge; v5.25 |   &check;   |

## Supported Chip matrix

| Chip           | WCH-Link | WCH-LinkE | WCH-LinkW | WCH-DAPLink |
| -------------- | :------: | :-------: | :-------: | :---------: |
| JTAG interface |          |  &check;  |  &check;  |   &check;   |
| SWD interface  | &check;  |  &check;  |  &check;  |   &check;   |
| CH32F10x       | &check;  |  &check;  |  &check;  |   &check;   |
| CH32F20x       | &check;  |  &check;  |  &check;  |   &check;   |
| CH32V003       |          |  &check;  |  &check;  |             |
| CH32V10x       | &check;  |  &check;  |  &check;  |             |
| CH32V20x       | &check;  |  &check;  |  &check;  |             |
| CH32V30x       | &check;  |  &check;  |  &check;  |             |
| CH569          | &check;  |  &check;  |           |             |
| CH573          | &check;  |  &check;  |           |             |
| CH579          | &check;  |  &check;  |  &check;  |   &check;   |
| CH583          | &check;  |  &check;  |           |             |

## Supported Baud matrix

|   Baud | WCH-Link | WCH-LinkE | WCH-LinkW | WCH-DAPLink |
| -----: | :------: | :-------: | :-------: | :---------: |
|   1200 | &check;  |  &check;  |  &check;  |   &check;   |
|   2400 | &check;  |  &check;  |  &check;  |   &check;   |
|   4800 | &check;  |  &check;  |  &check;  |   &check;   |
|   9600 | &check;  |  &check;  |  &check;  |   &check;   |
|  14400 | &check;  |  &check;  |  &check;  |   &check;   |
|  19200 | &check;  |  &check;  |  &check;  |   &check;   |
|  38400 | &check;  |  &check;  |  &check;  |   &check;   |
|  57600 | &check;  |  &check;  |  &check;  |   &check;   |
| 115200 | &check;  |  &check;  |  &check;  |   &check;   |
| 230400 | &check;  |  &check;  |  &check;  |   &check;   |
| 460800 |          |  &check;  |  &check;  |   &check;   |
| 921600 |          |  &check;  |  &check;  |   &check;   |

## SWD PIN matrix

| Chip     | SWDIO | SWCLK |
| -------- | :---: | :---: |
| CH32F10x | PA13  | PA14  |
| CH32F20x | PA13  | PA14  |
| CH32V003 |  PD1  |       |
| CH32V10x | PA13  | PA14  |
| CH32V20x | PA13  | PA14  |
| CH32V30x | PA13  | PA14  |
| CH32X035 | PC18  | PC19  |
| CH569    | PA11  | PA10  |
| CH573    | PB14  | PB15  |
| CH579    | PB16  | PB17  |
| CH583    | PB14  | PB15  |
| CH59x    | PB14  | PB15  |
| CH643    | PC18  | PC19  |

## Documentation

- [WCH-Link 相关资料汇总](https://web.archive.org/web/20230613102346/https://www.wch.cn/bbs/thread-71088-1.html)
- [WCH-Link 使用说明 v1.7](https://web.archive.org/web/20230613114619if_/https://www.wch.cn/downloads/file/417.html?time=2023-06-13%2019:46:05&code=1BaRkx0gWHP7accBAPUtCuJ0dk0emAIzZ85o8UIf)
- [WCH-LinkSCH.pdf](https://web.archive.org/web/20230613133629/https://www.wch.cn/downloads/file/421.html?time=2023-06-13%2021:35:48&code=CA0Mz2JvD7YBhFB9t8jVb3MhgGgZV4fxg23Ku5B6)
- [User Manual (Chinese)](https://web.archive.org/web/20230613102015if_/https://www.wch.cn/downloads/file/417.html?time=2023-06-13%2018:19:04&code=z6nAIBmh1M4Uv64xdbCeAwywfJ9OEPG6OBvdUz1A)
- [User Manual (English)](https://web.archive.org/web/20230613102158if_/http://www.wch-ic.com/downloads/file/372.html?time=2023-06-13%2018:20:36&code=uRfQmamyIynlCZPHO33rloOWiCgb44NLTXxStO8l)

## ISP

- [WCHISPTool_Setup.exe](https://web.archive.org/web/20220811233210if_/https://www.wch.cn/downloads/file/196.html?time=2022-06-30%2014:56:16&code=LS2LHywwDiw3P71gxsM1hfZClwSQlbI4nQga1Kzo) v3.3

## Firmware

- [WCH_RISC-V_MCU_ProgramTool.zip](https://web.archive.org/web/20230613112000if_/https://www.wch.cn/uploads/file/20220628/1656415558432295.zip)
- [WCH-Link v2.3](https://web.archive.org/web/20230613112654if_/https://www.wch.cn/uploads/file/20220718/1658124411917956.zip)
- [WCH-LinkE v1.1](https://web.archive.org/web/20230613112104if_/https://www.wch.cn/uploads/file/20220913/1663036474195451.zip)

## Other FOSS implementation

- <https://github.com/openwch/ch32v003> RISC-V QingKeV2 Microprocessor Debug Manual
- <https://github.com/cnlohr/ch32v003fun> A miniwchlink implementation
- <https://github.com/blackmagic-debug/blackmagic/pull/1399>
- [MounRiver Studio] compatible WCH-Link OpenOCD source code <https://github.com/xu7wong/openocd_wchlink> \
   from <https://www.wch.cn/bbs/thread-91946-1.html>

[MounRiver Studio]: http://www.mounriver.com "MounRiver Studio"
[WCH-LinkUtility]: https://web.archive.org/web/20230613114515if_/https://www.wch.cn/downloads/file/418.html?time=2023-06-13%2019:44:31&code=z88GXEXY3kNBV9rTwDe0iWerDk5iKHB50lkst8j8 "WCH LinkUtility"
[Keil]: https://www.keil.com "Keil Embedded Development Tools"
