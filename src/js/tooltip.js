/* 全局自定义 tooltip:悬停 [data-tip] 元素立即显示,替代原生 title 的延迟系统提示。
   单例浮层 fixed 定位,默认出现在目标上方(底栏按钮),顶部空间不足翻到下方;
   pointer-events:none 不挡交互。文案动态变化后由调用方调 refreshTip 就地刷新。 */
import { $ } from "./state.js";

let cur = null; // 提示当前显示中的目标元素

function show(el) {
  const text = el.dataset.tip;
  if (!text) { hide(); return; }
  const tip = $("tooltip");
  tip.textContent = text;
  tip.classList.remove("hidden");
  // 水平以目标为中心并钳制在视口内;垂直默认上方,顶部放不下翻到下方
  const r = el.getBoundingClientRect();
  const tr = tip.getBoundingClientRect();
  let x = r.left + r.width / 2 - tr.width / 2;
  x = Math.max(4, Math.min(x, window.innerWidth - tr.width - 4));
  let y = r.top - tr.height - 6;
  if (y < 4) y = r.bottom + 6;
  tip.style.left = x + "px";
  tip.style.top = y + "px";
  cur = el;
}

function hide() { cur = null; $("tooltip").classList.add("hidden"); }

export function initTooltip() {
  // mouseover 委托:进入带 data-tip 的元素立即显示;移到无提示区域即收起
  document.addEventListener("mouseover", (e) => {
    if (!(e.target instanceof Element)) return;
    const el = e.target.closest("[data-tip]");
    if (el === cur) return; // 同一目标内部子元素间移动(svg 等),保持不动
    if (el) show(el); else hide();
  });
  // 指针直接移出窗口(relatedTarget 为空)时不会再触发 mouseover,补一次收起
  document.addEventListener("mouseout", (e) => {
    if (cur && !(e.relatedTarget instanceof Element)) hide();
  });
  // 点击即收起:打开弹窗(如设置)时提示不残留;切换类按钮重新悬停可见新文案
  document.addEventListener("click", hide);
  // fixed 定位不随内容滚动,任何容器滚动即收起(与右键菜单同策略)
  window.addEventListener("scroll", hide, true);
}

// 元素的 data-tip 更新后调用:仅当该元素提示正在显示时就地重读文案与位置
// (如「只看待提交」开关、置顶切换后提示文案随状态变化)
export function refreshTip(el) {
  if (el === cur) show(el);
}
