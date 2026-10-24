#!/usr/bin/env bash
# Builds the browser version of Naqsha into dist/web (and dist/naqsha-web.zip).
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
# Optional: wasm-opt (binaryen) for a smaller download.
# Every path inside is relative, so the folder can be served from any sub-path (e.g. /naqsha/app/).
set -euo pipefail
cd "$(dirname "$0")"
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
OUT=dist/web

# FEATURES=pro builds Naqsha Pro in (needs the private pro/ submodule)
cargo build --profile web --target wasm32-unknown-unknown ${FEATURES:+--features $FEATURES} ${BUILD_STD:+-Zbuild-std=std,panic_abort}
rm -rf "$OUT" && mkdir -p "$OUT"
wasm-bindgen --target web --no-typescript --out-dir "$OUT" --out-name naqsha \
  target/wasm32-unknown-unknown/web/naqsha.wasm
if command -v wasm-opt >/dev/null; then
  if wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals "$OUT/naqsha_bg.wasm" -o "$OUT/naqsha_bg.wasm.opt"; then
    mv "$OUT/naqsha_bg.wasm.opt" "$OUT/naqsha_bg.wasm"
  else
    rm -f "$OUT/naqsha_bg.wasm.opt"; echo "(wasm-opt failed: keeping the unoptimised build)"
  fi
else
  echo "(wasm-opt not found: skipping size optimisation)"
fi
# the version in the URLs makes browsers fetch new code after an update, while the files
# themselves can be cached for a year
sed "s/__VERSION__/$VERSION/g" web/index.html > "$OUT/index.html"
cp web/icon.svg "$OUT/"
# Chinese/Japanese font: downloaded by the app when a drawing needs it (see src/scripts.rs)
mkdir -p "$OUT/fonts" && cp web/fonts/NotoSansCJK-sub.otf web/fonts/LICENSE-Noto.txt "$OUT/fonts/"
(cd "$OUT" && rm -f ../naqsha-web.zip && zip -qr ../naqsha-web.zip .)
ls -l "$OUT"
echo "Built $OUT and dist/naqsha-web.zip (Naqsha $VERSION)"
