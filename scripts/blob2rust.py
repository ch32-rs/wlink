#!/usr/bin/env python3
"""Convert a flash_op blob (.bin) into a Rust `pub const [u8; N]` block.

Usage:
    python3 scripts/blob2rust.py <bin-path> <CONST_NAME> [--va 0xADDR] [--chip-id 0xNN]

Output is printed to stdout; expected to be piped into src/flash_op.rs review.
The provenance header records DLL virtual address and chip_id so reviewers
can confirm the blob by re-extracting from McuCompilerDll.dll.
"""

import argparse
import sys
from pathlib import Path


def format_const(name: str, data: bytes, va: str | None, chip_id: str | None) -> str:
    header_lines = [f"// Blob extracted from McuCompilerDll.dll fcn.10003810 jump table."]
    if va:
        header_lines.append(f"// DLL VA: {va}")
    if chip_id:
        header_lines.append(f"// chip_id: {chip_id}")
    header_lines.append(f"// Size: {len(data)} bytes")
    header = "\n".join(header_lines)

    body_lines = []
    for i in range(0, len(data), 16):
        chunk = data[i : i + 16]
        body_lines.append("    " + ", ".join(f"0x{b:02x}" for b in chunk) + ",")
    body = "\n".join(body_lines)

    return (
        f"{header}\n"
        f"pub const {name}: [u8; {len(data)}] = [\n"
        f"{body}\n"
        f"];\n"
    )


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("bin", type=Path, help="Input .bin blob")
    p.add_argument("name", help="Rust const name, e.g. CH585")
    p.add_argument("--va", help="DLL virtual address, e.g. 0x10172878")
    p.add_argument("--chip-id", help="chip_id byte, e.g. 0x4b")
    args = p.parse_args()

    if not args.bin.is_file():
        print(f"error: {args.bin} not found", file=sys.stderr)
        return 1

    data = args.bin.read_bytes()
    if not data:
        print(f"error: {args.bin} is empty", file=sys.stderr)
        return 1

    sys.stdout.write(format_const(args.name, data, args.va, args.chip_id))
    return 0


if __name__ == "__main__":
    sys.exit(main())
