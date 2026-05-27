# 🚀 wingrip

<p align="center">
  <strong>Advanced Hybrid Window Manager & Dynamic Tiling Zones for Windows 10/11</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-1C1822?style=flat-square&logo=windows&logoColor=B496DC&labelColor=1C1822" alt="Platform: Windows 10/11">
  <img src="https://img.shields.io/badge/Language-Rust-ea4e31?style=flat-square&logo=rust&logoColor=ea4e31&labelColor=1C1822" alt="Language: Rust">
  <img src="https://img.shields.io/badge/UI-Fluent%20Win32-B496DC?style=flat-square&logo=windows&logoColor=1C1822&labelColor=1C1822" alt="UI: Fluent Win32">
</p>

---

**wingrip** is a lightweight, zero-overhead background utility that brings Linux X11/KDE-style window manipulation, dynamic visual tiling layouts, and shared-border joint resizing to Windows 10 & 11 using native Win32 GDI calls and safe, high-performance Rust.

---

## 🌟 Key Features

*   **⚡ Zero-Latency Window Manipulation:** Grab, drag, and resize any window instantly by clicking anywhere inside it while holding key modifiers.
*   **📐 3x3 Intelligent Resizing Grid:** Windows are virtually partitioned into a 3x3 matrix. Dragging from any edge or corner resizes the window relative to that quadrant, while middle dragging scales the window symmetrically from the center.
*   **🛸 Visual Snapping Overlay Grid:** Hold `Win + Shift + Left Click` to drag windows into layout zones (halves, thirds, or quadrants) with a modern semi-transparent overlay guide that supports click-through transparency.
*   **🔗 Shared-Border Joint Resizing:** When windows are snapped adjacent to each other in tiling zones, their borders are logically coupled. Resizing one snapped window synchronously shrinks or grows adjacent snapped windows, preserving a clean tiled layout.

---

## ⌨️ Mouse & Keyboard Gestures

| Trigger | Mouse Interaction | Action |
| :--- | :--- | :--- |
| **`Win + Left Click`** | Mouse Drag | Drag window anywhere instantly |
| **`Win + Left Click`** | Double-Click | Toggle between Maximize and Restore state |
| **`Win + Right Click`** | Mouse Drag | Resize window dynamically based on 3x3 region |
| **`Win + Right Click`** | Double-Click | Minimize target window to the system taskbar |
| **`Win + Shift + Left Click`** | Mouse Drag | Drag window into visual snapping zones (with overlay) |

---

## ⚙️ Configuration & Theming

All persistent parameters are stored in a local `config.toml` file at the root.

```toml
[settings]
deadzone_pixels = 6                  # Pixels to travel before triggering dragging
snapping_threshold_pixels = 30       # Snapping trigger zone boundary
layouts_enabled = true               # Toggle visual grid layout overlays
gestures_enabled = true              # Enable double-click Maximize/Minimize gestures

[blacklist]
processes = []

[ui]
preview_fill_color = 369865660       # Color of translucent overlay grids (HEX to Decimal)
preview_border_color = 14456500      # Border accent of overlay grids
preview_opacity = 140                # Alpha transparency (10-255)
preview_border_radius = 12           # Corner roundedness of snap overlays
gap_pixels = 8                       # Space between windows
```

---

## 🛠️ Build & Installation

### Prerequisites

*   **Rust Toolchain:** Standard Rust (cargo) installed via [rustup](https://rustup.rs/).
*   **Operating System:** Windows 10 or 11.
*   **UAC Privileges:** Must run as Administrator to manipulate windows owned by elevated applications (such as Task Manager or Terminal).

### Compilation

Clone the repository and build the optimized production release binary:

```powershell
# Build standard optimized release binary
cargo build --release
```

The compiled binary will be placed at `target\release\wingrip.exe`.

### Execution

To run `wingrip` silently in the background, you can start it from your terminal:

```powershell
# Start wingrip silently in the background
Start-Process -FilePath "target\release\wingrip.exe"
```

To configure wingrip, right-click the system tray icon in the taskbar and select **Settings** to open the Fluent settings panel.

---