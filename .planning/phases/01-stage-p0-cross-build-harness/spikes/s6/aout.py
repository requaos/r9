#!/usr/bin/env python3
"""S6 a.out wrapper/checker (D-17).

Builds and validates the 40-byte Plan 9 a.out header r9's loaders accept:

  8 x big-endian u32: magic, text, data, bss, syms, entry32 (vestigial),
                      spsz, pcsz
  1 x big-endian u64: entry (bytes 32..39)

Facts mirrored from r9's parsers at HEAD:
  x86_64/src/exec.rs:599-615, 666-696  (S_MAGIC_AMD64 0x8A97,
      HDR_SIZE 40, INITTEXT 0x200028, entry in text, text != 0)
  aarch64/src/exec.rs:41-51, 191-222   (R_MAGIC_AARCH64 0x8C47,
      HDR_SIZE 40, INITTEXT 0x10028, entry in text, text != 0)

The flat payload is the whole text segment; the probe has no data, bss
or symbol table, and the check asserts exactly that.

Run as `python3 -I`; standard library only.
"""

import argparse
import struct
import sys
from pathlib import Path

ARCHES = {
    "amd64": {"magic": 0x8A97, "initttext": 0x200028},
    "arm64": {"magic": 0x8C47, "initttext": 0x10028},
}

HDR_SIZE = 40
U32_MAX = 0xFFFF_FFFF


def die(msg: str) -> "None":
    print(f"aout.py: {msg}", file=sys.stderr)
    raise SystemExit(2)


def read_flat(path: str) -> bytes:
    payload = Path(path).read_bytes()
    if not payload:
        die(f"{path}: empty payload (a zero-length text segment is rejected by parse_aout)")
    if len(payload) > U32_MAX:
        die(f"{path}: payload {len(payload)} bytes exceeds the a.out u32 text field")
    return payload


def cmd_wrap(args: argparse.Namespace) -> int:
    arch = ARCHES[args.arch]
    payload = read_flat(args.flat)
    entry = arch["initttext"]
    header = struct.pack(
        ">IIIIIIII",
        arch["magic"],
        len(payload),
        0,  # data
        0,  # bss
        0,  # syms
        entry & U32_MAX,  # vestigial entry32
        0,  # spsz
        0,  # pcsz
    ) + struct.pack(">Q", entry)
    assert len(header) == HDR_SIZE
    Path(args.out).write_bytes(header + payload)
    print(
        f"WRAP-OK {args.arch} text={len(payload)} entry=0x{entry:x} "
        f"file={HDR_SIZE + len(payload)} out={args.out}"
    )
    return 0


def fail(rule: str, detail: str) -> int:
    print(f"AOUT-FAIL {rule}: {detail}")
    return 1


def cmd_check(args: argparse.Namespace) -> int:
    arch = ARCHES[args.arch]
    raw = Path(args.aout).read_bytes()
    if len(raw) < HDR_SIZE:
        return fail("short-header", f"{len(raw)} bytes, need {HDR_SIZE}")
    magic, text, data, bss, syms, _entry32, _spsz, _pcsz = struct.unpack(
        ">IIIIIIII", raw[:32]
    )
    entry = struct.unpack(">Q", raw[32:40])[0]

    if magic != arch["magic"]:
        return fail("magic", f"0x{magic:08x}, want 0x{arch['magic']:08x}")
    if text == 0:
        return fail("empty-text", "text field is 0")
    base = arch["initttext"]
    if not (base <= entry < base + text):
        return fail(
            "entry-outside-text",
            f"entry=0x{entry:x}, text=[0x{base:x}, 0x{base + text:x})",
        )
    declared = HDR_SIZE + text + data + syms
    if len(raw) != HDR_SIZE + text + data:
        return fail(
            "file-length",
            f"{len(raw)} bytes, want {HDR_SIZE} + text({text}) + data({data}) = "
            f"{HDR_SIZE + text + data}",
        )
    if data != 0:
        return fail("data-nonzero", f"data={data} (the probe is flat text only)")
    if bss != 0:
        return fail("bss-nonzero", f"bss={bss} (the probe is flat text only)")
    if syms != 0:
        return fail("syms-nonzero", f"syms={syms} (the probe carries no symbol table)")

    print(f"AOUT-OK {args.arch} text={text} entry=0x{entry:x}")
    return 0


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(description="S6 Plan 9 a.out wrapper/checker")
    sub = parser.add_subparsers(dest="cmd", required=True)

    wrap = sub.add_parser("wrap", help="prepend the 40-byte a.out header")
    wrap.add_argument("--arch", choices=sorted(ARCHES), required=True)
    wrap.add_argument("flat")
    wrap.add_argument("out")
    wrap.set_defaults(func=cmd_wrap)

    check = sub.add_parser("check", help="assert r9's parse_aout rules")
    check.add_argument("--arch", choices=sorted(ARCHES), required=True)
    check.add_argument("aout")
    check.set_defaults(func=cmd_check)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
