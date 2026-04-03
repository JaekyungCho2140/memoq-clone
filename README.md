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

| Platform | Installer | Architecture |
|----------|-----------|-------------|
| **macOS** | `.dmg` | Universal Binary (Apple Silicon + Intel) |
| **Windows** | `.msi` (NSIS) | x64 |
| **Linux** | `.AppImage` | x64 |

## Installation

### macOS

1. Download `memoQ-Clone_*.dmg` from [Releases](https://github.com/JaekyungCho2140/memoq-clone/releases)
2. Open the `.dmg` and drag **memoQ Clone** to Applications
3. On first launch, right-click → Open (if Gatekeeper blocks unsigned app)

### Windows

1. Download `memoQ-Clone_*_x64-setup.exe` or `memoQ-Clone_*_x64_en-US.msi` from [Releases](https://github.com/JaekyungCho2140/memoq-clone/releases)
2. Run the installer and follow the prompts

### Linux

1. Download `memoQ-Clone_*.AppImage` from [Releases](https://github.com/JaekyungCho2140/memoq-clone/releases)
2. Make it executable: `chmod +x memoQ-Clone_*.AppImage`
3. Run: `./memoQ-Clone_*.AppImage`

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

### Code Formatting

Rust 코드는 커밋 전에 반드시 `cargo fmt`를 실행해야 합니다. CI 파이프라인도 포맷을 검사합니다.

```bash
# Rust 코드 포맷 적용 (커밋 전 필수)
cargo fmt --manifest-path src-tauri/Cargo.toml

# 포맷 검사만 (변경 없음)
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

pre-commit hook이 자동으로 설정되어 있어 포맷이 적용되지 않은 경우 커밋이 차단됩니다.
Hook은 `.githooks/pre-commit`에 위치하며 `git config core.hooksPath .githooks`로 활성화됩니다.

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
