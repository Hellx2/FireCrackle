Experimental fire-animation labelling and timing-analysis tool for Windows x64 and Linux x86_64.

- Windows: extract the entire ZIP and run `FireCrackle.exe`, keeping `tools` beside it, or use the setup EXE. The installer downloads WebView2 if needed. Builds are unsigned.
- Linux: install your distribution's FFmpeg. Make the AppImage executable, or install the Debian package with `sudo apt install ./FireCrackle-0.1.0-linux-amd64.deb`. Built on Ubuntu 22.04; ARM and musl/Alpine are not included. See `README-linux.txt` for FUSE alternatives and requirements.
- Verify downloads with `SHA256SUMS.txt` (Windows) or `SHA256SUMS-linux-x64.txt` (Linux).

Includes autosave and label import/export, manual quad and first-person previews, matching/filtering tools, and animation timing charts. Timing flags indicate review priority, not proof of an edit. No recordings or personal labels are included.

Original code is MIT-licensed. Minecraft-derived assets and other third-party materials retain separate rights; read the packaged notices. Windows bundles FFmpeg notices and source links; Linux uses system FFmpeg.
