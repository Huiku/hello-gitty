use crate::git;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MAX_CONFLICT_FILE_BYTES: usize = 80_000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// AI 调用方式:"api" 直连 OpenAI 兼容接口(默认)/ "cli" 调用本地已安装的 AI CLI 工具
    #[serde(default = "default_ai_mode")]
    pub mode: String,
    /// mode=cli 时选中的工具 id(见 CLI_TOOLS 表)
    #[serde(default)]
    pub cli_tool: String,
    /// mode=cli 时工具可执行文件的绝对路径(来自扫描结果,免去每次调用重新探测)
    #[serde(default)]
    pub cli_path: String,
    /// 提交信息语言:"中文" 或 "英文"
    pub lang: String,
    /// 提交模式:"auto" 直接提交(AI 生成后自动提交) / "confirm" 生成后展示确认
    #[serde(default = "default_commit_mode")]
    pub commit_mode: String,
    /// 提交信息提示词预设 id("custom" 表示使用 custom_prompt)
    #[serde(default = "default_preset")]
    pub prompt_preset: String,
    /// 自定义提示词模板(占位符:{diff} {log} {lang})
    #[serde(default)]
    pub custom_prompt: String,
}

fn default_ai_mode() -> String {
    "api".into()
}

fn default_commit_mode() -> String {
    "auto".into()
}

fn default_preset() -> String {
    "conventional".into()
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.deepseek.com".into(),
            api_key: String::new(),
            model: "deepseek-v4-flash".into(),
            mode: default_ai_mode(),
            cli_tool: String::new(),
            cli_path: String::new(),
            lang: "中文".into(),
            commit_mode: default_commit_mode(),
            prompt_preset: default_preset(),
            custom_prompt: String::new(),
        }
    }
}

/// 内置提交信息提示词预设
#[derive(Serialize, Clone)]
pub struct PromptPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system: String,
    /// user 提示模板,支持 {diff} {log} {lang} 占位符
    pub user_template: String,
}

const USER_TAIL: &str = "\n\n以下是本次要提交的全部变更（diff）：\n\n{diff}\n\n请严格按系统消息中的格式生成提交信息：必须包含主题行与正文，正文用要点说明变更原因与主要改动，不要只输出一句话。";

pub fn presets() -> Vec<PromptPreset> {
    vec![
        PromptPreset {
            id: "conventional".into(),
            name: "常规提交".into(),
            description: "Conventional Commits 规范，主题 + 正文，语言跟随上方设置".into(),
            system: "你是一名资深软件工程师，严格遵循 Conventional Commits 等社区最佳实践为代码变更撰写 git 提交信息。\n\n\
【输出格式】主题与正文缺一不可，结构如下：\n\
<type>(<scope>): <subject>\n\
（空行）\n\
<body>\n\n\
【规则】\n\
- type 选自：feat 新功能 / fix 修复 / docs 文档 / style 格式 / refactor 重构 / perf 性能 / test 测试 / build 构建 / ci 持续集成 / chore 杂务；无法判断时用 chore\n\
- scope 可选，表示影响范围（模块或文件）\n\
- subject：祈使句，概括「做了什么」，不超过 50 字，句末不加句号\n\
- body：说明「为什么改」与「主要改了什么」，多项改动用「- 」列要点；不要逐行复述 diff；每行不超过 72 字\n\n\
【示例】\n\
feat(auth): 支持基于 OAuth 的第三方登录\n\n\
- 新增 OAuth 回调处理与 token 自动刷新\n\
- 登录页加入第三方登录入口\n\
- 抽象统一登录接口，便于后续扩展\n\n\
只输出提交信息本身，不要用 ``` 代码块包裹，不要任何前言或解释。".into(),
            user_template: format!("{{lang}}\n\n{{log}}{}", USER_TAIL),
        },
    ]
}

pub fn preset_by_id(id: &str) -> Option<PromptPreset> {
    presets().into_iter().find(|p| p.id == id)
}

/// 根据语言设置生成注入模板的语言指令
pub fn lang_instruction(lang: &str) -> String {
    match lang {
        "英文" => "请用英文撰写提交信息（Write the commit message in English）。".into(),
        _ => "请用简体中文撰写提交信息。".into(),
    }
}

/// 渲染提示词模板:替换 {lang} {log} {diff} 占位符
pub fn render_template(template: &str, diff: &str, log_hint: &str, lang: &str) -> String {
    template
        .replace("{lang}", lang)
        .replace("{log}", log_hint)
        .replace("{diff}", diff)
}

