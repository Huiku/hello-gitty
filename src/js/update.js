/* Fork installers are updated manually until a dedicated updater key is configured. */
import { $, toast } from "./state.js";

export function bindUpdateEvents() {
  window.__TAURI__.app.getVersion().then((v) => { $("set-app-version").value = v; })
    .catch(() => { $("set-app-version").value = "未知"; });
  $("btn-check-update").addEventListener("click", () => {
    toast("此版本请前往 github.com/Huiku/hello-gitty 的 Releases 下载安装更新", true);
  });
}

export function startAutoUpdateCheck() {
  // Do not replace this fork with upstream binaries.
}
