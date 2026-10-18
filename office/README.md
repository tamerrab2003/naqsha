# Naqsha for Microsoft Office

An Office add-in for Word, Excel and PowerPoint (Windows, Mac and the web). **Insert › Naqsha
Diagram** opens a task pane; **New diagram** opens the Naqsha editor in a large dialog, and
**Insert** puts the diagram into the file as a picture. Select the picture later and choose
**Edit selected diagram** to change it.

The pages in this folder are served from <https://nixpx.com/naqsha/office/> (copied to the nixpx
site's `public/naqsha/office/`); `manifest.xml` points there.

## Try it (sideloading)

- **Office on the web:** open a document, then Insert › Add-ins › Advanced (or More Add-ins) ›
  Upload My Add-in, and choose `manifest.xml`.
- **Mac:** copy `manifest.xml` into the app's `wef` folder (create it if it isn't there) and
  restart the app:
  - Word: `~/Library/Containers/com.microsoft.Word/Data/Documents/wef/`
  - Excel: `~/Library/Containers/com.microsoft.Excel/Data/Documents/wef/`
  - PowerPoint: `~/Library/Containers/com.microsoft.Powerpoint/Data/Documents/wef/`

  Then Insert › Add-ins (or My Add-ins) › Developer Add-ins › Naqsha.
- **Windows:** Insert › Add-ins › My Add-ins › Upload My Add-in, or a shared-folder catalog.

The manifest is also at <https://nixpx.com/naqsha/office/manifest.xml>.

## How it works

- `taskpane.html` / `taskpane.js`: the buttons, and putting pictures into the file.
- `dialog.html` / `dialog.js`: shows the editor (`../app/?embed=office`) and passes messages
  between it (`postMessage`, see `src/embed.rs`) and the task pane (`messageParent` /
  `messageChild`, in parts so no size limit is hit).

The diagram itself (`.nqs` JSON) is kept in the file, in the add-in's document settings under
`naqsha:<id>`. The picture carries the id:

| App | Where the id is |
|---|---|
| Word | the picture's alt text description (`… naqsha:<id>`) |
| PowerPoint | a shape tag `NAQSHA`, and the shape name |
| Excel | the shape name (`Naqsha diagram naqsha:<id>`) |

A picture copied into another file keeps the id but not the diagram, so it can't be edited there.

Requirements: WordApi 1.2; PowerPointApi 1.5 to edit (older PowerPoint can only insert); ExcelApi
1.9 (1.19 finds the selected diagram; before that the task pane lists the sheet's diagrams);
DialogApi 1.2 to edit when the dialog doesn't share storage with the task pane.

Keep the `<Id>` in `manifest.xml`: Office and the store know the add-in by it.
