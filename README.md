# memoQ Clone

An open-source desktop CAT (Computer-Assisted Translation) tool — a functional clone of [memoQ](https://memoq.com).

Built with **Tauri 2 + React + TypeScript** (frontend) and **Rust** (TM/TB/parser engines).

## Features (MVP Phase 1)

- File import (XLIFF 1.2/2.0, DOCX)
- Segment split editor (source | target view)
- Translation Memory (local TM, fuzzy match)
- Term Base (terminology lookup)
- Target file export

## Tech Stack

| Layer | Technology |
|---|---|
| UI Framework | React 18 + TypeScript |
| Desktop Shell | Tauri 2 |
| State Management | Zustand |
| Bundler | Vite |
| Backend Engine | Rust |
| TM/TB Storage | SQLite (via rusqlite) |
| File Parsers | quick-xml |
| Fuzzy Matching | strsim |

## Platform Support

- **Windows**: `.msi` installer
- **macOS**: `.dmg` installer

## Development Setup

### Prerequisites

```bash
# Rust (via rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js 20+
nvm install 20

# Tauri CLI
cargo install tauri-cli
```

### Run (Development)

```bash
npm install
cargo tauri dev
```

### Test

```bash
# TypeScript tests
npm test

# Rust tests
cargo test --manifest-path src-tauri/Cargo.toml
```

### Build (Production)

```bash
cargo tauri build
# Output: src-tauri/target/release/bundle/
```

## Project Structure

```
memoq-clone/
├── src/                  # React + TypeScript frontend
│   ├── components/       # UI components
│   ├── stores/           # Zustand state stores
│   ├── tauri/            # Tauri IPC command wrappers
│   └── types/            # TypeScript type definitions
├── src-tauri/            # Tauri + Rust backend
│   └── src/
│       ├── commands/     # Tauri command handlers
│       ├── parser/       # File parsers (XLIFF, DOCX)
│       ├── tm/           # Translation Memory engine
│       ├── tb/           # Term Base engine
│       └── models/       # Shared data models
└── .github/workflows/    # CI/CD pipelines
```

## License

MIT
