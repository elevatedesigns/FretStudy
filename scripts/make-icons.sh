#!/bin/sh
# Rebuilds every icon file from the Swift drawing: assets/icon-1024.png, icon-256.png
# (window icon), FretStudy.icns (macOS), and fretstudy.ico (Windows).
set -eu
cd "$(dirname "$0")/.."
swift scripts/make-icon.swift assets/icon-1024.png
SET=$(mktemp -d)/FretStudy.iconset
mkdir -p "$SET"
for s in 16 32 128 256 512; do
    sips -z $s $s assets/icon-1024.png --out "$SET/icon_${s}x${s}.png" >/dev/null
    d=$((s * 2))
    sips -z $d $d assets/icon-1024.png --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o assets/FretStudy.icns
sips -z 256 256 assets/icon-1024.png --out assets/icon-256.png >/dev/null
# Windows .ico: PNG images packed into an ICO container.
ICO_DIR=$(mktemp -d)
for s in 16 24 32 48 64 128 256; do
    sips -z $s $s assets/icon-1024.png --out "$ICO_DIR/$s.png" >/dev/null
done
python3 - "$ICO_DIR" assets/fretstudy.ico <<'PY'
import struct, sys, os
src, out = sys.argv[1], sys.argv[2]
sizes = [16, 24, 32, 48, 64, 128, 256]
blobs = [open(os.path.join(src, f"{s}.png"), "rb").read() for s in sizes]
header = struct.pack("<HHH", 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
entries = b""
for s, b in zip(sizes, blobs):
    entries += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(b), offset)
    offset += len(b)
open(out, "wb").write(header + entries + b"".join(blobs))
PY
echo "Icons written to assets/"
