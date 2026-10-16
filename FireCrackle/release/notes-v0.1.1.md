FireCrackle 0.1.1 for Windows x64 and Linux x86_64.

- **Windows fix:** FFmpeg and FFprobe run without opening console windows during frame decoding, thumbnail generation and video import.
- Photos, pasted screenshots, the selected gallery image, sprite choices and per-image mask settings now survive restarts. Removing photos and clearing the gallery are also saved.
- **Comparison proof** saves an original/masked PNG pair with the displayed sprite's row, sheet, orientation and opacity. First-person comparisons include FOV; editing handles are excluded.
- Header tabs stay in place when switching views. Sprite descriptions show row before sheet, and atlas preview badges appear before candidate badges.
- Includes the completed online-video opener and on-demand tool setup. Required downloads are verified and cached separately from the Windows installer; see the in-app credits for third-party tools.

Windows: use the setup EXE, or extract the ZIP and run `FireCrackle.exe`. Internet access is needed when a required tool is downloaded for the first time. Linux: use the AppImage or Debian package; install your distribution's FFmpeg. See the packaged instructions for requirements.

Verify downloads with `SHA256SUMS.txt` (Windows) or `SHA256SUMS-linux-x64.txt` (Linux). These are unsigned experimental builds. Timing flags are review prompts, not proof of an edit. Original code is MIT-licensed; third-party assets and tools retain their own licenses.
