// Pixel Flow - pipe-drawing logic, win detection, progress save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-flow-save';
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
  const lv = LEVELS[index];
  const N = lv.size;
  const paths = genPuzzle(N, lv.seed);
  const colors = paths.map(p => ({
    epA: { r: p[0].r, c: p[0].c },
    epB: { r: p[p.length - 1].r, c: p[p.length - 1].c },
    path: [{ r: p[0].r, c: p[0].c }],
  }));
  const cell = Math.floor(Math.min(330, 320) / N);
  const px = cell * N;
  state = {
    index, N, cell, colors,
    gx: Math.round((VW - px) / 2),
    gy: Math.round(72 + (336 - px) / 2),
    owner: [], active: -1, won: false,
  };
  rebuildOwner();
  updateHud();
}

function rebuildOwner() {
  const N = state.N;
  state.owner = Array.from({ length: N }, () => new Array(N).fill(-1));
  state.colors.forEach((col, ci) => {
    state.owner[col.epA.r][col.epA.c] = ci;
    state.owner[col.epB.r][col.epB.c] = ci;
    for (const p of col.path) state.owner[p.r][p.c] = ci;
  });
}

// ---- rule helpers --------------------------------------------------------
function eq(a, b) { return a.r === b.r && a.c === b.c; }
function adjacent(a, b) { return Math.abs(a.r - b.r) + Math.abs(a.c - b.c) === 1; }
function isEndpoint(col, r, c) {
  return (col.epA.r === r && col.epA.c === c) || (col.epB.r === r && col.epB.c === c);
}
function otherEnd(col) {
  return eq(col.path[0], col.epA) ? col.epB : col.epA;
}
function isComplete(col) {
  return col.path.length >= 2 && eq(col.path[col.path.length - 1], otherEnd(col));
}
function pathIndex(col, r, c) {
  return col.path.findIndex(p => p.r === r && p.c === c);
}

// ---- drawing -------------------------------------------------------------
function grab(r, c) {
  const X = state.owner[r][c];
  if (X < 0) return;
  const col = state.colors[X];
  const idx = pathIndex(col, r, c);
  if (idx >= 0) col.path.length = idx + 1;       // grabbed mid-path: retract to here
  else col.path = [{ r, c }];                    // grabbed the other endpoint: restart
  state.active = X;
  rebuildOwner();
}

function extend(r, c) {
  if (state.active < 0) return;
  const col = state.colors[state.active];
  const head = col.path[col.path.length - 1];
  if (head.r === r && head.c === c) return;
  const idx = pathIndex(col, r, c);
  if (idx >= 0) {                                // dragged back onto own pipe: retract
    col.path.length = idx + 1;
    rebuildOwner();
    checkWin();
    return;
  }
  if (!adjacent(head, { r, c })) return;         // forward steps must be one cell
  if (isComplete(col)) return;                   // finished pipe: retract only
  const o = state.owner[r][c];
  if (o >= 0 && o !== state.active) {
    if (isEndpoint(state.colors[o], r, c)) return;   // can't cross another dot
    const op = state.colors[o].path;                 // cut the crossed pipe
    const oi = op.findIndex(p => p.r === r && p.c === c);
    if (oi >= 0) op.length = oi;
  }
  col.path.push({ r, c });
  rebuildOwner();
  checkWin();
}

function resetLevel() {
  if (state.won) return;
  for (const col of state.colors) col.path = [{ r: col.epA.r, c: col.epA.c }];
  state.active = -1;
  rebuildOwner();
  updateHud();
}

function checkWin() {
  updateHud();
  if (state.won) return;
  if (!state.colors.every(isComplete)) return;
  state.won = true;
  state.active = -1;
  let owned = 0;
  for (let r = 0; r < state.N; r++) {
    for (let c = 0; c < state.N; c++) if (state.owner[r][c] >= 0) owned++;
  }
  const pct = Math.round(owned / (state.N * state.N) * 100);
  const stars = pct === 100 ? 3 : pct >= 85 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = pct === 100 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', pct);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 550);
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  const done = state.colors.filter(isComplete).length;
  let owned = 0;
  for (let r = 0; r < state.N; r++) {
    for (let c = 0; c < state.N; c++) if (state.owner[r][c] >= 0) owned++;
  }
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-pairs').textContent =
    t('pairs') + ' ' + done + '/' + state.colors.length;
  document.getElementById('hud-fill').textContent =
    Math.round(owned / (state.N * state.N) * 100) + '%';
}

// ---- render --------------------------------------------------------------
function center(r, c) {
  return {
    x: state.gx + c * state.cell + state.cell / 2,
    y: state.gy + r * state.cell + state.cell / 2,
  };
}
function render() {
  drawBackground(ctx);
  if (!state) return;
  drawGrid(ctx, state.gx, state.gy, state.N, state.cell);
  state.colors.forEach((col, ci) => {
    if (ci === state.active) return;
    drawPipe(ctx, col.path.map(p => center(p.r, p.c)), COLORS[ci], state.cell, false);
  });
  if (state.active >= 0) {
    const col = state.colors[state.active];
    drawPipe(ctx, col.path.map(p => center(p.r, p.c)), COLORS[state.active], state.cell, true);
  }
  state.colors.forEach((col, ci) => {
    const done = isComplete(col);
    const a = center(col.epA.r, col.epA.c), b = center(col.epB.r, col.epB.c);
    drawEndpoint(ctx, a.x, a.y, COLORS[ci], state.cell, done);
    drawEndpoint(ctx, b.x, b.y, COLORS[ci], state.cell, done);
  });
}

// ---- input ---------------------------------------------------------------
function cellAt(clientX, clientY) {
  const rect = canvas.getBoundingClientRect();
  const x = (clientX - rect.left) * VW / rect.width;
  const y = (clientY - rect.top) * VH / rect.height;
  const c = Math.floor((x - state.gx) / state.cell);
  const r = Math.floor((y - state.gy) / state.cell);
  if (r < 0 || c < 0 || r >= state.N || c >= state.N) return null;
  return { r, c };
}
function gameActive() {
  return state && !state.won &&
    !document.getElementById('screen-game').classList.contains('hidden');
}
canvas.addEventListener('pointerdown', e => {
  if (!gameActive()) return;
  const cell = cellAt(e.clientX, e.clientY);
  if (cell) grab(cell.r, cell.c);
});
canvas.addEventListener('pointermove', e => {
  if (!gameActive() || state.active < 0) return;
  const cell = cellAt(e.clientX, e.clientY);
  if (cell) extend(cell.r, cell.c);
});
function endStroke() { if (state) state.active = -1; }
canvas.addEventListener('pointerup', endStroke);
canvas.addEventListener('pointercancel', endStroke);

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
  LEVELS.forEach((lv, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) +
      '</span><em>' + lv.size + '×' + lv.size + '</em>';
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
  for (let i = 0; i < LEVEL_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startLevel(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-reset').onclick = resetLevel;
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

// ---- main loop -----------------------------------------------------------
function loop() {
  if (!document.getElementById('screen-game').classList.contains('hidden')) render();
  else drawBackground(ctx);
  requestAnimationFrame(loop);
}
showScreen('screen-title');
requestAnimationFrame(loop);
