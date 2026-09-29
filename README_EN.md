<div align="center">

# 🌊 Watermark Tool

**Batch Image Watermark Tool** — a desktop GUI app built with Rust + egui. It supports custom watermark placement, tiled layouts, opacity control, and batch export, with a built-in Chinese / English bilingual interface.

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-000?logo=rust&logoColor=fff)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue)](#)

> 📖 **简体中文版 / Chinese version**：[README.md](./README.md)

</div>

---

## 🖼️ Preview
![Project screenshot](https://raw.githubusercontent.com/cheng01315/batch-image-watermark/main/others/image.jpg)

## ✨ Features

### 🎯 Watermark Layout Engine
- **Single watermark mode**: 9 preset anchors (top-left / top-center / top-right / middle-left / center / middle-right / bottom-left / bottom-center / bottom-right), with margin and X / Y fine-tuning
- **Tiled watermark mode**: 3 tiling patterns
  - Grid
  - Brick (staggered)
  - Diagonal (seamless)
- **Adjustable tile spacing**: independent X / Y spacing percentage
- **Free rotation**: watermark rotation from ±180°

### 🎨 Watermark Style Control
- **Opacity**: 0% ~ 100% smooth adjustment, correct Alpha blending
- **Scaling**: scale relative to the source's short edge, or force an absolute pixel width
- **Resampling**: high-quality Lanczos3 scaling to avoid blur and aliasing

### 📦 Batch Export Engine
- **Multi-threaded**: powered by the standard library `std::thread` + `mpsc` channel; export runs on a background thread so the UI stays responsive
- **Live progress bar**: shows current / total / percentage progress
- **Name conflict strategy**:
  - Auto rename (append `_wm` suffix; add an index when still conflicting)
  - Overwrite existing file
  - Skip existing file
- **Format conversion**: keep source format / unify to JPEG / unify to PNG
- **JPEG quality**: configurable from 1 to 100

### 🌐 Bilingual Interface
- **Auto-detect system language**: launches in Chinese on Chinese-locale systems (Windows UI language = Chinese); defaults to English otherwise
- **Manual switch**: toggle between 「中文 / English」 at the top-right of the control panel; the choice is persisted
- **Env override (optional)**: set `WATERMARK_TOOL_LANG=zh` or `=en` to force a language

### 💾 Settings Persistence
- Config is auto-saved to the OS user config directory (Windows: an app-specific subfolder under `%APPDATA%`; the file `config.json` is located automatically via the `directories` crate)
- All parameters and the language choice are restored on next launch
- Supports PNG / JPEG / BMP / WebP input formats

### 🖼️ Preview & Interaction
- Classic layout: left control panel + right preview canvas; control-panel sections are expanded by default
- **Real-time preview** after editing parameters (auto-scaled to fit the window)
- Native file / folder picker dialogs (`rfd`)
- Toast-style notifications (success / warning / error)
- Export report dialog on completion (success / failed / skipped counts and file list)

---

## 🚀 Quick Start

### Option 1: Download & Run (recommended)
Just download `watermark-tool.exe` from the repo root and double-click to run — no runtime libraries required.

### Option 2: Build from Source
See the 🛠️ Build Guide below.

---

## 🎮 Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  ① Pick a watermark image ─► PNG/JPG/BMP/WebP as the logo    │
│  ② Select source images ──► multi-select, add more, or clear │
│  ③ Adjust watermark params ► position / tile / opacity / scale / rotate │
│  ④ Preview ──────────────► live composite preview on the right │
│  ⑤ Choose output folder ─► format, quality, naming strategy  │
│  ⑥ Start batch export ──► live progress bar + completion report │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛠️ Build Guide

### Prerequisites
- Rust toolchain 1.75+ (install via [rustup](https://rustup.rs/))

### Debug build (development)
```powershell
cd watermark-tool
cargo build
.\target\debug\watermark-tool.exe
```

### Release build (optimized)
```powershell
cargo build --release
.\target\release\watermark-tool.exe
```

Release profile optimizations (in `Cargo.toml`):
| Option | Value | Effect |
|--------|-------|--------|
| `opt-level` | 3 | highest optimization level |
| `lto` | fat | whole-program link-time optimization, smaller binary |
| `codegen-units` | 1 | single codegen unit, maximizes LTO |
| `strip` | true | strip debug symbols and symbol table |
| `panic` | abort | drop panic unwinding code, smaller & faster |

The final single-file EXE is about **5 ~ 8 MB** with no external DLL dependencies.

---

## 🧱 Tech Stack

| Category | Crate | Version | Purpose |
|----------|-------|---------|---------|
| GUI framework | eframe + egui | 0.28 | immediate-mode cross-platform desktop UI |
| Image codec | image | 0.25 | PNG/JPEG/BMP/WebP read & write |
| File dialog | rfd | 0.14 | native system file / folder picker |
| Serialization | serde + serde_json | 1.x | settings persistence |
| User dir | directories | 6.x | cross-platform user config directory |
| Error handling | anyhow | 1.x | ergonomic errors (`Context` / `Result`) |
| i18n | self-implemented (i18n.rs) | — | ZH / EN strings & system-language detection (no 3rd-party dep) |

---

## 📁 Project Structure

```
watermark-tool/
├── Cargo.toml            # project config & dependencies
├── Cargo.lock            # locked dependency versions
├── README.md             # Simplified Chinese version
├── README_EN.md          # this document (English)
├── watermark-tool.exe    # prebuilt Windows executable
├── others/
│   └── image.jpg         # README screenshot
└── src/
    ├── main.rs           # main program (~1800 lines): data / compositing core / export engine / UI
    └── i18n.rs           # i18n module: ZH / EN strings + system-language detection
```

### Inside src/main.rs (top to bottom)
1. **Data structures**: anchors, layout modes, tile patterns, name-conflict, export format, params struct
2. **Settings persistence**: `load_config()` / `save_config()` (writes `config.json`)
3. **Watermark compositing core**:
   - `resize_watermark()` — watermark scaling (Lanczos3)
   - `apply_opacity()` — opacity blend
   - `rotate_image()` — rotation
   - `alpha_composite()` — alpha-channel overlay
   - `compute_single_position()` — single-image position
   - `compose_watermark()` — single / tiled entry point
   - `determine_output_format()` / `resolve_conflict_path()` — format & naming
4. **Batch export engine**: `process_single_image()` + `start_export()` (background `thread::spawn` + `mpsc` progress reporting)
5. **UI layer**: `WatermarkApp` and the `eframe::App` impl (left panel / preview canvas / dialogs / toasts / language switch)
6. **Entry point**: `eframe::run_native` launches the native window

---

## 📌 Supported Image Formats

| Format | Read | Export | Notes |
|--------|------|--------|-------|
| PNG | ✅ | ✅ | transparent channel supported |
| JPEG | ✅ | ✅ | adjustable compression quality |
| BMP | ✅ | ❌ (export falls back to JPEG/PNG) |  |
| WebP | ✅ | ❌ (export falls back to JPEG/PNG) |  |

---

## ⚠️ Notes

1. **A PNG with a transparent background is recommended** for the watermark — best compositing result
2. Too-small tile spacing causes overlapping watermarks (visible in the preview)
3. Batch processing many large images uses significant memory — process in batches if needed
4. The first Release build is slow (whole-program LTO) — please be patient

---

## 📄 License

Released under the license in the [LICENSE](./LICENSE) file.
