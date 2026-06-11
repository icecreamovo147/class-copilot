# Application Icons

- `class-copilot-logo-source.png`: original AI-generated source with chroma-key background.
- `app-icon.png`: transparent 1024x1024 master icon used to generate platform assets.
- `icon.ico`: Windows multi-resolution application and tray icon.
- `icon.icns`: macOS Dock and application icon.
- `tray-icon-macos.png` and `tray-icon-macos@2x.png`: monochrome alpha template icons for the macOS menu bar.
- `tray-icon-windows.png`: color 32x32 icon for the Windows notification area.
- PNG files are generated at native sizes for Tauri and small system surfaces.

When creating the macOS tray icon, enable Tauri's template mode so macOS
automatically adapts it to light and dark menu bars.

Regenerate the platform icon set from `app-icon.png` with:

```bash
npx tauri icon src-tauri/icons/app-icon.png --output src-tauri/icons
```
