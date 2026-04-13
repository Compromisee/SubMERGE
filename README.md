

# SubMerge

A sleek, minimalist desktop application for fetching and embedding subtitles into MKV files. Built with Rust and egui.

#READ CODE.MD for all files
---

## Features

| Feature | Description |
|---|---|
| **Auto-Detection** | Parses season/episode from filenames (`S01E02`, `1x02`, `Season 1 Episode 2`) |
| **Free API** | Uses [Subdl.com](https://subdl.com) — no API key required |
| **Dry Run Mode** | Preview every action before touching any files |
| **Real Mode** | Downloads subtitles and merges them into the MKV via `mkvmerge` |
| **Drag & Drop** | Drop an MKV file directly onto the window |
| **Multi-Language** | English, Spanish, French, German subtitle support |
| **Dark Theme** | Deep dark palette with animated gradient accents |
| **Custom Icons** | Hand-drawn vector icons — zero emojis |
| **JetBrains Mono** | Monospace typography throughout the entire UI |
| **Animated UI** | Gradient borders, pulsing indicators, bouncing icons, glowing buttons |

---

## Screenshots

The interface features:
- Animated gradient title text (indigo → purple → pink)
- Rotating gradient outline on the mode toggle and input fields
- Dashed animated drop zone border
- Color-coded season/episode badges
- Pulsing status indicator dot
- Glowing action buttons during processing
- Scrollable log panel with typed entries

---

## Prerequisites

### 1. Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. MKVToolNix

`mkvmerge` is required for the actual subtitle merge step.

**Linux (Debian/Ubuntu)**
```bash
sudo apt update && sudo apt install mkvtoolnix
```

**Linux (Fedora)**
```bash
sudo dnf install mkvtoolnix
```

**Linux (Arch)**
```bash
sudo pacman -S mkvtoolnix-cli
```

**macOS**
```bash
brew install mkvtoolnix
```

**Windows**

Download the installer from [mkvtoolnix.download](https://mkvtoolnix.download/downloads.html#windows) and add it to your PATH, or install to the default location (the app checks common paths automatically).

### 3. System Dependencies (Linux only)

egui requires a few system libraries for the windowing backend:

```bash
# Debian/Ubuntu
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libgtk-3-dev

# Fedora
sudo dnf install libxcb-devel libxkbcommon-devel openssl-devel gtk3-devel
```

### 4. JetBrains Mono Font

Download the font and place the regular weight file into the project:

```bash
mkdir -p assets
curl -L -o /tmp/jetbrains-mono.zip \
  "https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip"
unzip -j /tmp/jetbrains-mono.zip "fonts/ttf/JetBrainsMono-Regular.ttf" -d assets/
```

Your `assets/` directory should contain:
```
assets/
└── JetBrainsMono-Regular.ttf
```

---

## Project Structure

```
submerge/
├── Cargo.toml
├── README.md
├── assets/
│   └── JetBrainsMono-Regular.ttf
└── src/
    ├── main.rs              # Entry point, window setup
    ├── app.rs               # Application state, async task coordination
    ├── parser.rs            # Filename → season/episode extraction
    ├── subtitle_api.rs      # Subdl.com API client, download + extract
    ├── mkv_merge.rs         # mkvmerge wrapper
    ├── utils.rs             # Temp dirs, filename sanitization
    └── ui/
        ├── mod.rs           # Main render function, all panels
        ├── theme.rs         # Colors, fonts, style configuration
        ├── icons.rs         # Hand-drawn vector icon renderer
        ├── components.rs    # Gradient borders, dashed rects
        └── animations.rs    # Time-based animation helpers
```

---

## Build & Run

### Development

```bash
cargo run
```

### Release (optimized + LTO)

```bash
cargo build --release
./target/release/submerge
```

---

## Usage

### Step-by-step

1. **Launch** the application
2. **Toggle mode** — click the `DRY RUN / REAL` toggle at the top
   - *Dry Run*: logs what would happen, changes nothing on disk
   - *Real*: downloads the subtitle file and merges it into the MKV
3. **Select a file** — drag an `.mkv` file onto the drop zone, or click it to open a file picker
4. **Verify detection** — the app parses the filename and displays season/episode badges
5. **Edit the show name** if the auto-detected name is incorrect
6. **Pick a language** — `EN`, `ES`, `FR`, or `DE`
7. **Click Search** — the app queries the Subdl API for matching subtitles
8. **Select a subtitle** from the results list
9. **Click Merge** (or Dry Run) — the subtitle is downloaded, extracted, and merged into the MKV
10. **Check the log panel** at the bottom for detailed output

### Filename Patterns Recognized

| Pattern | Example |
|---|---|
| `S01E02` | `Breaking.Bad.S01E02.720p.mkv` |
| `s01e02` | `breaking bad s01e02.mkv` |
| `1x02` | `The Office 1x02.mkv` |
| `Season 1 Episode 2` | `Show Season 1 Episode 2.mkv` |

The parser also strips common tags (`720p`, `1080p`, `HDTV`, `WEB-DL`, `x264`, etc.) from the detected show name.

### Output

In **Real** mode, the merged file is saved alongside the original:

```
Before:
  Breaking.Bad.S01E02.720p.mkv

After:
  Breaking.Bad.S01E02.720p.mkv          (original, untouched)
  Breaking.Bad.S01E02.720p_subbed.mkv   (new file with embedded subtitle)
```

---

## Configuration

### API

The app uses [Subdl.com](https://subdl.com) as its subtitle source. This API is free and requires no authentication for basic search queries. If the API is unreachable, the app falls back to mock results for testing purposes.

### mkvmerge Detection

The app searches for `mkvmerge` in the following locations:

| OS | Paths checked |
|---|---|
| All | `mkvmerge` (on PATH) |
| Linux | `/usr/bin/mkvmerge`, `/usr/local/bin/mkvmerge` |
| Windows | `C:\Program Files\MKVToolNix\mkvmerge.exe`, `C:\Program Files (x86)\MKVToolNix\mkvmerge.exe` |

---

## Design

### Color Palette

| Name | Hex | Usage |
|---|---|---|
| BG Primary | `#0D0D12` | Window background |
| BG Secondary | `#16161E` | Cards, panels |
| BG Tertiary | `#20202A` | Inputs, hover states |
| Text Primary | `#F0F0F5` | Headings, main text |
| Text Secondary | `#A0A0AF` | Labels, descriptions |
| Text Dim | `#5A5A69` | Placeholders, disabled |
| Accent Blue | `#4285F4` | Search, links, active |
| Accent Green | `#34C759` | Success, real mode |
| Accent Red | `#FF453A` | Errors |
| Accent Yellow | `#FFCC00` | Warnings, dry run |
| Accent Purple | `#AF52DE` | Season badges |
| Accent Cyan | `#32D7DB` | Episode badges |

### Gradient

The title text and border animations cycle through:

```
Indigo (#6366F1) → Purple (#A855F7) → Pink (#EC4899)
```

### Typography

JetBrains Mono is used at the following sizes:

| Element | Size |
|---|---|
| Title | 36px |
| Buttons | 14px |
| Body text | 14px |
| Labels | 12px |
| Log entries | 11px |
| Badges | 10–16px |

---

## Troubleshooting

### "mkvmerge not found"

Install MKVToolNix (see [Prerequisites](#2-mkvtoolnix)) and ensure `mkvmerge` is on your system PATH.

```bash
# Verify installation
mkvmerge --version
```

### "No subtitles found"

- Double-check the show name spelling
- Try a different language
- Ensure the season/episode numbers are correct
- The Subdl API may not have subtitles for every show

### Window doesn't open (Linux)

Install the required X11/Wayland development libraries:

```bash
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
```

### Font not loading

Ensure `assets/JetBrainsMono-Regular.ttf` exists relative to `Cargo.toml`. The font is embedded at compile time via `include_bytes!`.

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `eframe` | 0.29 | Native window + egui integration |
| `egui` | 0.29 | Immediate-mode GUI framework |
| `reqwest` | 0.12 | HTTP client (blocking) for API calls |
| `serde` | 1.0 | JSON deserialization |
| `regex` | 1 | Filename pattern matching |
| `rfd` | 0.15 | Native file dialog |
| `zip` | 2.0 | Extracting subtitles from ZIP archives |
| `flate2` | 1.0 | Extracting subtitles from GZIP archives |
| `dirs` | 5.0 | Platform-appropriate cache directories |

---

## License

MIT
```
