# Soundvault

A desktop app that scans a folder of Ableton Live projects, identifies your most frequently used samples across them, and consolidates the top picks into a single organized folder on disk.

**Soundvault is strictly read-only on your project files.** It never opens, writes, modifies, or even touches the timestamps of anything inside a scanned project. The only files it writes are copies of sample files into the output folder you choose.

## Features

- Scan a parent folder recursively or pick individual project folders.
- Classify samples three ways: by group track path (recommended), by filename (auto-detect), or by user-supplied keyword lists (manual).
- Multi-tier dedup: path → filename+size → Blake3 content hash. The same sample copied between projects is correctly clustered.
- Hierarchical taxonomy with sensible defaults for drums; user-overridable via `taxonomy.json`.
- Rank top N (5–50) per category by project count, then clip count, with configurable tiebreaker.
- Output folder mirrors the taxonomy hierarchy; only populated branches are created.
- Real-time progress: discovery → parsing → dedup → copy, with full cancellation.
- Manifest written at output root with full run details (input config, copied samples, source paths, timestamp).
- macOS first; Windows secondary. Cross-platform paths from day one.

## Tech stack

- **Backend:** Tauri 2.0 + Rust (flate2, quick-xml, walkdir, rayon, blake3, serde)
- **Frontend:** React 18 + TypeScript + Tailwind CSS, packaged inside Tauri's webview
- **Read-only safety:** `ReadOnlyProject` wrapper around all project-file access, CI integration test verifies zero writes to any path inside a scanned project root.

## Building

```bash
# Install Node deps
npm install

# Run in dev mode (hot reload front + Rust)
npm run tauri dev

# Build a signed release bundle
npm run tauri build
```

### macOS notarization

Set the following env vars (CI or local) before `npm run tauri build`:

```
APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
APPLE_ID="you@example.com"
APPLE_PASSWORD="@keychain:AC_PASSWORD"   # app-specific password
APPLE_TEAM_ID="TEAMID"
```

Tauri will sign the `.app`, build a `.dmg`, and submit for notarization automatically when these are present. See `src-tauri/tauri.conf.json` → `bundle.macOS` and the embedded `entitlements.plist` for the hardened-runtime config.

## Read-only guarantee

The Rust API surface is shaped so that project content is unreachable through any write-capable path:

1. All project file opens go through `readonly::ReadOnlyProject::open`, which uses `OpenOptions::new().read(true).write(false)`.
2. `discover::discover_projects` only walks directories and reads `.als` headers; it never holds file handles past parsing.
3. The output folder is validated at config-time to ensure it is **not** the same as, or a descendant of, any scanned project root.
4. `copy::copy_samples` writes only to the output root via `fs::copy` (never `rename`, never delete).
5. The CI test in `src-tauri/tests/readonly_test.rs` snapshots filesystem mtimes of a fixture project before/after a full scan and asserts equality.

## Layout

```
soundvault/
  package.json
  vite.config.ts
  tailwind.config.js
  src/                       # React frontend
  src-tauri/                 # Rust backend
    src/                     # Rust modules
    resources/taxonomy.json  # Default taxonomy
    tests/                   # Integration tests
```

## License

MIT
