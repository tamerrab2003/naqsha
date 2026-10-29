#!/usr/bin/env bash
# Builds Naqsha.app (universal: Apple Silicon + Intel) and Naqsha.dmg.
# Usage: ./build-mac.sh            -> dist/Naqsha.app + dist/Naqsha-<ver>.dmg
#        ./build-mac.sh --native   -> only your Mac's architecture (faster)
#        ./build-mac.sh --install  -> also copies the app into /Applications
set -euo pipefail
cd "$(dirname "$0")"

if ! command -v cargo >/dev/null; then
  echo "Rust is not installed. Install it with:"
  echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
APP=dist/Naqsha.app
NATIVE=0; INSTALL=0
for a in "$@"; do [[ $a == --native ]] && NATIVE=1; [[ $a == --install ]] && INSTALL=1; done

if [[ $NATIVE == 1 ]]; then
  cargo build --release ${FEATURES:+--features $FEATURES}
  BIN=target/release/naqsha
else
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release --target aarch64-apple-darwin ${FEATURES:+--features $FEATURES}
  cargo build --release --target x86_64-apple-darwin ${FEATURES:+--features $FEATURES}
  mkdir -p target/universal
  lipo -create -output target/universal/naqsha \
    target/aarch64-apple-darwin/release/naqsha target/x86_64-apple-darwin/release/naqsha
  BIN=target/universal/naqsha
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/naqsha"
cp assets/Naqsha.icns "$APP/Contents/Resources/Naqsha.icns"
sed "s/__VERSION__/$VERSION/g" macos/Info.plist > "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"

# Ad-hoc signature so Gatekeeper lets it run locally (replace "-" with a Developer ID to distribute).
codesign --force --deep --sign "${SIGN_IDENTITY:--}" "$APP"

DMG="dist/Naqsha-$VERSION.dmg"
rm -f "$DMG"
STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
# hdiutil's own size estimate for -srcfolder is sometimes too small ("create failed - No space left
# on device"), so give the image room explicitly, and retry: the CI runners' hdiutil is also flaky.
SIZE_MB=$(( $(du -sm "$STAGE" | cut -f1) * 13 / 10 + 32 ))
for try in 1 2 3; do
  if hdiutil create -volname "Naqsha" -srcfolder "$STAGE" -size "${SIZE_MB}m" -fs HFS+ -ov -format UDZO "$DMG" >/dev/null; then
    break
  fi
  echo "hdiutil create failed (try $try of 3, image ${SIZE_MB} MB)"; df -h "$STAGE" . | sed 's/^/  /'
  [[ $try == 3 ]] && exit 1
  rm -f "$DMG"; sleep 10
done
rm -rf "$STAGE"

if [[ $INSTALL == 1 ]]; then
  rm -rf /Applications/Naqsha.app
  cp -R "$APP" /Applications/
  echo "Installed to /Applications/Naqsha.app"
  if [[ -d /Applications/Schemix.app ]]; then
    echo "Note: the old /Applications/Schemix.app is still there. Your settings and recent files"
    echo "move over to Naqsha on first launch; delete Schemix.app once you're happy."
  fi
fi
echo "Built $APP and $DMG"
