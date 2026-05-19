// Pixel Word Hunt - drag-select word finding, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-word-hunt-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, stars: d.stars || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, stars: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

let state = null;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const puzzle = PUZZLES[index];
  const gen = genGrid(puzzle);
  const n = puzzle.n;
  const cell = Math.min(36, Math.floor(286 / n));
  const gridPx = cell * n;
  state = {
    index, puzzle, n, grid: gen.grid, placements: gen.placements,
    cell, gx: Math.round((VW - gridPx) / 2), gy: 56,
    found: new Set(), foundCells: new Map(),
    selStart: null, selEnd: null,
    time: 0, hints: 0, won: false, running: false, hint: null,
  };
  state.listY = state.gy + gridPx + 12;
  updateHud();
}

function wordColor(i) { return WH_HUES[i % WH_HUES.length]; }

// the straight cell line from a to b, or null if not on an 8-direction line.
function lineCells(a, b) {
  const dr = b.r - a.r, dc = b.c - a.c;
  if (!(dr === 0 || dc === 0 || Math.abs(dr) === Math.abs(dc))) return null;
  const len = Math.max(Math.abs(dr), Math.abs(dc)) + 1;
  const sr = Math.sign(dr), sc = Math.sign(dc);
  const cells = [];
  for (let i = 0; i < len; i++) cells.push([a.r + sr * i, a.c + sc * i]);
  return cells;
}
function cellsEqual(a, b) {
  return a.length === b.length && a.every((c, i) => c[0] === b[i][0] && c[1] === b[i][1]);
}

function evaluateSelection() {
  if (!state.selStart || !state.selEnd) return;
  const cells = lineCells(state.selStart, state.selEnd);
  if (!cells || cells.length < 3) return;
  const rev = cells.slice().reverse();
  for (const w of state.puzzle.words) {
    if (state.found.has(w)) continue;
    const pc = state.placements[w];
    if (cellsEqual(cells, pc) || cellsEqual(rev, pc)) {
      state.found.add(w);
      const color = wordColor(state.puzzle.words.indexOf(w));
      for (const [r, c] of pc) state.foundCells.set(r + ',' + c, color);
      updateHud();
      if (state.found.size === state.puzzle.words.length) winLevel();
      return;
    }
  }
}

function winLevel() {
  state.won = true;
  state.running = false;
  const stars = state.hints === 0 ? 3 : state.hints <= 2 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < PUZZLE_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', fmtTime(state.time), state.hints);
    document.getElementById('btn-next').style.display = i + 1 < PUZZLE_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 600);
}

function useHint() {
  if (!state || state.won) return;
  const left = state.puzzle.words.filter(w => !state.found.has(w));
  if (!left.length) return;
  state.hint = { word: left[0], t: 2.0 };
  state.hints++;
  if (!state.running) state.running = true;
}

// ---- update / render -----------------------------------------------------
function update(dt) {
  if (!state) return;
  if (state.running && !state.won) { state.time += dt; updateHud(); }
  if (state.hint) { state.hint.t -= dt; if (state.hint.t <= 0) state.hint = null; }
}

