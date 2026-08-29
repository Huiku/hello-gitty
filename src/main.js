/* Hello Gitty 前端入口:应用初始化、设置对话框、全局弹层收起。
   各功能域拆分在 js/ 下(ES modules,无打包器):state(共享状态/工具)、
   sidebar(侧栏/项目管理)、panel(仓库面板)、git-ops(Git 操作/AI)、
   history(历史)、run-panel(运行服务器)、dashboard(多仓库总览)。 */
import { $, invoke, toast, DEFAULT_AI, settings, setSettings, setRepo, repo, applyTheme, DEFAULT_THEME } from "./js/state.js";
import { loadRepos, fetchRemote, setupDragDrop, bindSidebarEvents, closeCtxMenu } from "./js/sidebar.js";
import { refresh, showEmpty, bindPanelEvents } from "./js/panel.js";
import { bindHistoryEvents } from "./js/history.js";
import { bindGitEvents, initGitListeners } from "./js/git-ops.js";
import { syncRunPanel, syncRunToggle, bindRunEvents, initRunListeners, refreshExternalStatus, closeRunCtxMenu, closeRunMoreMenu } from "./js/run-panel.js";
import { bindDashboardEvents } from "./js/dashboard.js";
import { bindUpdateEvents, startAutoUpdateCheck } from "./js/update.js";
import { bindRepoFileWatcher } from "./js/file-watcher.js";
import { applyUiScale } from "./js/ui-scale.js";
import { initTooltip } from "./js/tooltip.js";

const SIDEBAR_COMPACT_MAX = 96; // 简洁展示的宽度上限(含);与 sidebar.js 保持一致

init();

async function init() {
  try {
    setSettings(await invoke("settings_load"));
    settings.ai = { ...DEFAULT_AI, ...settings.ai }; // 兼容旧配置,补齐新字段
    settings.repos = settings.repos || [];
  } catch (_) { settings.repos = []; }
  // 先用 settings 主题,但若 settings 是首次启动(无 theme 字段)走 DEFAULT_THEME 跟随系统
  applyTheme(settings.theme || DEFAULT_THEME);
  applyUiScale();
  // 旧配置迁移:last_repo 不在列表时补进列表
  if (settings.last_repo && !settings.repos.includes(settings.last_repo)) {
    settings.repos.push(settings.last_repo);
    try { await invoke("settings_save", { settings }); } catch (_) {}
  }
  setRepo(settings.last_repo && settings.repos.includes(settings.last_repo)
    ? settings.last_repo
    : (settings.repos[0] || null));

  // 恢复侧栏宽度;简洁/全面展示由宽度自动决定
  let sbw = settings.sidebar_width || 172;
  if (settings.sidebar_collapsed && sbw > SIDEBAR_COMPACT_MAX) sbw = 48; // 兼容旧配置
  $("sidebar").style.width = sbw + "px";
  // 恢复右侧 diff 面板宽度(拖拽调整过则用记忆值)
  if (settings.diff_width) $("diff-panel").style.width = settings.diff_width + "px";
  // 恢复运行日志面板高度(拖拽调整过则用记忆值)
  if (settings.run_height) $("run-panel").style.setProperty("--run-panel-height", settings.run_height + "px");

  bindSidebarEvents();
  bindPanelEvents();
  bindHistoryEvents();
  bindGitEvents();
  bindRunEvents();
  bindDashboardEvents();
  bindDialogEvents();
  bindUpdateEvents();
  await bindRepoFileWatcher();
  bindGlobalDismiss();
  initTooltip(); // 自定义 tooltip:悬停立即显示,替代原生 title 延迟提示

  initGitListeners();
  initRunListeners();
  setupDragDrop();

  await loadRepos();
  if (repo) {
    await refresh();
    fetchRemote(); // 启动时后台核对一次远程状态
  } else {
    // 无项目:默认选中「首页」Tab,右侧展示新建项目空态(而非空白)
    const { showOverview } = await import("./js/dashboard.js");
    await showOverview();
  }
  syncRunPanel(); // 运行面板:回填命令、刷新日志与运行态
  syncRunToggle(); // 展开/收起按钮初始态(日志区默认收起,箭头朝下)
  // 定时后台 fetch:远程有新提交时,ahead/behind 徽标与远程历史自动更新
  setInterval(fetchRemote, 60_000);
  // 定期重查外部运行态:在别处启动/停止项目后,按钮状态自动跟随(端口探测开销极小)
  setInterval(refreshExternalStatus, 30_000);
  startAutoUpdateCheck(); // 启动稍后静默检查应用更新,有新版本才弹窗
}