/// AI 调用总入口:按配置分流到 OpenAI 兼容 API 或本地 CLI 工具
async fn chat(cfg: &AiConfig, cwd: &str, system: &str, user: &str) -> Result<String, String> {
    if cfg.mode == "cli" {
        chat_via_cli(cfg, cwd, system, user).await
    } else {
        chat_via_api(cfg, system, user).await
    }
}

async fn chat_via_api(cfg: &AiConfig, system: &str, user: &str) -> Result<String, String> {
    if cfg.api_key.trim().is_empty() {
        return Err("尚未配置 AI API Key，请先打开设置完成配置".into());
    }
    let base = cfg.base_url.trim().trim_end_matches('/');
    let url = format!("{base}/chat/completions");
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "temperature": 0.3
    });
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.api_key.trim()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求 AI 服务失败： {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("AI 服务返回错误（{status}）： {text}"));
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| "AI 服务响应解析失败".to_string())?;
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "AI 服务响应中缺少内容".to_string())?;
    Ok(clean_markdown(content.trim().to_string()))
}

/// 去除模型常见的 ``` 代码块包裹
fn clean_markdown(s: String) -> String {
    let s = s.trim();
    if s.starts_with("```") && s.ends_with("```") {
        let mut lines = s.lines();
        let _ = lines.next();
        let mut out = Vec::new();
        let mut it = lines.peekable();
        while let Some(l) = it.next() {
            if it.peek().is_none() && l.trim() == "```" {
                break;
            }
            out.push(l);
        }
        return out.join("\n").trim().to_string();
    }
    s.to_string()
}

/* ===== 本地 AI CLI 工具:扫描本机已安装的命令行 AI,并经其完成 AI 调用 ===== */

/// 单个 CLI 工具的静态定义:如何找到它、如何以非交互方式调用
struct CliToolDef {
    id: &'static str,
    /// 展示名
    name: &'static str,
    /// 可执行文件名(PATH 中检索用)
    bin: &'static str,
    /// stdin 提示词之前的固定参数(提示词一律经 stdin 传入,不受 argv 长度限制)
    args: &'static [&'static str],
    /// 模型参数;None = 不支持/位置参数特殊处理(如 ollama 的 run <model>)
    model_flag: Option<&'static str>,
    /// 模型输入建议(datalist);有列表子命令(list_args)的工具在扫描时动态覆盖
    models: &'static [&'static str],
    /// 工具自带的模型列表子命令参数(ollama list / crush models / opencode models):
    /// 扫描时动态获取完整清单,保证与终端实际可用一致。
    /// None = 工具未提供列表命令,只能用静态建议(CLI 的 -m 本身接受任意模型 ID)
    list_args: Option<&'static [&'static str]>,
    /// 判定「已配置」的凭据路径(相对 HOME,任一存在即视为已配置;空 = 装好即可用)
    creds: &'static [&'static str],
    /// 给用户的使用前提说明
    hint: &'static str,
}

