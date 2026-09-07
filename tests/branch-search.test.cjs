const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { JSDOM } = require('jsdom');
const source = fs.readFileSync('src/js/git-ops.js', 'utf8');
const section = source.slice(source.indexOf('let branchCache'), source.indexOf('async function switchBranch')).replace('export function', 'function');
function setup(invoke = async () => data) {
  const dom = new JSDOM('<button id="btn-branch"></button><div id="branch-menu" class="hidden"></div>');
  const selected = [];
  const ctx = vm.createContext({ document: dom.window.document, repo: 'repo-a', invoke, $: id => dom.window.document.getElementById(id), switchBranch: name => selected.push(name) });
  vm.runInContext(section, ctx);
  return { ctx, selected, document: dom.window.document, window: dom.window,
    open: () => vm.runInContext('openBranchMenu()', ctx),
    names: () => [...dom.window.document.querySelectorAll('.branch-item')].map(x => x.title),
    input(value) { const input = dom.window.document.querySelector('input'); input.value = value; input.dispatchEvent(new dom.window.Event('input')); return input; },
    key(el, key, isComposing = false) { el.dispatchEvent(new dom.window.KeyboardEvent('keydown', { key, bubbles: true, isComposing })); }
  };
}
const data = { current: 'main', locals: ['main', 'Feature/Search', '修复/搜索'], remotes: ['origin/main', 'origin/Feature/Search', 'upstream/release'] };
test('filters both groups, trims query, supports Unicode and restores all branches', async () => {
  const s = setup(); await s.open();
  assert.equal(s.document.activeElement.type, 'search');
  s.input('  FEATURE/sea  '); assert.deepEqual(s.names(), ['Feature/Search', 'origin/Feature/Search']);
  s.input('搜索'); assert.deepEqual(s.names(), ['修复/搜索']);
  s.input('missing'); assert.equal(s.names().length, 0); assert.match(s.document.body.textContent, /没有匹配/);
  s.input(''); assert.equal(s.names().length, 6);
  assert.equal(s.document.querySelector('.current').title, 'main');
});
test('keyboard navigation, full remote name, IME and Escape', async () => {
  const s = setup(); await s.open();
  const input = s.input('upstream');
  s.key(input, 'Enter', true); assert.equal(s.selected.length, 0);
  s.key(input, 'Enter'); assert.deepEqual(s.selected, ['upstream/release']);
  s.input(''); s.key(input, 'ArrowUp'); assert.equal(s.document.activeElement.title, 'upstream/release');
  s.key(s.document.activeElement, 'ArrowDown'); assert.equal(s.document.activeElement.title, 'main');
  s.key(input, 'Escape'); assert.ok(s.document.getElementById('branch-menu').classList.contains('hidden'));
  assert.equal(s.document.activeElement.id, 'btn-branch');
});
test('uses query typed during loading and reuses cache with a fresh query', async () => {
  let resolve; let calls = 0;
  const s = setup(() => { calls++; return new Promise(r => resolve = r); });
  const pending = s.open(); s.input('release'); resolve(data); await pending;
  assert.deepEqual(s.names(), ['upstream/release']);
  await s.open(); assert.equal(calls, 1); assert.equal(s.names().length, 6);
});
test('ignores stale responses after reopening or changing repositories', async () => {
  const resolves = []; const s = setup(() => new Promise(r => resolves.push(r)));
  const first = s.open(); const second = s.open();
  resolves[1](data); await second;
  resolves[0]({ current: 'stale', locals: ['stale'], remotes: [] }); await first;
  assert.ok(!s.names().includes('stale'));
  vm.runInContext('repo = "repo-b"', s.ctx);
  const third = s.open(); vm.runInContext('repo = "repo-c"', s.ctx);
  resolves[2](data); await third; assert.equal(s.names().length, 0);
});
test('failure message and closed menu do not reopen', async () => {
  const s = setup(async () => { throw Error('offline'); }); await s.open();
  assert.match(s.document.body.textContent, /加载失败/);
  let resolve; const t = setup(() => new Promise(r => resolve = r));
  const pending = t.open(); t.key(t.document.querySelector('input'), 'Escape'); resolve(data); await pending;
  assert.ok(t.document.getElementById('branch-menu').classList.contains('hidden'));
});
