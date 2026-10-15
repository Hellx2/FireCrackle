# Contributing

Keep behavior changes separate from formatting or refactors. Describe the concrete problem, the resulting behavior and the checks you ran. Preserve existing label logs, source tags, autosave compatibility and undo semantics.

## Checks

Run from the repository root:

```powershell
node --test scripts/check-repository.test.mjs
node scripts/check-repository.mjs
cargo fmt --manifest-path FireCrackle/crates/studio_core/Cargo.toml -- --check
cargo fmt --manifest-path FireCrackle/src-tauri/Cargo.toml -- --check
cargo fmt --manifest-path fire-detection/core/Cargo.toml -- --check
cargo test --manifest-path FireCrackle/crates/studio_core/Cargo.toml
cargo test --manifest-path fire-detection/core/Cargo.toml
```

For video integration tests, put FFmpeg/FFprobe on `PATH`; otherwise tests that require them may explicitly skip. For the frontend, run `npm ci`, `npm test` and `npm run build` in `FireCrackle`. The Windows CI installs the pinned runtime before the Rust tests and compiles the desktop command tests. Python and mod instructions are in `fire-detection/README.md`.

`npm run release -- -Installer` in `FireCrackle` builds local tester downloads. Publishing a release or changing repository visibility is a separate action requiring the owner's instruction.

## Source and data boundaries

- Keep synthetic/minimal test fixtures small and explain their purpose.
- Keep recordings, generated feature/label tables and `.firecrackle` projects out of Git.
- Never commit Minecraft source/JARs, decompiled sources, Yarn mapping artifacts, local caches or credentials.
- Dependency declarations and original mod code are allowed; vendoring upstream game/mapping content is not.
- Keep third-party notices intact and do not apply the project's MIT label to upstream assets.

The repository hygiene check catches common mistakes in tracked files. It supplements review; it is not a proof that a file has no sensitive content or that upstream redistribution permission exists.
