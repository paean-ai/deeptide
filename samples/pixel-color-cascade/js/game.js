// Pixel Color Cascade - flood-fill puzzle: board, greedy par, ripple, scoring.

const SAVE_KEY = 'pixel-color-cascade-save';
const RING_DELAY = 0.028;     // seconds of ripple delay per BFS ring

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let state = null;
let progress = loadProgress();
let lastT = performance.now();

// ---- save --------------------------------------------------------------
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number')
      return { unlocked: d.unlocked, stars: d.stars || {}, bestMoves: d.bestMoves || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, stars: {}, bestMoves: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}

// ---- flood-fill core ---------------------------------------------------
// BFS the origin-connected region of a plain grid; returns [x,y] cells.
function regionPlain(g, n) {
  const col = g[0][0];
  const seen = new Uint8Array(n * n);
  const list = [[0, 0]];
  const q = [0];
  seen[0] = 1;
  while (q.length) {
    const idx = q.shift();
    const x = idx % n, y = (idx / n) | 0;
    const nb = [[x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]];
    for (const [nx, ny] of nb) {
      if (nx < 0 || ny < 0 || nx >= n || ny >= n) continue;
      const ni = ny * n + nx;
      if (seen[ni] || g[ny][nx] !== col) continue;
      seen[ni] = 1; q.push(ni); list.push([nx, ny]);
    }
  }
  return list;
}
function isUniform(g, n) {
  const c = g[0][0];
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) if (g[y][x] !== c) return false;
  return true;
}
// Greedy solver: each move picks the colour that grows the region most.
function greedySolve(g0, n, k) {
  const g = g0.map(r => r.slice());
  let moves = 0, guard = 0;
  while (!isUniform(g, n) && guard < 600) {
    guard++;
    const cur = g[0][0];
    const terr = regionPlain(g, n);
    let bestC = cur, bestSize = -1;
    for (let c = 0; c < k; c++) {
      if (c === cur) continue;
      const t2 = g.map(r => r.slice());
      for (const [x, y] of terr) t2[y][x] = c;
      const sz = regionPlain(t2, n).length;
      if (sz > bestSize) { bestSize = sz; bestC = c; }
    }
    for (const [x, y] of terr) g[y][x] = bestC;
    moves++;
  }
  return moves;
}

// BFS the origin region of the live `cells` grid, returning [x,y,dist].
function regionCells(cells, n) {
  const col = cells[0].c;
  const seen = new Uint8Array(n * n);
  const list = [[0, 0, 0]];
  const q = [0];
  seen[0] = 1;
  while (q.length) {
    const idx = q.shift();
    const x = idx % n, y = (idx / n) | 0;
    const d = list.find(e => e[0] === x && e[1] === y)[2];
    const nb = [[x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]];
    for (const [nx, ny] of nb) {
      if (nx < 0 || ny < 0 || nx >= n || ny >= n) continue;
      const ni = ny * n + nx;
      if (seen[ni] || cells[ni].c !== col) continue;
      seen[ni] = 1; q.push(ni); list.push([nx, ny, d + 1]);
    }
  }
  return list;
}

// ---- level lifecycle ---------------------------------------------------
function loadLevel(index) {
  const level = LEVELS[index];
  const n = level.n;
  const board = buildBoard(level);
  const par = greedySolve(board, n, level.k);
  const limit = par + levelSlack(index);

  const cells = [];
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++)
      cells.push({ c: board[y][x], prev: board[y][x], flipAt: -1 });

  const cell = Math.floor(324 / n);
  const bs = cell * n;
  const sw = Math.min(48, Math.floor((VW - 36) / level.k) - 8);
  const gap = Math.floor((VW - 36 - sw * level.k) / Math.max(1, level.k - 1));
  const startX = Math.round((VW - (sw * level.k + gap * (level.k - 1))) / 2);
  const swatches = [];
  for (let i = 0; i < level.k; i++)
    swatches.push({ x: startX + i * (sw + gap), y: 392, w: sw });

  state = {
    index, level, n, k: level.k, cells, par, limit,
    moves: 0, clock: 0, waveEnd: 0, busy: false, won: false, lost: false,
    layout: { bx: Math.round((VW - bs) / 2), by: 22, bs, cell },
    swatches,
  };
  updateHud();
}

function flood(colorIdx) {
  const s = state;
  if (!s || s.busy || s.won || s.lost) return;
  const cur = s.cells[0].c;
  if (colorIdx === cur) return;
  const region = regionCells(s.cells, s.n);
  let maxD = 0;
  for (const [x, y, d] of region) {
    const cell = s.cells[y * s.n + x];
    cell.prev = cell.c;
    cell.c = colorIdx;
    cell.flipAt = s.clock + d * RING_DELAY;
    if (d > maxD) maxD = d;
  }
  s.moves++;
  s.waveEnd = s.clock + maxD * RING_DELAY + 0.2;
  s.busy = true;
  updateHud();
}

