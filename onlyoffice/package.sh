#!/bin/sh
# Packs the ONLYOFFICE plugin: a .plugin file is a zip of the plugin folder's contents.
set -e
cd "$(dirname "$0")"
mkdir -p ../dist
rm -f ../dist/naqsha-onlyoffice.plugin
zip -qr -X ../dist/naqsha-onlyoffice.plugin config.json index.html scripts resources
echo "Built dist/naqsha-onlyoffice.plugin"