const CLI_TOOLS: &[CliToolDef] = &[
    CliToolDef {
        id: "claude",
        name: "Claude Code",
        bin: "claude",
        args: &["-p"],
        model_flag: Some("--model"),
        // 别名集 = /model 选择器选项(取自 CLI 内置注册表);再附当前代完整模型 ID
        models: &[
            "default", "sonnet", "opus", "haiku", "opusplan",
            "claude-sonnet-4-5", "claude-opus-4-5", "claude-haiku-4-5",
        ],
        list_args: None, // 无模型列表子命令
        creds: &[".claude", ".claude.json"],
        hint: "使用 Anthropic 账号登录后可用；模型可填别名(opus/sonnet/haiku/opusplan)或带日期后缀的完整模型 ID",
    },
    CliToolDef {
        id: "codex",
        name: "Codex CLI",
        bin: "codex",
        args: &["exec", "-"],
        model_flag: Some("-m"),
        // = CLI 内置模型目录(新) + 文档在列的历史版本,均可经 -m 调用
        models: &[
            "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna",
            "gpt-5.5", "gpt-5.5-pro",
            "gpt-5.4", "gpt-5.4-mini", "gpt-5.4-nano",
            "gpt-5.3-codex", "gpt-5.2", "gpt-5.2-codex",
            "gpt-5.1-codex-max", "gpt-5.1-codex-mini",
        ],
        list_args: None, // 完整目录由 ChatGPT 后端按账号下发,CLI 无列表子命令
        creds: &[".codex/auth.json"],
        hint: "使用 ChatGPT 账号登录后可用；-m 亦可填账号目录内的任意模型 slug",
    },
    CliToolDef {
        id: "gemini",
        name: "Gemini CLI",
        bin: "gemini",
        args: &[],
        model_flag: Some("-m"),
        // CLI 支持的全部文本模型代系(取自 CLI 内置模型表,不含图像/实时等专用变体)
        models: &[
            "gemini-3.5-flash",
            "gemini-3.1-pro", "gemini-3.1-pro-preview",
            "gemini-3.1-flash-lite", "gemini-3.1-flash-lite-preview",
            "gemini-3-pro-preview", "gemini-3-flash", "gemini-3-flash-preview",
            "gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.5-flash-lite",
        ],
        list_args: None, // 可用模型由服务端按账号下发,CLI 无列表子命令
        creds: &[".gemini/oauth_creds.json", ".gemini/settings.json"],
        hint: "使用 Google 账号登录后可用",
    },
    CliToolDef {
        id: "qwen",
        name: "Qwen Code",
        bin: "qwen",
        args: &[],
        model_flag: Some("-m"),
        // coder 系列为 OAuth 免费额度模型,其余代系为 Model Studio API 模型(取自 CLI 内置模型表)
        models: &[
            "qwen3-coder-plus", "qwen3-coder-flash", "qwen3-coder-next",
            "qwen3.8-max", "qwen3.7-max", "qwen3.7-plus",
            "qwen3.6-plus", "qwen3.6-flash", "qwen3.5-plus", "qwen3-max",
        ],
        list_args: None, // 可用模型由服务端下发,CLI 无列表子命令
        creds: &[".qwen/oauth_creds.json"],
        hint: "使用 Qwen 账号登录后可用；coder 系列为免费额度模型",
    },
    CliToolDef {
        id: "ollama",
        name: "Ollama",
        bin: "ollama",
        args: &["run"],
        model_flag: None,
        models: &[],
        list_args: Some(&["list"]),
        creds: &[],
        hint: "本地模型服务，无需联网；必须选择一个已拉取的模型",
    },
    CliToolDef {
        id: "crush",
        name: "Crush",
        bin: "crush",
        args: &["run"],
        model_flag: Some("--model"),
        models: &[],
        list_args: Some(&["models"]),
        creds: &[".config/crush/crush.json"],
        hint: "模型填 provider/model(见建议列表),留空用其默认模型",
    },
    CliToolDef {
        id: "opencode",
        name: "OpenCode",
        bin: "opencode",
        args: &["run"],
        model_flag: Some("-m"),
        models: &[],
        list_args: Some(&["models"]),
        creds: &[".local/share/opencode/auth.json"],
        hint: "模型格式为 provider/model，留空使用其默认模型",
    },
];

fn tool_by_id(id: &str) -> Option<&'static CliToolDef> {
    CLI_TOOLS.iter().find(|t| t.id == id)
}

/// 组装 CLI 命令行参数(不含程序路径与 stdin 提示词)
fn cli_args(def: &CliToolDef, model: &str) -> Vec<String> {
    let mut args: Vec<String> = def.args.iter().map(|s| s.to_string()).collect();
    if def.id == "ollama" {
        // ollama 的模型是位置参数:run <model> < stdin
        if !model.is_empty() {
            args.push(model.to_string());
        }
    } else if let Some(flag) = def.model_flag {
        if !model.is_empty() {
            args.push(flag.to_string());
            args.push(model.to_string());
        }
    }
    args
}

/// 扫描结果(返回给前端渲染选择)
#[derive(Serialize, Clone)]
pub struct CliToolInfo {
    pub id: String,
    pub name: String,
    pub bin: String,
    /// 找到的可执行文件绝对路径;未找到为空
    pub path: String,
    /// `--version` 输出首行(尽力而为)
    pub version: String,
    pub found: bool,
    /// 已安装且凭据检测通过(ollama 需能列出模型)
    pub ready: bool,
    /// 模型建议(datalist)
    pub models: Vec<String>,
    pub hint: String,
}

