#!/bin/sh
# Builds dist/FretStudy.app.
#
#   scripts/bundle-macos.sh             this Mac's chip only (fast, for testing)
#   scripts/bundle-macos.sh --universal Apple Silicon + Intel in one app (for releases)
#
# macOS only lets an app use the microphone if its Info.plist explains why, so the binary
# is wrapped in an app bundle. It is signed ad hoc unless FRETSTUDY_SIGN_IDENTITY names a
# "Developer ID Application" certificate (see scripts/package-macos.sh and docs/04).
set -eu
cd "$(dirname "$0")/.."

UNIVERSAL=0
[ "${1:-}" = "--universal" ] && UNIVERSAL=1

if [ "$UNIVERSAL" = 1 ]; then
    cargo build --release -p fretstudy --target aarch64-apple-darwin
    cargo build --release -p fretstudy --target x86_64-apple-darwin
    BIN=$(mktemp -d)/FretStudy
    lipo -create -output "$BIN" \
        target/aarch64-apple-darwin/release/fretstudy \
        target/x86_64-apple-darwin/release/fretstudy
else
    cargo build --release -p fretstudy
    BIN=target/release/fretstudy
fi

APP="dist/FretStudy.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/FretStudy"
cp assets/FretStudy.icns "$APP/Contents/Resources/FretStudy.icns"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/app/Cargo.toml | head -1)
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>FretStudy</string>
    <key>CFBundleDisplayName</key><string>FretStudy</string>
    <key>CFBundleIdentifier</key><string>com.fretstudy.app</string>
    <key>CFBundleExecutable</key><string>FretStudy</string>
    <key>CFBundleIconFile</key><string>FretStudy</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSMicrophoneUsageDescription</key>
    <string>FretStudy listens to your guitar to tune it and to check the notes and rhythms you play. Nothing is recorded or sent anywhere unless you choose to save a take.</string>
</dict>
</plist>
PLIST

if [ -n "${FRETSTUDY_SIGN_IDENTITY:-}" ]; then
    # Hardened runtime (needed for notarization) with permission to use the microphone.
    codesign --force --options runtime --timestamp \
        --entitlements scripts/macos-entitlements.plist \
        --sign "$FRETSTUDY_SIGN_IDENTITY" "$APP"
    echo "Signed with: $FRETSTUDY_SIGN_IDENTITY"
else
    codesign --force --sign - "$APP"
    echo "Signed ad hoc (fine on this Mac; other Macs will warn). Set FRETSTUDY_SIGN_IDENTITY to sign for release."
fi
echo "Built $APP"
