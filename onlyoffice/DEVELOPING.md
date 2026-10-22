# Naqsha for ONLYOFFICE: notes for maintainers

(The plugin's `README.md` is for users and goes to the ONLYOFFICE marketplace; this file stays in
the Naqsha repository.)

## Package

```sh
./onlyoffice/package.sh        # → dist/naqsha-onlyoffice.plugin
```

The release workflow attaches it to every GitHub release as `naqsha-onlyoffice.plugin`.

## How it works

`scripts/naqsha.js` loads `https://nixpx.com/naqsha/app/?embed=onlyoffice` in an iframe and talks to
it with `postMessage` (the messages are described in `src/embed.rs`). It passes the editor's theme
on (`{naqsha: "theme", type}`, on load and in `onThemeChanged`). On Insert the editor sends the
whole diagram (`.nqs` JSON) and a PNG of the current page; the plugin checks that the picture is a
`data:image/png;base64,` URL and the size is positive, then stores them as an OLE object
(`AddOleObject`, or `EditOleObject` for a diagram being edited).

The editor address and origin are fixed in the script; there is no override in the release build.

The plugin GUID, `asc.{DA395EC3-313E-48F3-BE8D-A4217BE92C27}`, is written into every document that
holds a Naqsha diagram: never change it.

## Publishing to the ONLYOFFICE marketplace

1. Fork <https://github.com/ONLYOFFICE/onlyoffice.github.io> and clone the fork.
2. `./onlyoffice/marketplace.sh <path to the clone>`: copies the plugin to
   `sdkjs-plugins/content/naqsha` and adds it to `store/config.json`.
3. Commit, push, and open a pull request with the text in `PULL_REQUEST.md`.

For an update, raise `version` in `config.json`, add a `CHANGELOG.md` entry, and repeat.