/// 扫描本机已安装的 AI CLI 工具。检索顺序:进程 PATH → 常见安装目录
/// (GUI 启动的应用 PATH 不含 Homebrew 等)→ 登录 shell(兼容 nvm 等仅在 profile 中配置的安装)
pub async fn scan_cli_tools() -> Vec<CliToolInfo> {
    let dirs = search_dirs();
    let probes = CLI_TOOLS.iter().map(|def| probe_tool(def, &dirs));
    futures_util::future::join_all(probes).await
}

async fn probe_tool(def: &CliToolDef, dirs: &[std::path::PathBuf]) -> CliToolInfo {
    let mut info = CliToolInfo {
        id: def.id.into(),
        name: def.name.into(),
        bin: def.bin.into(),
        path: String::new(),
        version: String::new(),
        found: false,
        ready: false,
        models: def.models.iter().map(|s| s.to_string()).collect(),
        hint: def.hint.into(),
    };
    let mut path = find_in_dirs(def.bin, dirs);
    #[cfg(unix)]
    if path.is_none() {
        path = login_shell_which(def.bin).await;
    }
    let Some(p) = path else { return info };
    info.found = true;
    info.path = p.clone();

    // 凭据检测:尽力而为(macOS 的 Keychain 类凭据文件探测不到也不影响实际可用)
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let cred_ok = def.creds.is_empty() || home.as_ref().is_some_and(|h| {
        def.creds.iter().any(|c| h.join(c).exists())
    });

    // 模型清单:有列表子命令的工具(ollama list / crush models / opencode models)
    // 动态获取完整列表,与终端一致;失败或为空时回落静态建议。
    // ollama 额外以「能列出模型」作为服务就绪依据,其余工具按凭据检测
    let fetched = match def.list_args {
        Some(list_args) => list_models(&p, list_args).await.filter(|m| !m.is_empty()),
        None => None,
    };
    match fetched {
        Some(models) => info.models = models,
        None if def.id == "ollama" => info.models.clear(), // 服务不可达:不给建议
        None => {}
    }
    info.ready = if def.id == "ollama" {
        !info.models.is_empty()
    } else {
        cred_ok
    };
    info.version = probe_version(&p).await;
    info
}

/// 追加的检索目录:补齐 GUI 启动时缺失的 PATH(Homebrew/用户级安装)
fn search_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(pd) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&pd));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from);
    if let Some(h) = &home {
        for rel in ["bin", ".local/bin", ".cargo/bin"] {
            dirs.push(h.join(rel));
        }
    }
    for d in ["/opt/homebrew/bin", "/usr/local/bin"] {
        dirs.push(std::path::PathBuf::from(d));
    }
    // 去重(保持首次出现顺序)
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| !d.as_os_str().is_empty() && seen.insert(d.clone()));
    dirs
}

