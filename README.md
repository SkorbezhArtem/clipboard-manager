# 📋 Clipboard Manager

> A fast, lightweight clipboard history manager for Windows — built with Tauri (Rust + TypeScript).

Tracks everything you copy, lets you search and re-paste in seconds, with full encryption support, templates, syntax highlighting, and more.

---

## ✨ Features

### Core
- **📋 Clipboard History** — Automatically saves every copied text and image. Up to 10 000 entries (configurable).
- **🔍 Instant Search** — Full-text search across all history with live match highlighting and recent search history dropdown.
- **📌 Pin Items** — Pin frequently used snippets to the top so they never scroll away.
- **🖼️ Image Support** — Full capture and preview of clipboard images (PNG / JPEG).
- **🔢 Quick Copy 1–9** — Press a number key to instantly copy the N-th visible item without touching the mouse.

### Organisation
- **⭐ Templates** — Star any item to save it as a permanent template. Templates survive auto-cleanup and are always accessible via the Templates tab.
- **🏷️ Tags** — Attach comma-separated tags to items for easy categorisation. Searchable.
- **🗂️ Filter Tabs** — Switch between All / Text / Images / Pinned / Templates in one click.

### Intelligence
- **🎨 Syntax Highlighting** — Code snippets are auto-detected and highlighted with language-aware colouring (JS, TS, Python, Rust, SQL, CSS, HTML, Bash, C++).
- **🕐 Search History** — Recent searches are saved locally and shown as a dropdown when the search bar is focused.

### Security
- **🔒 AES-256-GCM Encryption** — Protect all stored content with a master password. Key derived via Argon2id.
- **🔑 Password Management** — Set, change, lock and unlock encryption at any time from Settings.

### Data
- **💾 Export / Import** — Back up and restore your full clipboard history as JSON. Export goes directly to your Downloads folder.
- **🗑️ Auto-cleanup** — Automatically remove old items on startup:
  - Global cleanup (all types older than N days)
  - Per-type: text older than N days, images older than N days
  - Pinned items and templates are never deleted.

### UI / UX
- **🎨 Dark & Light Theme** — Fully themed UI, applied consistently across all windows.
- **🖥️ System Tray** — Runs silently in the tray. Left-click to toggle, right-click for menu.
- **⌨️ Global Hotkey** — Open from anywhere with `Ctrl+Shift+V` (customisable in Settings).
- **🔔 Toast Notifications** — Non-blocking, styled feedback for every action.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+Shift+V` | Open / close the manager (global) |
| `↑` / `↓` | Navigate items |
| `Enter` | Copy selected item & close |
| `1` – `9` | Quick-copy the N-th visible item |
| `T` | Toggle template on selected item |
| `P` | Pin / unpin selected item |
| `D` | Delete selected item |
| `Esc` | Close window |

---

## 🛠️ Tech Stack

| Layer | Technology |
|---|---|
| 🖥️ Desktop framework | [Tauri 2](https://tauri.app/) |
| 🦀 Backend language | [Rust](https://www.rust-lang.org/) (stable) |
| 🗄️ Database | SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) |
| 📋 Clipboard access | [arboard](https://github.com/1Password/arboard) |
| 🔒 Encryption | AES-256-GCM · Argon2id ([aes-gcm](https://crates.io/crates/aes-gcm), [argon2](https://crates.io/crates/argon2)) |
| 🟦 Frontend | TypeScript |
| ⚡ Bundler | [Vite 5](https://vitejs.dev/) |
| 🎨 Syntax highlighting | [highlight.js](https://highlightjs.org/) (tree-shaken) |
| ⌨️ Hotkeys | [@tauri-apps/plugin-global-shortcut](https://tauri.app/) |

---

## 🚀 Getting Started

### Prerequisites

- **Node.js** 18 or newer — [nodejs.org](https://nodejs.org/)
- **Rust** (stable toolchain) — [rustup.rs](https://rustup.rs/)
- **Windows 10 / 11** (x64)

### Development

```bash
# 1. Clone the repo
git clone https://github.com/your-username/clipboard-manager.git
cd clipboard-manager

# 2. Install JS dependencies
npm install

# 3. Start dev server with hot-reload
npm run tauri dev
```

The app window opens automatically. The Vite dev server handles frontend hot-reload; Rust recompiles on backend changes.

### Production Build

```bash
npm run tauri build
```

Outputs:
- **Installer:** `src-tauri/target/release/bundle/msi/*.msi`
- **Portable EXE:** `src-tauri/target/release/clipboard-manager.exe`

---

## 📁 Project Structure

```
clipboard-manager/
├── src/                        # Frontend
│   ├── index.html              # Main window
│   ├── settings.html           # Settings window
│   ├── main.ts                 # Main app logic
│   ├── settings.ts             # Settings page logic
│   ├── toast.ts                # Toast notification system
│   ├── highlight.ts            # Syntax highlighting (hljs wrapper)
│   ├── searchHistory.ts        # Recent searches (localStorage)
│   ├── style.css               # Global styles & themes
│   └── types.ts                # Shared TypeScript interfaces
│
└── src-tauri/                  # Rust backend
    ├── Cargo.toml
    └── src/
        ├── main.rs             # Tauri setup & all command handlers
        ├── db.rs               # SQLite schema, queries, migrations
        ├── clipboard.rs        # Background clipboard watcher
        ├── encryption.rs       # AES-256-GCM + Argon2 encryption
        └── settings.rs         # Settings persistence
```

---

## ⚙️ Configuration

All settings are saved in the SQLite database and editable via the **Settings** page (`⚙️` button or tray menu).

| Setting | Default | Description |
|---|---|---|
| History limit | 1 000 | Max items to keep |
| Global auto-cleanup | Off / 30 days | Delete all old items |
| Text cleanup | 0 (off) | Delete old text-only items |
| Image cleanup | 7 days | Delete old image items |
| Theme | Dark | `dark` or `light` |
| Global hotkey | `Ctrl+Shift+V` | Customisable key combination |

---

## 🔒 Encryption Details

When encryption is enabled:
- All content is encrypted with **AES-256-GCM** before being written to disk.
- The encryption key is derived from your master password using **Argon2id** with a random salt.
- The database stores only ciphertext — the plaintext never touches the disk unencrypted.
- Locking the vault clears the in-memory key until you unlock again.

---

## 📄 License

MIT — free to use, modify and distribute.
