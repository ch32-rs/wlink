# CH57X & CH58X & CH59X

BLE MCUs. They are sharing almost the same preipheral set

## Debug Pins

- PB15 TCK
- PB14 TIO

## Notes

- Enable debug using WCHISPTool
- Always specify chip type using `--chip xxx` parameter during connect, auto-detection mechanism does not work on these targets
- Similiarly, use `-c "chip_id CH59x"` while using official OpenOCD release from MRS, as there's no way to know the chip type before connecting to it
- After debug enabled, flash should be prgrammed at lease once
- `a9 bd f9 f3` means erased flash or protected flash. The detailed mechanism is unknown