/// 在候选目录中查找可执行文件
fn find_in_dirs(bin: &str, dirs: &[std::path::PathBuf]) -> Option<String> {
    for dir in dirs {
        let cand = dir.join(bin);
        if is_executable_file(&cand) {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    None
}

fn is_executable_file(p: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        p.is_file()
            && p.metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        p.is_file()
    }
}

/// 经登录 shell 探测命令位置:`$SHELL -l -c 'command -v <bin>'`,
/// 兼容只在 shell profile 里配置 PATH 的安装方式(nvm/volta 等)。限时防卡。
#[cfg(unix)]
async fn login_shell_which(bin: &str) -> Option<String> {
    use std::process::Stdio;
    use tokio::process::Command;
    let shell = std::env::var("SHELL").ok().filter(|s| !s.trim().is_empty())?;
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        Command::new(&shell)
            .arg("-l")
            .arg("-c")
            .arg(format!("command -v {bin}"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !out.status.success() {
        return None;
    }
    // command -v 对 alias/函数会输出不带路径的名字,子进程无法执行,丢弃
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !s.contains('/') {
        return None;
    }
    is_executable_file(std::path::Path::new(&s)).then_some(s)
}

/// 探测版本号(首行,尽力而为)
async fn probe_version(bin: &str) -> String {
    use std::process::Stdio;
    use tokio::process::Command;
    let Ok(res) = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        Command::new(bin)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await
    else {
        return String::new();
    };
    let Ok(out) = res else { return String::new() };
    let src = if out.stdout.is_empty() { &out.stderr } else { &out.stdout };
    String::from_utf8_lossy(src)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .chars()
        .take(80)
        .collect()
}

/// 解析模型列表命令输出:ollama list(表头 + 多列)与 crush models / opencode models
/// (每行一个 provider/model)统一取每行首个空白分隔字段,跳过表头与空行
fn parse_model_list(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("NAME"))
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

/// 列表命令超时:crush models 首次运行需构建提供方目录,实测可超 3 秒
const MODELS_LIST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// 运行工具自带的模型列表子命令(`ollama list` / `crush models` / `opencode models`),
/// 解析 stdout 为模型名列表;命令失败返回 None
async fn list_models(bin: &str, args: &[&str]) -> Option<Vec<String>> {
    use std::process::Stdio;
    use tokio::process::Command;
    let res = tokio::time::timeout(
        MODELS_LIST_TIMEOUT,
        Command::new(bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !res.status.success() {
        return None;
    }
    Some(parse_model_list(&String::from_utf8_lossy(&res.stdout)))
}

/// 子进程 PATH:工具所在目录优先,再补上常见安装目录,
/// 保证 `#!/usr/bin/env node` 之类的 shebang 能在 GUI 启动的极简 PATH 下找到解释器
fn child_path(bin: &str) -> String {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(p) = std::path::Path::new(bin).parent() {
        if !p.as_os_str().is_empty() {
            dirs.push(p.to_path_buf());
        }
    }
    dirs.extend(search_dirs());
    std::env::join_paths(&dirs)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// CLI 调用超时:CLI 冷启动 + 大 diff 推理可能较慢
const CLI_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// 经本地 CLI 完成一次 AI 调用:提示词经 stdin 传入,取 stdout 为回复
async fn chat_via_cli(cfg: &AiConfig, cwd: &str, system: &str, user: &str) -> Result<String, String> {
    let bin = cfg.cli_path.trim();
    if bin.is_empty() {
        return Err("尚未选择本地 CLI 工具，请到 设置 → AI 模型配置 中扫描并选择".into());
    }
    let def = tool_by_id(&cfg.cli_tool)
        .ok_or("未知的 CLI 工具，请到设置中重新扫描并选择")?;
    if !std::path::Path::new(bin).exists() {
        return Err(format!("CLI 工具不存在：{bin}（可能已卸载或移动，请重新扫描）"));
    }
    let model = cfg.model.trim();
    if def.id == "ollama" && model.is_empty() {
        return Err("Ollama 必须指定模型，请到设置中选择".into());
    }

    // CLI 没有独立的 system 通道:系统指令与任务合并为一条提示词
    let prompt = format!("【系统指令】\n{system}\n\n【任务】\n{user}");

    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(cli_args(def, model));
    if !cwd.is_empty() {
        cmd.current_dir(cwd);
    }
    cmd.env("PATH", child_path(bin));
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 {} 失败：{e}", def.name))?;
    // 提示词含完整 diff,可能超出管道缓冲:独立任务写 stdin,
    // 与 stdout 读取并发进行,否则双方互相等待会死锁
    if let Some(mut stdin) = child.stdin.take() {
        tauri::async_runtime::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(prompt.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }
    let out = tokio::time::timeout(CLI_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| format!("{} 执行超时（超过 {} 秒）", def.name, CLI_TIMEOUT.as_secs()))?
        .map_err(|e| format!("执行 {} 失败：{e}", def.name))?;
    if !out.status.success() {
        let err = tail_chars(
            &String::from_utf8_lossy(&out.stderr),
            400,
        );
        return Err(format!(
            "{} 返回错误（exit code {}）：{}",
            def.name,
            out.status.code().unwrap_or(-1),
            if err.is_empty() { "无错误输出".into() } else { err }
        ));
    }
    let text = clean_markdown(String::from_utf8_lossy(&out.stdout).trim().to_string());
    if text.is_empty() {
        return Err(format!("{} 未返回内容", def.name));
    }
    Ok(text)
}

/// 取文本末尾最多 n 个字符(CLI 的报错通常在输出末尾)
fn tail_chars(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.trim().chars().collect();
    if chars.len() <= n {
        chars.into_iter().collect()
    } else {
        chars[chars.len() - n..].iter().collect()
    }
}

/// 构建提交信息的 (system, user) 提示词:暂存差异 + 近期提交参考 + 常规提交预设 + 用户额外要求
fn build_commit_prompt(cfg: &AiConfig, repo: &str) -> Result<(String, String), String> {
    // 提交语义:只提交已暂存内容,AI 信息基于暂存差异
    let diff = git::diff_for_ai(repo, true)?;
    let log = git::recent_log(repo, 8);
    let log_hint = if log.is_empty() {
        String::new()
    } else {
        format!("以下是该仓库近期的提交记录，仅供你参考 type 前缀与语言习惯，不要照搬其长度或结构：\n{}\n", log.join("\n"))
    };
    let lang = lang_instruction(&cfg.lang);

    // 固定使用最佳实践预设(常规提交)作为提示词引擎;用户填写的「额外要求」是最高优先级指令,
    // 与预设/语言/近期记录等任何其他规则冲突时一律以它为准,同时注入 system 与 user 双保险
    let p = preset_by_id("conventional").expect("conventional 预设必然存在");
    let extra = cfg.custom_prompt.trim();
    let system = if extra.is_empty() {
        p.system.clone()
    } else {
        format!(
            "{}\n\n【用户额外要求·最高优先级】这是用户对本次提交信息的硬性要求，优先于以上所有规则（包括语言、格式、长度约定）:\n{}",
            p.system,
            extra
        )
    };
    // 有额外要求时不再注入语言指令:语言归属额外要求管辖,避免与硬编码默认(中文)互相矛盾
    let lang_for_user = if extra.is_empty() {
        lang
    } else {
        String::new()
    };
    let user = render_template(&p.user_template, &diff, &log_hint, &lang_for_user);
    // 额外要求贴近 diff 注入 user 消息:紧邻输入内容的位置模型遵循度最高
    let user = if extra.is_empty() {
        user
    } else {
        format!(
            "{}\n\n【用户额外要求·最高优先级】与系统消息中的任何规则冲突时，一律按以下要求执行：\n{}",
            user,
            extra
        )
    };
    Ok((system, user))
}

pub async fn generate_commit_message(cfg: &AiConfig, repo: &str) -> Result<String, String> {
    let (system, user) = build_commit_prompt(cfg, repo)?;
    let msg = chat(cfg, repo, &system, &user).await?;
    if msg.is_empty() {
        return Err("AI 未生成提交信息".into());
    }
    Ok(msg)
}

/// 流式生成提交信息:逐 token 经 on_delta 回调推送已累积的全文,最终返回(已去围栏)全文。
/// on_delta 收到的是当前累积的完整文本,前端可直接整体回填,避免增量拼接。
pub async fn generate_commit_message_stream(
    cfg: &AiConfig,
    repo: &str,
    on_delta: impl Fn(&str) + Send + Sync + 'static,
) -> Result<String, String> {
    // CLI 模式:一次性执行整段提示词,结果作为单个增量推送(前端按全文回填,无需逐 token)
    if cfg.mode == "cli" {
        let (system, user) = build_commit_prompt(cfg, repo)?;
        let msg = chat_via_cli(cfg, repo, &system, &user).await?;
        on_delta(&msg);
        return Ok(msg);
    }
    if cfg.api_key.trim().is_empty() {
        return Err("尚未配置 AI API Key，请先打开设置完成配置".into());
    }
    let (system, user) = build_commit_prompt(cfg, repo)?;
    let base = cfg.base_url.trim().trim_end_matches('/');
    let url = format!("{base}/chat/completions");
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "temperature": 0.3,
        "stream": true
    });
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.api_key.trim()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求 AI 服务失败： {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        // 流式接口出错时 body 仍是普通 JSON 错误
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("AI 服务返回错误（{status}）： {text}"));
    }

    // SSE 流式解析:按字节块读入行缓冲,提取 `data: {...}` 中的 choices[0].delta.content
    let mut stream = resp.bytes_stream();
    // 按字节缓冲:网络分块边界落在任意字节上,多字节字符(如中文)可能被劈在
    // 两个 chunk 里,对整块做 from_utf8 会误报"非法 UTF-8";只对完整行解码
    // (\n 的字节值不会出现在 UTF-8 多字节序列中,按字节切行是安全的)
    let mut buf: Vec<u8> = Vec::new();
    let mut full = String::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("读取 AI 流式响应失败： {e}"))?;
        buf.extend_from_slice(&bytes);
        // 处理缓冲里所有完整行
        while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            if line.is_empty() || !line.starts_with("data:") {
                continue;
            }
            let data = line["data:".len()..].trim();
            if data == "[DONE]" {
                let result = clean_markdown(full.clone());
                on_delta(&result);
                return Ok(result);
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = v.pointer("/choices/0/delta/content").and_then(|c| c.as_str()) {
                    full.push_str(delta);
                    on_delta(&full);
                }
            }
        }
    }
    // 流自然结束(未收到 [DONE])— 兜底返回累积内容
    if full.trim().is_empty() {
        return Err("AI 未生成提交信息".into());
    }
    let result = clean_markdown(full);
    on_delta(&result);
    Ok(result)
}

pub async fn resolve_conflict_file(cfg: &AiConfig, repo: &str, path: &str) -> Result<(), String> {
    let full = Path::new(repo).join(path);
    let content = std::fs::read_to_string(&full)
        .map_err(|e| format!("读取 {path} 失败： {e}"))?;
    if content.len() > MAX_CONFLICT_FILE_BYTES {
        return Err(format!("{path} 过大（{}KB），超出 AI 处理上限，请手动解决", content.len() / 1024));
    }
    if !content.contains("<<<<<<<") && !content.contains("=======") {
        // 无冲突标记(例如文件级删除冲突),跳过
        return Err(format!("{path} 未包含冲突标记，请手动处理"));
    }
    let system = "你是一名擅长解决 git 合并冲突的资深工程师。合并冲突时保留双方代码的正确意图，保证语法与语义正确，不留下任何冲突标记（<<<<<<< ======= >>>>>>>），也不添加解释性文字。只输出合并后的完整文件内容。";
    let user = format!("文件路径：{path}\n\n文件内容（含冲突标记）：\n```\n{content}\n```\n\n请输出解决冲突后的完整文件内容。");
    let resolved = chat(cfg, repo, system, &user).await?;
    // 防御:AI 可能再次包上代码块
    let resolved = clean_markdown(resolved);
    std::fs::write(&full, resolved).map_err(|e| format!("写入 {path} 失败： {e}"))?;
    git::stage_file(repo, path)?;
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct ConflictOutcome {
    pub path: String,
    pub ok: bool,
    pub error: Option<String>,
}

pub async fn resolve_all_conflicts(
    cfg: &AiConfig,
    repo: &str,
    on_progress: impl Fn(usize, usize, &str) + Send + Sync + 'static,
) -> Result<Vec<ConflictOutcome>, String> {
    let files = git::conflict_files(repo);
    if files.is_empty() {
        return Err("当前没有待解决的冲突".into());
    }
    let total = files.len();
    let mut outcomes = Vec::with_capacity(total);
    for (i, path) in files.iter().enumerate() {
        on_progress(i + 1, total, path);
        match resolve_conflict_file(cfg, repo, path).await {
            Ok(()) => outcomes.push(ConflictOutcome { path: path.clone(), ok: true, error: None }),
            Err(e) => outcomes.push(ConflictOutcome { path: path.clone(), ok: false, error: Some(e) }),
        }
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_placeholders() {
        let out = render_template("{lang} | {log} | {diff}", "DIFF", "LOG", "LANG");
        assert_eq!(out, "LANG | LOG | DIFF");
    }

    #[test]
    fn presets_lookup_and_fallback() {
        assert!(preset_by_id("conventional").is_some());
        assert!(preset_by_id("custom").is_none(), "custom 不应是内置预设");
        assert!(preset_by_id("nonexistent").is_none());
        // 全部预设的 user 模板必须包含 diff 占位符
        for p in presets() {
            assert!(p.user_template.contains("{diff}"), "{} 模板缺 {{diff}}", p.id);
        }
    }

    #[test]
    fn lang_instruction_both() {
        assert!(lang_instruction("中文").contains("简体中文"));
        assert!(lang_instruction("英文").contains("English"));
    }

    #[test]
    fn default_commit_mode_is_auto() {
        assert_eq!(AiConfig::default().commit_mode, "auto");
    }

    /* ===== 本地 CLI 相关 ===== */

    #[test]
    fn default_mode_is_api() {
        assert_eq!(AiConfig::default().mode, "api");
        // 旧配置 JSON 无 mode 字段:反序列化默认走 api,保证向后兼容
        let legacy: AiConfig = serde_json::from_str(
            r#"{"base_url":"https://x.com","api_key":"k","model":"m","lang":"中文"}"#,
        )
        .unwrap();
        assert_eq!(legacy.mode, "api");
    }

    #[test]
    fn tool_table_lookup() {
        assert!(tool_by_id("claude").is_some());
        assert!(tool_by_id("ollama").is_some());
        assert!(tool_by_id("nonexistent").is_none());
    }

    #[test]
    fn cli_args_per_tool() {
        let claude = tool_by_id("claude").unwrap();
        assert_eq!(cli_args(claude, "sonnet"), vec!["-p", "--model", "sonnet"]);
        assert_eq!(cli_args(claude, ""), vec!["-p"]); // 留空用 CLI 默认模型

        let ollama = tool_by_id("ollama").unwrap();
        assert_eq!(cli_args(ollama, "qwen3:8b"), vec!["run", "qwen3:8b"]);
        assert_eq!(cli_args(ollama, ""), vec!["run"]); // 模型必填在调用前已校验

        let gemini = tool_by_id("gemini").unwrap();
        assert_eq!(cli_args(gemini, "gemini-2.5-pro"), vec!["-m", "gemini-2.5-pro"]);
        assert_eq!(cli_args(gemini, ""), Vec::<String>::new());

        let crush = tool_by_id("crush").unwrap();
        assert_eq!(cli_args(crush, "opencode/glm-5"), vec!["run", "--model", "opencode/glm-5"]);
        assert_eq!(cli_args(crush, ""), vec!["run"]); // 留空用其默认模型
    }

    #[test]
    fn model_list_parsing() {
        // ollama list:表头 + 多列,取每行首字段
        let out = "NAME              ID              SIZE     MODIFIED\n\
                   qwen3:8b          a8c0             5.2 GB   2 days ago\n\
                   \n\
                   deepseek-r1:14b   ea35            9.0 GB   3 weeks ago";
        assert_eq!(parse_model_list(out), vec!["qwen3:8b", "deepseek-r1:14b"]);
        // crush/opencode models:每行一个 provider/model
        assert_eq!(
            parse_model_list("opencode/gpt-5.5\nalibaba/glm-5\n"),
            vec!["opencode/gpt-5.5", "alibaba/glm-5"]
        );
        assert!(parse_model_list("").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn find_in_dirs_locates_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("hello-gitty-cli-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("fake-ai-cli");
        std::fs::write(&fake, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let found = find_in_dirs("fake-ai-cli", &[dir.clone()]);
        assert_eq!(found, Some(fake.to_string_lossy().into_owned()));

        // 无执行权限的文件不算可执行
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(find_in_dirs("fake-ai-cli", &[dir.clone()]), None);
        // 缺失的命令返回 None
        assert_eq!(find_in_dirs("missing-cli", &[dir.clone()]), None);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn tail_chars_keeps_suffix() {
        assert_eq!(tail_chars("abcdef", 3), "def");
        assert_eq!(tail_chars("ab", 10), "ab");
        // 多字节字符按字符截断,不劈开 UTF-8
        assert_eq!(tail_chars("你好世界", 2), "世界");
    }

    /// 临时 git 仓库(与 git.rs 测试同款模式)
    fn temp_repo(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hellogitty-ai-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let r = dir.to_str().unwrap();
        crate::git::run_git(r, &["init", "-q"]).unwrap();
        crate::git::run_git(r, &["config", "user.email", "test@example.com"]).unwrap();
        crate::git::run_git(r, &["config", "user.name", "test"]).unwrap();
        dir
    }

    #[test]
    fn custom_prompt_is_highest_priority() {
        let repo = temp_repo("custom-prompt");
        std::fs::write(repo.join("a.txt"), "hello").unwrap();
        crate::git::run_git(repo.to_str().unwrap(), &["add", "a.txt"]).unwrap();

        let base = AiConfig {
            custom_prompt: "使用英文撰写提交信息".into(),
            ..AiConfig::default()
        };
        let (system, user) = build_commit_prompt(&base, repo.to_str().unwrap()).unwrap();

        // system 与 user 都包含最高优先级声明 + 用户原文
        assert!(system.contains("最高优先级"));
        assert!(system.contains("使用英文撰写提交信息"));
        assert!(user.contains("最高优先级"));
        assert!(user.contains("使用英文撰写提交信息"));
        // 有额外要求时,语言指令(简体中文)被抑制,避免与额外要求冲突
        assert!(!user.contains("简体中文"));

        // 无额外要求时保持原行为:语言指令正常注入
        let none = AiConfig::default();
        let (_, user_none) = build_commit_prompt(&none, repo.to_str().unwrap()).unwrap();
        assert!(user_none.contains("简体中文"));
        assert!(!user_none.contains("最高优先级"));

        std::fs::remove_dir_all(repo).ok();
    }
}
