# Hello Gitty v0.2.0

本次更新扩展了 AI 接入方式，并重新整理了界面与设置体验。原有 OpenAI 兼容 API 配置继续可用，旧版用户配置会自动兼容。

## 重点更新

- **支持本地 AI CLI**：可自动扫描并调用 Claude Code、Codex CLI、Gemini CLI、Qwen Code、Ollama、Crush 和 OpenCode。提示词通过标准输入传递，较大的 Git diff 不受命令行长度限制。
- **新增深浅主题**：设置中可切换深色或浅色主题；首次启动会跟随系统偏好，保存后持续使用所选主题。
- **侧栏更易管理**：新增“只显示待提交项目”筛选；侧栏宽度、收起状态和筛选状态均会保存。
- **设置页更清晰**：主题、AI、提交和更新选项分组展示；关闭或取消时会恢复尚未保存的主题预览。
- **界面可读性提升**：优化深浅主题对比度、控件圆角、选中状态、下拉箭头、按钮尺寸和即时悬停提示。
- **更新更稳妥**：安装应用更新前会先保存当前设置，避免重启时丢失刚刚修改的配置。
- **配置向前兼容**：保存设置时会保留当前版本不认识的字段，减少跨版本升级造成的配置丢失。

## 下载

| 平台 | 安装包 |
| --- | --- |
| macOS Apple Silicon | `Hello.Gitty_0.2.0_aarch64.dmg` |
| macOS Intel | `Hello.Gitty_0.2.0_x64.dmg` |
| Windows 10/11 x64 | `Hello.Gitty_0.2.0_x64-setup.exe` |

应用内自动更新使用同一版本的签名更新包，并通过 `latest.json` 覆盖以上三个平台。

> macOS 安装包暂未经过 Apple 公证，首次启动可能需要右键应用并选择“打开”，或在“系统设置 → 隐私与安全性”中允许。
>
> Windows 安装包暂未使用商业代码签名证书，SmartScreen 提示时请确认下载来源为本仓库后选择“仍要运行”。

## 升级说明

- 可直接覆盖安装，无需卸载旧版本。
- 仓库列表、AI 设置、运行命令、侧栏布局和其他本机配置会继续保留。
- 使用本地 CLI 前，请先完成对应工具的安装与登录；Ollama 还需要至少一个已拉取的模型。

---

## English

This release adds local AI CLI integrations and refreshes the interface and settings experience. Existing OpenAI-compatible API configurations remain supported, and settings from older versions continue to load.

### Highlights

- **Local AI CLIs**: automatically scan and invoke Claude Code, Codex CLI, Gemini CLI, Qwen Code, Ollama, Crush, and OpenCode. Prompts are sent through standard input, avoiding command-line length limits for larger Git diffs.
- **Dark and light themes**: switch themes in Settings. First launch follows the system preference; saved selections persist.
- **Sidebar filtering**: show only repositories with uncommitted changes. Sidebar width, collapsed state, and filter state persist.
- **Clearer settings**: appearance, AI, commit, and update options are separated into focused sections. Closing or cancelling restores an unsaved theme preview.
- **Improved legibility**: refined contrast, control radius, selected states, dropdown arrows, button sizing, and instant tooltips in both themes.
- **Safer updates**: current settings are saved before an in-app update is installed and the app restarts.
- **Forward-compatible settings**: unknown fields are preserved when settings are saved, reducing configuration loss across versions.

### Downloads

| Platform | Installer |
| --- | --- |
| macOS Apple Silicon | `Hello.Gitty_0.2.0_aarch64.dmg` |
| macOS Intel | `Hello.Gitty_0.2.0_x64.dmg` |
| Windows 10/11 x64 | `Hello.Gitty_0.2.0_x64-setup.exe` |

In-app updates use signed updater artifacts from the same release. `latest.json` covers all three supported platform targets.

> The macOS package is not Apple-notarized yet. On first launch, you may need to right-click the app and choose Open, or allow it under System Settings → Privacy & Security.
>
> The Windows installer does not yet use a commercial code-signing certificate. If SmartScreen appears, verify that the installer came from this repository before choosing Run anyway.

### Upgrading

- Install directly over the previous version; uninstalling first is not required.
- Repository lists, AI settings, run commands, sidebar layout, and other local settings are preserved.
- Before using a local CLI, install and sign in to that tool. Ollama also requires at least one downloaded model.
