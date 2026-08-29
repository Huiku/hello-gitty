# Hello Gitty v0.2.1

这是一个 macOS 安装修复版本。

## 修复内容

- 修复 macOS 应用包签名不完整，下载或自动更新后可能被系统提示“已损坏，无法打开”的问题。
- macOS Apple Silicon 与 Intel 应用现在都会在打包阶段完成 ad-hoc 签名。
- 发布流程新增严格的 `codesign` 校验；签名无效时不会再发布安装包。
- 保留 v0.2.0 的本地 AI CLI、深浅主题、侧栏筛选和界面优化等全部功能。

## 下载

| 平台 | 安装包 |
| --- | --- |
| macOS Apple Silicon | `Hello.Gitty_0.2.1_aarch64.dmg` |
| macOS Intel | `Hello.Gitty_0.2.1_x64.dmg` |
| Windows 10/11 x64 | `Hello.Gitty_0.2.1_x64-setup.exe` |

> 已安装 v0.2.0 且看到“已损坏”提示的 macOS 用户，请下载 v0.2.1 DMG 并覆盖安装。设置与仓库列表不会丢失。
>
> 应用尚未经过 Apple 公证，首次启动仍可能出现“无法验证开发者”提示；右键应用选择“打开”即可。该提示与本次已经修复的“应用已损坏”不同。

---

## English

This is a macOS installation hotfix.

### Fixes

- Fixed an incomplete macOS app-bundle signature that could make downloaded or auto-updated copies appear “damaged” and prevent them from opening.
- Both Apple Silicon and Intel builds are now ad-hoc signed during packaging.
- The release workflow now runs strict `codesign` validation and will reject invalid macOS packages.
- All v0.2.0 features remain included: local AI CLIs, dark/light themes, sidebar filtering, and interface refinements.

### Downloads

| Platform | Installer |
| --- | --- |
| macOS Apple Silicon | `Hello.Gitty_0.2.1_aarch64.dmg` |
| macOS Intel | `Hello.Gitty_0.2.1_x64.dmg` |
| Windows 10/11 x64 | `Hello.Gitty_0.2.1_x64-setup.exe` |

> If v0.2.0 shows a “damaged” warning on macOS, download the v0.2.1 DMG and install it over the existing app. Settings and repository lists are preserved.
>
> The app is not Apple-notarized yet, so first launch may still show an “unidentified developer” warning. Right-click the app and choose Open. That warning is different from the damaged-app failure fixed here.
