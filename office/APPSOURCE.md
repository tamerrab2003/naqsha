# AppSource listing: Naqsha for Microsoft Office

What to enter in Partner Center (Marketplace offers › Microsoft 365 and Copilot › Office add-in).
Field names follow the submission form; check the character limits it shows.

## Offer setup

- **Offer name:** Naqsha (must match `DisplayName` in `manifest.xml`)
- **Publisher name:** Nixpx LLC (must match `ProviderName` in `manifest.xml`)
- **Package:** `manifest.xml` (from this folder, or https://nixpx.com/naqsha/office/manifest.xml)
- **Additional purchases:** yes — Naqsha Pro is a paid subscription or licence, bought on our site
  through Lemon Squeezy

## Properties

- **Categories** (pick from the form's list): Productivity; Visualization (or Data visualization);
  Content creation / Editing tools
- **Industries** (optional): Manufacturing & Resources; Energy; Government
- **Legal / EULA:** https://nixpx.com/naqsha/terms
- **Privacy policy:** https://nixpx.com/naqsha/privacy
- **Support:** https://nixpx.com/naqsha/support

## Listing (English)

**Search results summary / short description**

> Draw flowcharts, P&IDs and single-line diagrams that stay editable in Word, Excel and PowerPoint.

**Search keywords** (up to 3): diagram, flowchart, P&ID

**Description** (HTML; paste into the form, it has no preview):

```html
<p><b>Naqsha puts real diagrams in your Office files and keeps them editable.</b> Draw a flowchart for a procedure, a process flow for a design report, an electrical single-line diagram for a tender, or an architecture diagram for a slide, and insert it as a sharp picture. Select it later and choose <i>Edit selected diagram</i> to change it in Naqsha: the diagram itself is kept inside your document, workbook or presentation.</p>
<p><b>What you get</b></p>
<ul>
<li>A full diagram editor in a large window: shapes, connectors that route around obstacles, containers and swimlanes, layers, multiple pages, auto-arrange, and undo.</li>
<li>Templates: flowcharts, cross-functional (swimlane) flowcharts, CI/CD pipelines, cloud and software architecture, C4 diagrams and more.</li>
<li>Works in Word, Excel and PowerPoint on Windows, Mac and the web.</li>
<li>Your diagrams stay in your file. Nothing is uploaded, and there is no account to create.</li>
</ul>
<p><b>Naqsha Pro (optional, paid)</b></p>
<p>Naqsha is free to use. Naqsha Pro adds tools for engineers who deliver drawings: P&amp;ID, water and wastewater, IEC electrical, production line and IoT symbols with automatic tags; line balancing and value stream maps; sheets with your title block; drawing sets with a legend and register; equipment, instrument and cable schedules; and a bill of materials from your own product list. Pro starts with a 14-day free trial inside the add-in (no payment details needed). After that it is a subscription ($8 a month or $79 a year) or a lifetime licence ($129, with a year of updates), bought at nixpx.com/naqsha through our reseller Lemon Squeezy. One licence works in the add-in, the browser version and the desktop apps for Mac, Windows and Linux.</p>
<p><b>Requirements:</b> an internet connection (the editor is loaded from nixpx.com). Editing diagrams again needs Word 2016 or later, PowerPoint with PowerPoint API 1.5, or Excel with Excel API 1.9; the newest Office versions find the selected diagram automatically.</p>
```

**Screenshots** (1366 × 768 PNG, up to 5; use the clean sample files, never client work: *Sample
report.docx*, *Sample deck.pptx* and *Sample equipment list.xlsx*):

1. Word (*Sample report*): the Naqsha editor window open over the report, showing the Pump Station
   P&ID template.
2. Word: the P&ID inserted under "Figure 1", task pane open with *Edit selected diagram* highlighted.
3. PowerPoint (*Sample deck*, slide 2): the IIoT Monitoring Architecture template on the slide.
4. Excel (*Sample equipment list*): a pump station diagram next to the equipment table.
5. The editor with a production line template (Beverage Bottling Line) and Pro › Line Balance open.

**Logo:** 300 × 300 PNG: `assets/icon-300.png`.

## Notes for certification

Paste into *Notes for certification* (fill in the key — never commit it here):

```
How to test

1. Open Word, Excel or PowerPoint (web or desktop) and start Naqsha from Insert > Naqsha Diagram.
2. In the task pane, click "New diagram". The Naqsha editor opens in a dialog with a template gallery.
   Choose "Basic Flowchart" and click Create, then click Insert (bottom right of the dialog).
   The diagram is inserted as a picture.
3. Select the inserted picture. In the task pane, "Edit selected diagram" becomes the main button;
   click it. The same diagram opens in the editor. Change something (e.g. drag a shape) and click
   Insert: the picture is replaced in place, keeping its size.
4. Save, close and reopen the file; select the diagram and click Edit again: it opens with your changes.
5. Cancel in the dialog closes it without changing the file.

Free vs Pro
- All of the above is free. Naqsha Pro (paid) adds engineering templates and tools; it is bought on
  our website (nixpx.com/naqsha, via Lemon Squeezy), not in the add-in.
- To test Pro: in the editor dialog, open the "Pro" tab > Licence, paste this test licence key and
  click Activate:  <TEST LICENCE KEY>
  Alternatively click "Start the free 14-day trial". Then File > New > Water & Process or Electrical
  templates, and the Pro tab tools (Title Block, PDF Set, DXF, Schedules, BOM) are available.

Other notes
- No sign-in or account is needed. The add-in needs internet access to nixpx.com.
- Diagram data is stored in the document (Office.context.document.settings), and the picture carries
  its id (alt text in Word, a shape tag in PowerPoint, the shape name in Excel).
- Support: support@nixpx.com
```

## Before submitting

- [ ] `npx office-addin-manifest validate manifest.xml` passes (run it on a machine with internet).
- [ ] A test licence key for reviewers, with enough activations (reviewers use several machines).
- [ ] Screenshots from clean sample files.
- [ ] https://nixpx.com/naqsha/privacy, /terms and /support load.
