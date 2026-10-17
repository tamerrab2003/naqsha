Title: Add Naqsha plugin (editable diagrams: flowcharts, P&IDs, single-line diagrams)

## Naqsha

Naqsha is a diagram editor (flowcharts, cloud and software architecture, P&IDs, electrical
single-line diagrams). This plugin lets ONLYOFFICE users insert Naqsha diagrams into documents,
spreadsheets and presentations and keep them editable, much like the draw.io plugin:

- **Plugins › Naqsha** opens the editor. Pick a template or draw, then press **Insert**.
- The diagram is stored as an OLE object: its picture is what the document shows and prints,
  and its data is the whole diagram (all pages), so **double-clicking it** opens it again for editing.
- Works in the Document, Spreadsheet and Presentation editors. UI strings are translated into
  Arabic, Chinese, Czech, French, German, Italian, Japanese, Portuguese (Brazil), Russian and Spanish.

### How it works

The plugin shows the Naqsha web editor (https://nixpx.com/naqsha/app/, the same build as the
desktop app) in the plugin window and talks to it with `postMessage`. On Insert the editor returns
the diagram JSON and a PNG of the page, which go to `AddOleObject` / `EditOleObject`. Nothing is
sent anywhere else: the diagram lives in the document.

- An internet connection to nixpx.com is needed (like the draw.io plugin's online mode).
- No API key is needed. The core editor is free; some engineering templates and tools are part
  of Naqsha Pro, which has a free 14-day trial inside the editor.
- Source: https://github.com/tamerrab2003/naqsha (the `onlyoffice/` folder, MIT licence).

### Tested

ONLYOFFICE Desktop Editors on macOS: insert a new diagram, double-click to edit and update it,
save and reopen the .docx.

### Contact

Nixpx LLC, @tamerrab2003 on GitHub
