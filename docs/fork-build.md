# Fork builds

Repository: https://github.com/Huiku/hello-gitty

The top-right branch picker now searches local and remote branch names, ignoring case and surrounding whitespace. Arrow keys select a branch, Enter switches, and Escape closes the picker. Clearing the query restores all branches.

## Download and build

The `Build installers` workflow runs on main and feature/** pushes, pull requests, v* tags, and manual dispatch. It checks JavaScript syntax, runs branch interaction tests and Rust tests, and builds:

| Platform | Installer |
| --- | --- |
| Windows x64 | NSIS `-setup.exe` |
| Mac Intel | x64 `.dmg` |
| Mac Apple Silicon | aarch64 `.dmg` |

Download branch builds from the Actions run's Artifacts section (retained for 30 days). When all builds for a v* tag succeed, the workflow publishes a GitHub Release with all installers. No signing secrets are required. Windows CI checks installation, startup and uninstallation; macOS CI validates the ad-hoc signature and DMG integrity.

For manual builds, select Actions → Build installers → Run workflow and choose the feature branch. To release, create a new v* tag on the modified commit.

## Signing and updates

macOS builds use ad-hoc signing without Apple Developer ID notarization. Windows installers do not have Authenticode signing. The operating system may warn about an unidentified developer.

Automatic updates are disabled so upstream binaries cannot replace this fork. Download future installers from https://github.com/Huiku/hello-gitty/releases. Enabling in-app updates later requires an independent Tauri signing key and this fork's update endpoint.

## Local validation

```sh
npm ci
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run build
```

Icon generation needs Python with pillow, numpy and opencv-python-headless. Windows also needs the Rust MSVC toolchain, C++ Build Tools and WebView2; macOS needs Xcode Command Line Tools. GitHub Actions installs the build dependencies.
