#!/bin/sh
# Makes the Linux release (x86_64):
#   dist/FretStudy-Linux-x86_64.AppImage   one file: chmod +x, then double-click or run it
#   dist/FretStudy-Linux-x86_64.tar.gz     the plain binary plus a .desktop file and icon
#
# Needs: Rust, plus the ALSA and X11/Wayland development packages, e.g. on Ubuntu:
#   sudo apt install pkg-config libasound2-dev libxkbcommon-dev libwayland-dev \
#        libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev
# The AppImage step downloads appimagetool if it isn't on PATH.
set -eu
cd "$(dirname "$0")/.."

cargo build --release -p fretstudy
mkdir -p dist

# Plain tarball.
STAGE=$(mktemp -d)/FretStudy
mkdir -p "$STAGE"
cp target/release/fretstudy "$STAGE/fretstudy"
cp assets/fretstudy.desktop "$STAGE/"
cp assets/icon-256.png "$STAGE/fretstudy.png"
tar -C "$(dirname "$STAGE")" -czf dist/FretStudy-Linux-x86_64.tar.gz FretStudy

# AppImage.
APPDIR=$(mktemp -d)/FretStudy.AppDir
mkdir -p "$APPDIR/usr/bin"
cp target/release/fretstudy "$APPDIR/usr/bin/fretstudy"
cp assets/fretstudy.desktop "$APPDIR/fretstudy.desktop"
cp assets/icon-256.png "$APPDIR/fretstudy.png"
ln -s usr/bin/fretstudy "$APPDIR/AppRun"

TOOL=$(command -v appimagetool || true)
if [ -z "$TOOL" ]; then
    TOOL=$(mktemp -d)/appimagetool
    curl -fsSL -o "$TOOL" \
        https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
fi
# --appimage-extract-and-run lets it work without FUSE (e.g. in CI containers).
ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$APPDIR" dist/FretStudy-Linux-x86_64.AppImage

echo "Built dist/FretStudy-Linux-x86_64.AppImage and dist/FretStudy-Linux-x86_64.tar.gz"