function resolve() {
  const s = state;
  const grid = [];
  for (let y = 0; y < s.n; y++) {
    const row = [];
    for (let x = 0; x < s.n; x++) row.push(s.cells[y * s.n + x].c);
    grid.push(row);
  }
  if (isUniform(grid, s.n)) winLevel();
  else if (s.moves >= s.limit) failLevel();
}

function starsFor(moves, par, index) {
  if (moves <= par) return 3;
  if (moves <= par + Math.ceil(levelSlack(index) / 2)) return 2;
  return 1;
}

function winLevel() {
  const s = state;
  s.won = true;
  const stars = starsFor(s.moves, s.par, s.index);
  const prevStars = progress.stars[s.index] || 0;
  if (stars > prevStars) progress.stars[s.index] = stars;
  const prevBest = progress.bestMoves[s.index];
  if (prevBest == null || s.moves < prevBest) progress.bestMoves[s.index] = s.moves;
  progress.unlocked = Math.max(progress.unlocked, Math.min(LEVEL_COUNT, s.index + 2));
  saveProgress();

  document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
  document.getElementById('win-stars').textContent =
    '★'.repeat(stars) + '☆'.repeat(3 - stars);
  document.getElementById('win-line').textContent = t('starLine', s.moves, s.par);
  document.getElementById('btn-next').style.display =
    s.index + 1 < LEVEL_COUNT ? '' : 'none';
  showOverlay('overlay-win');
}

function failLevel() {
  state.lost = true;
  showOverlay('overlay-fail');
}

// ---- update / render ---------------------------------------------------
function update(dt) {
  const s = state;
  s.clock += dt;
  if (s.busy && s.clock >= s.waveEnd) {
    s.busy = false;
    resolve();
  }
}
function render() {
  const s = state;
  drawBackground(ctx);
  drawGrid(ctx, s.layout, s.cells, s.n, s.clock);
  drawSwatches(ctx, s.swatches, s.cells[0].c);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = `${t('level')} ${state.index + 1}`;
  document.getElementById('hud-moves').textContent =
    `${t('moves')} ${state.moves}/${state.limit}`;
  document.getElementById('hud-par').textContent = `${t('par')} ${state.par}`;
}

// ---- level select ------------------------------------------------------
function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const unlocked = i < progress.unlocked;
    const btn = document.createElement('button');
    btn.className = 'level-cell' + (unlocked ? '' : ' locked');
    const st = progress.stars[i] || 0;
    btn.innerHTML = `<b>${i + 1}</b><span>${unlocked
      ? (st ? '★'.repeat(st) : '·')
      : '🔒'}</span>`;
    btn.disabled = !unlocked;
    btn.onclick = () => { loadLevel(i); showScreen('screen-game'); };
    grid.appendChild(btn);
  }
}

// ---- screens / overlays ------------------------------------------------
function showScreen(id) {
  hideAllOverlays();
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() {
  document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
}
function overlaysClosed() {
  return document.querySelectorAll('.overlay:not(.hidden)').length === 0;
}

// ---- input -------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || !overlaysClosed() ||
      document.getElementById('screen-game').classList.contains('hidden')) return;
  e.preventDefault();
  const r = canvas.getBoundingClientRect();
  const px = (e.clientX - r.left) / r.width * VW;
  const py = (e.clientY - r.top) / r.height * VH;
  // swatch?
  for (let i = 0; i < state.swatches.length; i++) {
    const s = state.swatches[i];
    if (px >= s.x && px <= s.x + s.w && py >= s.y && py <= s.y + s.w) {
      flood(i);
      return;
    }
  }
  // grid tile?
  const L = state.layout;
  if (px >= L.bx && px < L.bx + L.bs && py >= L.by && py < L.by + L.bs) {
    const cx = Math.floor((px - L.bx) / L.cell);
    const cy = Math.floor((py - L.by) / L.cell);
    if (cx >= 0 && cy >= 0 && cx < state.n && cy < state.n) {
      flood(state.cells[cy * state.n + cx].c);
    }
  }
});

document.getElementById('btn-play').onclick = () => {
  loadLevel(Math.min(progress.unlocked - 1, LEVEL_COUNT - 1));
  showScreen('screen-game');
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-retry').onclick = () => { loadLevel(state.index); showScreen('screen-game'); };
document.getElementById('btn-fail-retry').onclick = () => { loadLevel(state.index); showScreen('screen-game'); };
document.getElementById('btn-fail-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-next').onclick = () => {
  loadLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
  showScreen('screen-game');
};
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
setupLanguageToggle(() => { if (state) updateHud(); buildLevelGrid(); });

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  if (state && !document.getElementById('screen-game').classList.contains('hidden')) {
    if (overlaysClosed()) update(dt);
    render();
  }
  requestAnimationFrame(loop);
}

showScreen('screen-title');
requestAnimationFrame(loop);