/* ===== 设置 ===== */
// CLI 工具扫描缓存(本次会话内复用)与设置弹窗中临时的选中项 { id, path }
let cliScanCache = null;
let cliSel = null;
let themeBeforeSettings = DEFAULT_THEME;

function openSettings() {
  themeBeforeSettings = settings.theme || DEFAULT_THEME;
  $("set-theme").value = themeBeforeSettings;
  $("set-ai-mode").value = settings.ai.mode === "cli" ? "cli" : "api";
  $("set-base-url").value = settings.ai.base_url || DEFAULT_AI.base_url;
  $("set-api-key").value = settings.ai.api_key || "";
  $("set-model").value = settings.ai.model || DEFAULT_AI.model;
  $("set-cli-model").value = settings.ai.mode === "cli" ? (settings.ai.model || "") : "";
  $("set-commit-mode").value = settings.ai.commit_mode || "auto";
  $("set-custom-prompt").value = settings.ai.custom_prompt || "";
  // 已保存的 CLI 选择先按保存值回显;扫描完成后若已不在结果中会被清掉
  cliSel = settings.ai.cli_tool ? { id: settings.ai.cli_tool, path: settings.ai.cli_path } : null;
  syncAiModeFields();
  if ($("set-ai-mode").value === "cli") ensureCliScan();

  $("dlg-settings").classList.remove("hidden");
}

function closeSettings(restoreTheme = false) {
  if (restoreTheme) applyTheme(themeBeforeSettings);
  $("dlg-settings").classList.add("hidden");
}

// 调用方式切换:直连 API 与本地 CLI 两组字段互斥显示
function syncAiModeFields() {
  const cli = $("set-ai-mode").value === "cli";
  $("ai-api-fields").classList.toggle("hidden", cli);
  $("ai-cli-fields").classList.toggle("hidden", !cli);
}

async function saveSettings() {
  // 主题已在选择时预览;保存时只需写入配置
  const theme = applyTheme($("set-theme").value);
  const mode = $("set-ai-mode").value;
  if (mode === "cli" && !cliSel) {
    toast("请先扫描并选择一个 CLI 工具", false);
    return;
  }
  settings.theme = theme;
  settings.ai = {
    ...settings.ai, // 保留表单外字段(如 prompt_preset)
    mode,
    cli_tool: mode === "cli" ? cliSel.id : "",
    cli_path: mode === "cli" ? cliSel.path : "",
    base_url: $("set-base-url").value.trim() || DEFAULT_AI.base_url,
    api_key: $("set-api-key").value.trim(),
    // CLI 模式下模型可为空(使用该 CLI 自身的默认模型)
    model: mode === "cli"
      ? $("set-cli-model").value.trim()
      : ($("set-model").value.trim() || DEFAULT_AI.model),
    lang: "中文", // 语言配置已从 UI 移除,固定中文(后端反序列化需要该字段)
    commit_mode: $("set-commit-mode").value,
    custom_prompt: $("set-custom-prompt").value,
  };
  try {
    await invoke("settings_save", { settings });
    closeSettings();
    toast("设置已保存", true);
  } catch (e) {
    settings.theme = themeBeforeSettings;
    applyTheme(themeBeforeSettings);
    toast("保存失败：" + e, false);
  }
}


/* ===== 本地 CLI 工具扫描(设置 → AI 模型配置) ===== */
// 已扫描过则直接用缓存;未扫描过(切到 CLI 或打开设置时)现场扫一次
async function ensureCliScan() {
  if (cliScanCache) { renderCliList(); return; }
  await scanCliTools();
}

async function scanCliTools() {
  const btn = $("btn-cli-rescan");
  const select = $("cli-tool-select");
  btn.disabled = true;
  select.disabled = true;
  select.innerHTML = '<option value="">正在扫描本机 CLI 工具…</option>';
  try {
    cliScanCache = await invoke("ai_cli_scan");
  } catch (e) {
    cliScanCache = [];
    select.innerHTML = "";
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "扫描失败：" + e;
    select.appendChild(option);
    select.disabled = false;
    btn.disabled = false;
    return;
  }
  btn.disabled = false;
  renderCliList();
}

