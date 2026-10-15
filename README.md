<div align="center">

<img src="assets/icon-512.png" width="128" height="128" alt="Naqsha app icon: an eight-pointed star drawn as a connector between nodes">

# Naqsha

**Native diagramming and architecture for macOS: flowcharts, cloud and solution architecture, CI/CD pipelines, Visio and draw.io files.**

*naqsha* (نقشه): a plan, a map, a blueprint.

[Download for macOS](../../releases/latest) · [Build from source](#build-from-source) · [Features](#features) · [File formats](#open-and-save-any-format) · [Roadmap](#roadmap)

![macOS 11+](https://img.shields.io/badge/macOS-11%2B-0F0F1A?logo=apple&logoColor=white)
![Apple Silicon + Intel](https://img.shields.io/badge/universal-Apple%20Silicon%20%2B%20Intel-6C4DFF)
![Written in Rust](https://img.shields.io/badge/written%20in-Rust-19C3DD?logo=rust&logoColor=white)
![Opens .vsdx and .drawio](https://img.shields.io/badge/opens-.vsdx%20·%20.drawio%20·%20.svg-6C4DFF)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/hero-dark.png">
  <img src="docs/images/hero-light.png" alt="Naqsha showing a multi-page solution architecture pack: an overview page whose sections are live previews of the context, container, deployment, key-flow and decision pages" width="100%">
</picture>

</div>

---

## Why Naqsha

- **A real Mac app.** It's GPU-rendered and starts instantly. It has a native menu bar and opens files from Finder, and it works offline. Your diagrams are plain files on your disk, not rows in someone else's cloud.
- **It speaks Visio and draw.io.** Open a `.vsdx` from a colleague, edit it, and send back a `.vsdx`. The same goes for `.drawio`, with pages, layers, containers, shape data and connector routes kept.
- **Built for architects and DevOps engineers.** It comes with C4, cloud and enterprise-integration stencils, and with templates for solution architecture packs, GitOps and environment promotion. It also imports GitHub Actions, GitLab CI, Azure Pipelines, Bitbucket Pipelines and Jenkins definitions straight into diagrams.
- **Large systems stay readable.** Any shape can open its own page (a sub-diagram) and shows a live preview of it, so an overview stays clean while the detail is one click away.

## Features

<table>
<tr>
<td width="50%" valign="top">

**Drawing**
- 180 shapes in 10 searchable stencil groups: basic, flowchart, network, UML and data, containers and swimlanes, cloud infrastructure, cloud zones, enterprise integration, CI/CD and DevOps, architecture artifacts (C4, ADRs, risks, requirements)
- Your own SVG icon folders as stencils, e.g. a cloud vendor's official icon pack
- Right-angle, straight and curved connectors with glue, AutoConnect, line jumps and routing around shapes
- Drag any connector segment to reroute it by hand
- Auto Arrange: layered top-down or left-to-right, grid, containers and swimlanes

</td>
<td width="50%" valign="top">

**Organising**
- Multiple pages, layers, groups, containers and swimlanes
- Sub-diagrams with live previews, breadcrumbs and Back (⌘↑)
- Shape data (key/value fields) on any shape or connector
- Themes, quick styles, Format Painter, light and dark mode. Text stays readable on any fill.
- Multiple windows, one per diagram
- Crash recovery and an unsaved-changes prompt

</td>
</tr>
</table>

### Start from a template

<img src="docs/images/gallery.png" alt="The New Diagram gallery with categories and live template previews" width="100%">

18 built-in templates. You can also save your own diagrams as templates (File › Save as Template).

| Category | Templates |
|---|---|
| Solution architecture | Solution Architecture Pack (overview with 5 linked pages) · C4 System Context · C4 Containers · Decisions, Risks & Requirements · Enterprise Integration |
| Cloud & infrastructure | Three-Tier Web App · Microservices & Events · Data Platform · Network & Security Zones |
| CI/CD & DevOps | CI/CD Pipeline · GitOps Delivery · Environment Promotion · Infrastructure as Code |
| Software design | Sequence Diagram · Data Model (ER) |
| Flowchart | Basic Flowchart · Cross-Functional (swimlanes) · Blank |

### Turn a pipeline file into a diagram

<img src="docs/images/pipeline.png" alt="A GitHub Actions workflow imported as a diagram: trigger, then jobs in stage columns, with environments labelled" width="100%">

Open or drop a CI/CD definition and Naqsha draws it:
- The trigger comes first, then jobs in stage columns, ordered by `needs`, `dependsOn` or `stage`.
- Matrix builds, environments and manual approvals are labelled on the jobs.
- Each job opens a page of its steps, with `run:` scripts shown as code.

| Supported definitions |
|---|
| GitHub Actions (`.github/workflows/*.yml`) · GitLab CI (`.gitlab-ci.yml`) · Azure Pipelines (`azure-pipelines.yml`) · Bitbucket Pipelines (`bitbucket-pipelines.yml`) · Jenkins (`Jenkinsfile`, declarative) |

## Open and save any format

| Format | Open | Save | What's kept |
|---|:---:|:---:|---|
| Naqsha `.nqs` | ✓ | ✓ | Everything. Plain JSON, so it diffs well in git. Older `.schx` files open too. |
| Microsoft Visio `.vsdx` | ✓ | ✓ | Masters, styles, groups, rotation and flip, text blocks, layers, shape data, glued connectors, pictures, page links |
| draw.io / diagrams.net `.drawio` | ✓ | ✓ | Pages, layers, containers, HTML labels, label positions, data, waypoints, page links. Both compressed and plain files. |
| SVG | ✓ | ✓ | Paths become editable shapes, text becomes text, embedded images become pictures |
| PNG · PDF | — | ✓ | Vector PDF. Sub-diagram previews are included. |

Legacy binary Visio files (`.vsd`) aren't supported; re-save them as `.vsdx` in Visio.

## Install

### Download
Get the latest `.dmg` from **[Releases](../../releases/latest)**, open it and drag **Naqsha** to Applications. It's one universal app for Apple Silicon and Intel Macs, running macOS 11 or later.

### Build from source
```bash
# one-time: install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# build Naqsha.app and a .dmg (universal), and install it into /Applications
./build-mac.sh --install          # add --native for a faster build for your Mac only
```
The build is ad-hoc signed, so it runs on the machine that built it. To distribute it, set `SIGN_IDENTITY="Developer ID Application: …"` and notarize. Pushing a `v*` tag runs the GitHub Actions workflow in `.github/workflows/macos.yml`, which builds the DMG on a macOS runner.

## Keyboard shortcuts

| Action | Keys | Action | Keys |
|---|---|---|---|
| New (template gallery) | ⌘N | Auto Arrange | ⇧⌘L |
| Open / Save | ⌘O / ⌘S | Light / dark | ⇧⌘D |
| Pointer · Connector · Text · Pan | V · C · T · H | Open sub-diagram / Back | ⌘↓ / ⌘↑ |
| Duplicate | ⌘D or ⌥-drag | Group / Ungroup | ⌘G / ⇧⌘G |
| Edit text | Double-click, or just type | Front / Back | ⇧⌘] / ⇧⌘[ |
| Fit page / 100% | ⌘1 / ⌘0 | Lock | ⌘L |
| Cancel a drag, close a menu | Esc | All shortcuts | Help menu |

## Command line

```bash
naqsha --export in.vsdx out.drawio          # convert between .nqs, .vsdx, .drawio
naqsha --export in.nqs out.pdf 2 --dark      # page 2 as PDF / PNG / SVG, optionally dark
naqsha --export .github/workflows/ci.yml ci.png
naqsha --arrange lr in.drawio out.nqs        # auto-arrange: tb | lr | grid
naqsha --templates ./out                     # write every built-in template as .nqs
```

## Roadmap

- **Naqsha on the web:** the same editor in the browser at nixpx.com/naqsha, like draw.io, with no install
- Comments and review mode
- Visio stencil files (`.vssx`) as shape libraries
- Data-linked shapes (CSV / JSON)

Ideas and bug reports are welcome in [Issues](../../issues).

## Contributing

```bash
cargo test --release      # 29 tests: formats, routing, layout, sub-diagrams, pipelines, templates
cargo run --release       # run the app
```

<details>
<summary>Project layout</summary>

```
src/main.rs        startup, CLI, per-window frame
src/shell.rs       multiple windows
src/app.rs         application state, documents, undo, files
src/model.rs       document model (pages, shapes, connectors), serde JSON
src/canvas.rs      canvas interaction: select, move, resize, connect, edit routes
src/render.rs      drawing shapes and connectors; readable text colours
src/geom.rs        transforms, connection points, connector routing
src/route.rs       obstacle-avoiding routing, route clean-up, line jumps
src/layout.rs      Auto Arrange
src/subpage.rs     sub-diagrams and navigation
src/templates.rs   built-in templates
src/library.rs     stencils and shape geometry
src/userlib.rs     SVG stencil folders
src/ui.rs          ribbon, panels, format pane, dialogs, shortcuts
src/export.rs      SVG / PNG / PDF export
src/formats/       vsdx.rs · drawio.rs · svgin.rs · pipeline.rs
src/macos.rs       native menu bar, Finder integration
assets/            icon.svg (source), Naqsha.icns
examples/          sample.nqs, make_icon.rs, render_svg.rs
```
</details>

## Formerly Schemix

Naqsha was called Schemix during early development. Older `.schx` files open as normal. Your settings, recent files and saved templates move over automatically the first time Naqsha starts.

## License

© 2026 Nixpx LLC. License to be announced.

<div align="center"><sub>Made by <a href="https://nixpx.com">Nixpx</a></sub></div>
