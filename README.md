# FPV Shifter

A desktop photo editor that shifts a photo's first-person viewpoint using 4 points. Pin the points to the photo, move them, and the whole photo warps to follow. Then export the result at full resolution as JPEG, PNG, TIFF or WebP. It works fully offline.

Built with Tauri 2: Rust for files and export, plain TypeScript for the editor.

## What you need

- [Node.js](https://nodejs.org) and [Rust](https://rustup.rs)
- The [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your platform

If you installed Rust without letting it edit your shell profile, run `source "$HOME/.cargo/env"` first.

## Run it in development

```bash
npm install
npm run tauri dev
```

## Run the checks

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## Build installers

Tauri builds installers for the system it runs on.

| Platform | Command | Result, in `src-tauri/target/release/bundle/` |
| --- | --- | --- |
| macOS | `npm run tauri build` | `.app` and `.dmg` |
| Windows | `npm run tauri build` | `.exe` and `.msi` |
| Linux | `npm run tauri build` | `.deb` and `.AppImage` |

**macOS:** the `.dmg` step asks Finder to arrange the window, so macOS may ask for permission to control Finder. If it times out, run `CI=true npm run tauri build`, which skips that step.

**Windows from any machine:** push this repository to GitHub and run the `Build for PC` workflow from the Actions tab, or push a tag such as `v0.1.0`. The installers appear as workflow artifacts. It does not publish a release.

## Unsigned installers

The installers are not code-signed.

- **Windows:** SmartScreen warns about the installer until a code-signing certificate is added.
- **macOS:** Gatekeeper blocks the app on other Macs until it is signed and notarized. To open it anyway, right-click the app and choose Open.

## Where things are

- `design/`: the design this app was built from. Start with `design.md`.
- `docs/ARCHITECTURE.md`: how the code fits together.
- `docs/DECISIONS.md`: choices the design did not specify, and known limits.
- `docs/TEST-BY-HAND.md`: what automated tests cannot confirm.
- `npm run fixtures`: regenerates `fixtures/homography.json` from the maths in `design/index.html`.