function render() {
  drawBackground(ctx);
  if (!state) return;
  const { n, cell, gx, gy } = state;
  const selCells = state.selStart && state.selEnd ? lineCells(state.selStart, state.selEnd) : null;
  const selSet = new Set((selCells || []).map(c => c[0] + ',' + c[1]));
  const hintSet = new Set();
  if (state.hint) for (const [r, c] of state.placements[state.hint.word]) hintSet.add(r + ',' + c);

  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const key = r + ',' + c;
      let bg = '#1c1810', fg = '#d8cfb6';
      if (state.foundCells.has(key)) { bg = state.foundCells.get(key); fg = '#10100a'; }
      if (selSet.has(key)) { bg = '#f2cf3f'; fg = '#10100a'; }
      if (hintSet.has(key) && (state.hint.t * 6 | 0) % 2 === 0) { bg = '#5fc06e'; fg = '#10100a'; }
      drawLetter(ctx, gx + c * cell, gy + r * cell, cell, state.grid[r][c], bg, fg);
    }
  }
  // word list
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const words = state.puzzle.words, cols = 3;
  const colW = 112, x0 = (VW - cols * colW) / 2 + colW / 2;
  for (let i = 0; i < words.length; i++) {
    const w = words[i];
    const col = i % cols, row = (i / cols) | 0;
    const wx = x0 + col * colW, wy = state.listY + 10 + row * 18;
    const done = state.found.has(w);
    ctx.fillStyle = done ? wordColor(i) : '#b5ac95';
    ctx.font = (done ? '900 ' : '') + '11px ui-monospace, monospace';
    ctx.fillText(w, wx, wy);
    if (done) {
      ctx.strokeStyle = wordColor(i);
      ctx.lineWidth = 1.5;
      const tw = ctx.measureText(w).width;
      ctx.beginPath();
      ctx.moveTo(wx - tw / 2, wy);
      ctx.lineTo(wx + tw / 2, wy);
      ctx.stroke();
    }
  }
  ctx.textAlign = 'left';
}

// ---- HUD -----------------------------------------------------------------
function fmtTime(s) {
  const m = Math.floor(s / 60), sec = Math.floor(s % 60);
  return m + ':' + String(sec).padStart(2, '0');
}
function updateHud() {
  if (!state) return;
  document.getElementById('hud-theme').textContent =
    state.puzzle.theme[currentLang === 'zh' ? 1 : 0].toUpperCase();
  document.getElementById('hud-found').textContent =
    state.found.size + '/' + state.puzzle.words.length;
  document.getElementById('hud-time').textContent = fmtTime(state.time);
}

// ---- input ---------------------------------------------------------------
function cellAt(px, py) {
  const c = Math.floor((px - state.gx) / state.cell);
  const r = Math.floor((py - state.gy) / state.cell);
  if (r < 0 || c < 0 || r >= state.n || c >= state.n) return null;
  return { r, c };
}
function pointer(e) {
  const rect = canvas.getBoundingClientRect();
  return cellAt((e.clientX - rect.left) * VW / rect.width,
                (e.clientY - rect.top) * VH / rect.height);
}
function gameActive() {
  return state && !state.won &&
    !document.getElementById('screen-game').classList.contains('hidden');
}
canvas.addEventListener('pointerdown', e => {
  if (!gameActive()) return;
  const cell = pointer(e);
  if (!cell) return;
  state.selStart = cell;
  state.selEnd = cell;
  if (!state.running) state.running = true;
});
canvas.addEventListener('pointermove', e => {
  if (!gameActive() || !state.selStart) return;
  const cell = pointer(e);
  if (cell) state.selEnd = cell;
});
function endSelect() {
  if (state && state.selStart) {
    evaluateSelection();
    state.selStart = null;
    state.selEnd = null;
  }
}
canvas.addEventListener('pointerup', endSelect);
canvas.addEventListener('pointercancel', endSelect);

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  hideAllOverlays();
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() {
  document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
}
function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  PUZZLES.forEach((pz, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    btn.innerHTML = '<b>' + pz.theme[currentLang === 'zh' ? 1 : 0] + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) + '</span>';
    if (!locked) btn.onclick = () => startLevel(i);
    grid.appendChild(btn);
  });
}
function startLevel(index) {
  buildLevel(index);
  showScreen('screen-game');
}

document.getElementById('btn-play').onclick = () => {
  let next = 0;
  for (let i = 0; i < PUZZLE_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startLevel(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-hint').onclick = useHint;
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, PUZZLE_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

// ---- main loop -----------------------------------------------------------
let last = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  if (!document.getElementById('screen-game').classList.contains('hidden')) {
    update(dt);
    render();
  } else {
    drawBackground(ctx);
  }
  requestAnimationFrame(loop);
}
showScreen('screen-title');
requestAnimationFrame(loop);
