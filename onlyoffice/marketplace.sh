#!/bin/sh
# Puts the plugin into a clone of your fork of ONLYOFFICE/onlyoffice.github.io, ready for a pull
# request to the ONLYOFFICE plugin marketplace:
#   - sdkjs-plugins/content/naqsha/  the plugin (ONLYOFFICE packs it themselves)
#   - store/config.json              registers it in the Plugin Manager
#
#   ./onlyoffice/marketplace.sh ~/work/onlyoffice.github.io
set -e
FORK="${1:?usage: marketplace.sh <path to your onlyoffice.github.io clone>}"
[ -f "$FORK/store/config.json" ] || { echo "$FORK doesn't look like an onlyoffice.github.io clone"; exit 1; }
cd "$(dirname "$0")"

DEST="$FORK/sdkjs-plugins/content/naqsha"
rm -rf "$DEST"
mkdir -p "$DEST"
cp -R config.json index.html scripts resources translations CHANGELOG.md LICENSE README.md "$DEST/"

python3 - "$FORK/store/config.json" <<'EOF'
import json, sys
p = sys.argv[1]
text = open(p, encoding="utf-8").read()
if '"naqsha"' in text:
    print("store/config.json already lists naqsha")
else:
    json.loads(text)                       # still valid before we touch it
    i = text.rstrip().rindex("]")
    body = text[:i].rstrip()
    text = body + ',\n\t{ "name": "naqsha", "discussion": "" }\n' + text[i:]
    json.loads(text)                       # and after
    open(p, "w", encoding="utf-8").write(text)
    print("added naqsha to store/config.json")
EOF
echo "Copied the plugin to $DEST"
