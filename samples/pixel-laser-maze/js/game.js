// Pixel Laser Maze - mirror placement, beam rendering, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-laser-maze-save';
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
  const L = LEVELS[index];
  const cell = Math.min(40, Math.floor(Math.min(332 / L.cols, 300 / L.rows)));
  state = {
    index, L, cell,
    gx: Math.round((VW - L.cols * cell) / 2),
    gy: Math.round(98 + (302 - L.rows * cell) / 2),
    mirrors: new Map(),
    emptySet: new Set(L.empties),
    taps: 0, won: false, winStars: 0,
  };
  updateHud();
}

function cellCenter(x, y) {
  return { x: state.gx + x * state.cell + state.cell / 2,
           y: state.gy + y * state.cell + state.cell / 2 };
}

// ---- mirror placement ----------------------------------------------------
function tapCell(x, y) {
  if (state.won) return;
  const key = x + ',' + y;
  if (!state.emptySet.has(key)) return;
  const cur = state.mirrors.get(key);
  if (cur === undefined) {
    if (state.mirrors.size >= state.L.mirrors) return;
    state.mirrors.set(key, '/');
  } else if (cur === '/') {
    state.mirrors.set(key, '\\');
  } else {
    state.mirrors.delete(key);
  }
  state.taps++;
  updateHud();
  if (beamWins(state.L, state.mirrors)) winLevel();
}

function winLevel() {
  state.won = true;
  const par = state.L.mirrors;
  const stars = state.taps <= par + 2 ? 3 : state.taps <= par + 6 ? 2 : 1;
  state.winStars = stars;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', state.mirrors.size);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 600);
}

// ---- beam path (for rendering) -------------------------------------------
function beamPath() {
  const L = state.L;
  let x = L.emitter[0], y = L.emitter[1];
  let dx = L.edir[0], dy = L.edir[1];
  const pts = [cellCenter(x, y)];
  const cap = L.cols * L.rows * 4;
  for (let s = 0; s < cap; s++) {
    x += dx; y += dy;
    if (x < 0 || y < 0 || x >= L.cols || y >= L.rows) { pts.push(cellCenter(x, y)); break; }
    if (L.walls.has(x + ',' + y)) break;
    pts.push(cellCenter(x, y));
    const m = state.mirrors.get(x + ',' + y);
    if (m === '/') { const t = -dy; dy = -dx; dx = t; }
    else if (m === '\\') { const t = dy; dy = dx; dx = t; }
  }
  return pts;
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  const L = state.L, cell = state.cell;
  for (let y = 0; y < L.rows; y++) {
    for (let x = 0; x < L.cols; x++) {
      drawCell(ctx, state.gx + x * cell, state.gy + y * cell, cell);
    }
  }
  for (const w of L.walls) {
    const [x, y] = w.split(',').map(Number);
    drawWall(ctx, state.gx + x * cell, state.gy + y * cell, cell);
  }
  drawBeam(ctx, beamPath());
  drawEmitter(ctx, state.gx + L.emitter[0] * cell, state.gy + L.emitter[1] * cell, cell, L.edir);
  const lit = traceBeam(L, state.mirrors);
  for (const tg of L.targets) {
    drawTarget(ctx, state.gx + tg[0] * cell, state.gy + tg[1] * cell, cell,
      lit.has(tg[0] + ',' + tg[1]));
  }
  for (const [key, type] of state.mirrors) {
    const [x, y] = key.split(',').map(Number);
    drawMirror(ctx, state.gx + x * cell, state.gy + y * cell, cell, type);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-mirrors').textContent =
    t('mirrors') + ' ' + state.mirrors.size + '/' + state.L.mirrors;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const x = Math.floor((px - state.gx) / state.cell);
  const y = Math.floor((py - state.gy) / state.cell);
  if (x >= 0 && y >= 0 && x < state.L.cols && y < state.L.rows) tapCell(x, y);
});

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
  LEVELS.forEach((L, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
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
  for (let i = 0; i < LEVEL_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startLevel(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-reset').onclick = () => startLevel(state.index);
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
