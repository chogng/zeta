"""Replay production terminal fixtures or verify a terminal's exported text buffer."""

import argparse
import json
import os
from pathlib import Path
import shutil
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["replay", "verify"])
    parser.add_argument("fixtures", type=Path)
    parser.add_argument("case", help="Fixture name, for example 40x5-120.ansi")
    parser.add_argument("--capture", type=Path)
    args = parser.parse_args()
    cases = json.loads((args.fixtures / "manifest.json").read_text(encoding="utf-8"))
    case = next(case for case in cases if case["file"] == args.case)
    if args.action == "replay":
        size = shutil.get_terminal_size()
        if (size.columns, size.lines) != (case["width"], case["height"]):
            parser.error(f"Expected {case['width']}x{case['height']}, got {size}")
        if os.name == "nt":
            import ctypes

            kernel = ctypes.windll.kernel32
            kernel.GetStdHandle.restype = ctypes.c_void_p
            handle = kernel.GetStdHandle(-11)
            mode = ctypes.c_ulong()
            if not kernel.GetConsoleMode(ctypes.c_void_p(handle), ctypes.byref(mode)):
                raise ctypes.WinError()
            if not kernel.SetConsoleMode(ctypes.c_void_p(handle), mode.value | 4):
                raise ctypes.WinError()
        sys.stdout.buffer.write((args.fixtures / args.case).read_bytes())
        sys.stdout.buffer.flush()
        # Keep the screen stable while the terminal exports its scrollback.
        sys.stdin.readline()
        return
    if args.capture is None:
        parser.error("verify requires --capture with UTF-8 plain text including scrollback")
    text = "".join(args.capture.read_text(encoding="utf-8").split())
    markers = ["ZETA-SHELL-SENTINEL", *case["markers"]]
    previous = -1
    for marker in markers:
        assert text.count(marker) == 1, f"{marker!r}: expected once, got {text.count(marker)}"
        position = text.index(marker)
        assert position > previous, f"{marker!r}: wrong conversation order"
        previous = position
    print(f"PASS {args.case}: shell history and all {len(case['markers'])} content markers retained in order")


if __name__ == "__main__":
    main()
