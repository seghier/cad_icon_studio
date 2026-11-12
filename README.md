# CAD Icon Studio Pro

> **High-Performance Vector Pathfinder, Shape Builder & Multi-Size Studio for Technical & CAD Icons**

**CAD Icon Studio Pro** is a standalone, ultra-responsive desktop vector graphic design application crafted in **Rust**, powered by **egui / eframe** and **tiny-skia**. Built specifically for CAD software developers, UI/UX designers, and technical illustrators who need pixel-crisp, geometric vector icons and multi-resolution asset generation.

---

## ✨ Features

- **📐 Parametric CAD Vector Primitives**:
  - **NURBS & Smooth Curves**: Interpolated Catmull-Rom splines converted dynamically into cubic Bézier paths.
  - **3-Point Arcs & Circles**: Precise geometric arc definitions with custom start, sweep angles, and pie closure options.
  - **Polylines & Regular Polygons**: Interactive point-by-point drawing with automatic closure and corner radius control.
  - **Quadratic & Cubic Béziers**: Clean vector curves with intuitive control handles.

- **⚡ 2D Boolean Pathfinder Engine**:
  - Full support for constructive solid geometry operations on 2D vector shapes:
    - **Union** (Merge shapes into single compound outline)
    - **Difference** (Subtract partner shape)
    - **Intersection** (Retain overlapping regions)
    - **Exclusion (XOR)** (Symmetric difference)
    - **Divide / Fragment** (Slice geometry along overlapping boundaries)

- **🎨 Shape Builder & Curve Edge Painter**:
  - Point-in-polygon flood fill for sub-regions.
  - Individual edge stroke coloring and style customization (custom dash arrays, caps, and joins).

- **🔄 Interactive Precision Transform Tools**:
  - Full 2D transform handles: Move, corner-point repositioning, pivot-based rotation with angle snapping, and uniform/non-uniform bounding box scaling.
  - Z-order management (Bring to Front, Send to Back, Move Up/Down) and shape duplication.

- **🌗 Dual-Theme Side-by-Side Workspace**:
  - Live side-by-side Dark and Light theme viewports to ensure icons maintain readability and contrast across all UI environments.

- **🔍 Multi-Resolution Real-Time Preview Strip**:
  - Instantaneous rasterization previews at industry-standard toolbar icon dimensions (**64px**, **48px**, **32px**, and **24px**).
  - Background options: Dark, Light, or Transparent Checkerboard.

- **💾 Clean SVG & PNG Export**:
  - Export production-ready clean SVG vector graphics with customizable stroke and fill rules (Winding / EvenOdd).
  - High-res anti-aliased PNG export rendered by `tiny-skia`.
  - JSON project save/load format (`.json`) for full vector hierarchy preservation.

---

## 🛠️ Architecture & Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/) (2021 Edition)
- **GUI Framework**: [egui / eframe](https://github.com/emilk/egui) (v0.29) — Immediate-mode GPU-accelerated interface
- **Rasterization**: [tiny-skia](https://github.com/RazrFalcon/tiny-skia) (v0.11) — Software 2D rendering pipeline with anti-aliasing
- **Serialization**: [serde](https://serde.rs/) & [serde_json](https://github.com/serde-rs/json)
- **Dialogs**: [rfd](https://github.com/PolyMeilex/rfd) — Native file open/save dialogs

---

## 🚀 Getting Started

### Pre-built Binaries (Windows)

Download the latest standalone executable from the [Releases](https://github.com/seghier/cad_icon_studio/releases) tab. No installation required—just unzip and run `cad_icon_studio.exe`.

### Building from Source

1. **Install Rust** (if not already installed):
   Visit [rustup.rs](https://rustup.rs/) to install the Rust toolchain.

2. **Clone the Repository**:
   ```bash
   git clone https://github.com/seghier/cad_icon_studio.git
   cd cad_icon_studio
   ```

3. **Run in Development**:
   ```bash
   cargo run
   ```

4. **Build Release Binary**:
   ```bash
   cargo build --release
   ```
   The optimized executable will be located at:
   ```text
   target/release/cad_icon_studio.exe
   ```

---

## ⌨️ Shortcut Cheat Sheet

| Key | Tool / Action |
|:---|:---|
| **V** | Move / Transform Tool |
| **A** | Node / Corner Point Edit Tool |
| **N** | NURBS Curve Tool |
| **M** | Shape Builder Fill Tool |
| **I** | Curve Edge Painter Tool |
| **Delete** | Delete Selected Shape |
| **Ctrl + S** | Save Project (.json) |
| **Ctrl + O** | Open Project (.json) |
| **Ctrl + D** | Duplicate Selected Shape |

---

## 📄 License

This project is open-source. See the repository for details.
