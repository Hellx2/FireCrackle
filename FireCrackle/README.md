# FireCrackle

Video labelling and fire-sprite matching app. Current test release: **0.1.0**, Windows x64.

## Build something to share

On Windows with Node.js 24, Rust stable (MSVC), Visual Studio C++ build tools and Windows SDK:

```powershell
Set-Location FireCrackle
npm run release
```

This verifies/downloads pinned FFmpeg, installs dependencies on a fresh checkout (or checks existing ones), runs checks, builds the app and creates a portable ZIP in `releases/v0.1.0/<build>/`. Send that ZIP; testers extract the whole folder and run `FireCrackle.exe`. Keep its adjacent `tools` folder. Testers need Windows 10/11 x64 and Edge WebView2, with no developer tools or separate FFmpeg installation.

For a single installer EXE that installs WebView2 when missing (internet required):

```powershell
npm run release -- -Installer
```

For a RAR, install WinRAR (including `Rar.exe`), then:

```powershell
npm run release -- -Format rar
```

ZIP is the default so Windows can extract it directly. The portable EXE alone cannot open videos without the bundled tools. Builds are unsigned; this pipeline does not publish them or configure an updater.

Each run has a fresh output folder, `release.json` containing commit/build/runtime versions, and SHA-256 checksums. Only the executable, pinned runtime and tester instructions are packaged, never recordings or labels. Normal builds run checks; `-SkipChecks` is an explicit shortcut. `-CleanInstall` forces locked `npm ci` as CI does; stop any dev command first so its native modules can be replaced. Validate prerequisites/configuration without building with `npm run release -- -ValidateOnly`.

FFmpeg comes from [Gyan's Windows builds](https://www.gyan.dev/ffmpeg/builds/), pinned in `release/runtime-lock.json`; its original license/build notices and source links ship in `tools`. Update version, URL, SHA-256 and source together when upgrading. `.release-cache` holds downloads; `src-tauri/runtime` holds generated runtime files. Neither is committed.

## Automated build

The repository's **FireCrackle Windows release** GitHub Actions workflow builds a ZIP and setup EXE on manual dispatch or a `v*` tag. Tags must match `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`. Download the files from workflow artifacts; it does not publish a public GitHub Release. The whole repository is required because the app uses `fire-detection/core` and embedded fire assets.

## Develop

```powershell
npm ci
npm run tauri dev
```

Development uses FFmpeg/FFprobe from `PATH`. Packaged apps prefer absolute paths to `tools` beside their executable, including when opened from a shortcut or another working directory. Labels autosave alongside the video; UI preferences are per Windows account.
