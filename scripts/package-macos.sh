#!/bin/sh
# Makes the macOS release: a universal FretStudy.app inside dist/FretStudy-<version>.dmg.
#
# Without any setup, the app is signed ad hoc: it runs on this Mac, and other Macs show a
# "can't be opened" warning (right-click → Open gets past it).
#
# For a public release (no warnings), join the Apple Developer Program, then:
#   export FRETSTUDY_SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
#   xcrun notarytool store-credentials fretstudy-notary   # once: saves your Apple ID login
#   export FRETSTUDY_NOTARY_PROFILE=fretstudy-notary
#   scripts/package-macos.sh
set -eu
cd "$(dirname "$0")/.."

scripts/bundle-macos.sh --universal

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/app/Cargo.toml | head -1)
DMG="dist/FretStudy-${VERSION}.dmg"
STAGE=$(mktemp -d)
cp -R dist/FretStudy.app "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -volname "FretStudy" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null

if [ -n "${FRETSTUDY_SIGN_IDENTITY:-}" ]; then
    codesign --force --timestamp --sign "$FRETSTUDY_SIGN_IDENTITY" "$DMG"
fi
if [ -n "${FRETSTUDY_NOTARY_PROFILE:-}" ]; then
    xcrun notarytool submit "$DMG" --keychain-profile "$FRETSTUDY_NOTARY_PROFILE" --wait
    xcrun stapler staple "$DMG"
    echo "Notarized and stapled."
fi
echo "Built $DMG ($(du -h "$DMG" | cut -f1))"