// 将扫描结果放入下拉框;未安装工具保留展示但不可选
function renderCliList() {
  const select = $("cli-tool-select");
  const found = (cliScanCache || []).filter((t) => t.found);
  if (cliSel && !found.some((t) => t.id === cliSel.id)) cliSel = null;

  select.innerHTML = "";
  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = found.length ? "请选择 CLI 工具" : "未发现可用 CLI 工具";
  select.appendChild(placeholder);
  for (const t of cliScanCache || []) {
    const option = document.createElement("option");
    option.value = t.id;
    option.textContent = t.name + (t.found ? (t.ready ? "（已配置）" : "（未就绪）") : "（未安装）");
    option.title = t.found ? (t.version || t.path) : t.hint;
    option.disabled = !t.found;
    select.appendChild(option);
  }
  select.value = cliSel ? cliSel.id : "";
  select.disabled = false;
  const selected = (cliScanCache || []).find((t) => cliSel && t.id === cliSel.id);
  if (selected) fillCliModels(selected);
  else $("cli-model-list").innerHTML = "";
}

// 选中工具后回填模型建议(datalist)
function fillCliModels(t) {
  $("cli-model-list").innerHTML = "";
  for (const m of t.models || []) {
    const opt = document.createElement("option");
    opt.value = m;
    $("cli-model-list").appendChild(opt);
  }
}
function bindDialogEvents() {
  $("btn-settings").addEventListener("click", openSettings);
  $("btn-settings-cancel").addEventListener("click", () => closeSettings(true));
  $("btn-settings-save").addEventListener("click", saveSettings);
  $("btn-settings-close").addEventListener("click", () => closeSettings(true));
  $("set-theme").addEventListener("change", (e) => applyTheme(e.target.value));
  // AI 调用方式切换;切到 CLI 时按需触发扫描
  $("set-ai-mode").addEventListener("change", () => {
    syncAiModeFields();
    if ($("set-ai-mode").value === "cli") ensureCliScan();
  });
  $("cli-tool-select").addEventListener("change", (e) => {
    const selected = (cliScanCache || []).find((t) => t.id === e.target.value && t.found);
    cliSel = selected ? { id: selected.id, path: selected.path } : null;
    if (selected) fillCliModels(selected);
    else $("cli-model-list").innerHTML = "";
  });
  $("btn-cli-rescan").addEventListener("click", scanCliTools);
  // 设置页左侧菜单:切换分组
  document.querySelectorAll(".settings-nav-item").forEach((btn) => {
    btn.addEventListener("click", () => {
      const tab = btn.dataset.tab;
      document.querySelectorAll(".settings-nav-item").forEach((b) => b.classList.toggle("active", b === btn));
      document.querySelectorAll(".settings-tab").forEach((t) => t.classList.toggle("hidden", t.dataset.tab !== tab));
    });
  });
}

/* ===== 全局弹层收起 ===== */
// 点击空白处 / ESC:收起所有弹层;任何容器滚动都收起右键菜单(fixed 定位不跟随滚动)
function bindGlobalDismiss() {
  document.addEventListener("click", () => {
    $("more-menu").classList.add("hidden");
    $("dash-more-menu").classList.add("hidden");
    $("branch-menu").classList.add("hidden");
    closeCtxMenu();
    closeRunCtxMenu();
    closeRunMoreMenu();
  });
  // 总览卡片命令下拉框:点击页面任意空白处立即失焦,取消激活态
  // (WKWebView 下原生 select 点击外部不会自动失焦;捕获阶段监听,不受各控件 stopPropagation 影响)
  document.addEventListener("click", (e) => {
    if (e.target instanceof Element && e.target.closest(".run-cmd-select")) return; // 点的是下拉框本身,保持展开
    const el = document.activeElement;
    if (el?.classList?.contains("run-cmd-select")) el.blur();
  }, true);
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      $("more-menu").classList.add("hidden");
      $("dash-more-menu").classList.add("hidden");
      $("diff-panel").classList.add("hidden"); // ESC 收起右侧 diff 面板
      closeCtxMenu();
      closeRunCtxMenu();
      closeRunMoreMenu();
    }
  });
  window.addEventListener("scroll", () => { closeCtxMenu(); closeRunCtxMenu(); closeRunMoreMenu(); }, true);
}
