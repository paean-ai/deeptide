// Pixel Unblock - block dragging, win detection, progress save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-unblock-save';
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

const CELL = 50, GX = Math.round((VW - CELL * GRID_N) / 2), GY = 110;
let state = null;
let drag = null;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const level = LEVELS[index];
  state = {
    index, level,
    pos: startPos(level),
    par: solveLevel(level),
    moves: 0, history: [], won: false,
  };
  drag = null;
  updateHud();
}

function blockBox(i) {
  const b = state.level.blocks[i], p = state.pos[i];
  return {
    x: b.horiz ? p : b.x,
    y: b.horiz ? b.y : p,
    w: b.horiz ? b.len : 1,
    h: b.horiz ? 1 : b.len,
  };
}
function blockAtCell(cx, cy) {
  for (let i = 0; i < state.level.blocks.length; i++) {
    const r = blockBox(i);
    if (cx >= r.x && cx < r.x + r.w && cy >= r.y && cy < r.y + r.h) return i;
  }
  return -1;
}

// How far block i can slide along its lane, given the others.
function reachableRange(i) {
  const b = state.level.blocks[i];
  const g = Array.from({ length: GRID_N }, () => new Array(GRID_N).fill(false));
  for (let j = 0; j < state.level.blocks.length; j++) {
    if (j === i) continue;
    const r = blockBox(j);
    for (let dy = 0; dy < r.h; dy++) {
      for (let dx = 0; dx < r.w; dx++) g[r.y + dy][r.x + dx] = true;
    }
  }
  let lo = state.pos[i], hi = state.pos[i];
  const maxA = GRID_N - b.len;
  while (lo - 1 >= 0) {
    const ex = b.horiz ? lo - 1 : b.x, ey = b.horiz ? b.y : lo - 1;
    if (g[ey][ex]) break;
    lo--;
  }
  while (hi + 1 <= maxA) {
    const ex = b.horiz ? hi + 1 + b.len - 1 : b.x;
    const ey = b.horiz ? b.y : hi + 1 + b.len - 1;
    if (g[ey][ex]) break;
    hi++;
  }
  return [lo, hi];
}

// ---- win -----------------------------------------------------------------
function checkWin() {
  if (!isWin(state.level, state.pos)) return;
  state.won = true;
  const par = state.par;
  const stars = state.moves <= par ? 3
    : state.moves <= par + Math.ceil(par * 0.6) + 1 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', state.moves, par);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 480);
}

function undo() {
  if (!state || state.won || !state.history.length) return;
  const h = state.history.pop();
  state.pos = h.pos;
  state.moves = h.moves;
  drag = null;
  updateHud();
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  drawBoard(ctx, GX, GY, CELL, state.level.exitRow);
  for (let i = 0; i < state.level.blocks.length; i++) {
    const r = blockBox(i);
    const x = GX + r.x * CELL, y = GY + r.y * CELL;
    const dragging = drag && drag.idx === i;
    if (i === state.level.targetIdx) drawTarget(ctx, x, y, r.w * CELL, r.h * CELL, dragging);
    else drawBlock(ctx, x, y, r.w * CELL, r.h * CELL,
      BLOCK_COLORS[i % BLOCK_COLORS.length], dragging);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-moves').textContent = 'MOVES ' + state.moves;
  document.getElementById('hud-par').textContent = 'PAR ' + state.par;
}

// ---- input ---------------------------------------------------------------
function pointerCanvas(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const p = pointerCanvas(e);
  const cx = Math.floor((p.x - GX) / CELL), cy = Math.floor((p.y - GY) / CELL);
  if (cx < 0 || cy < 0 || cx >= GRID_N || cy >= GRID_N) return;
  const i = blockAtCell(cx, cy);
  if (i < 0) return;
  const b = state.level.blocks[i];
  drag = {
    idx: i, startAnchor: state.pos[i],
    startPixel: b.horiz ? p.x : p.y,
    range: reachableRange(i),
    before: state.pos.slice(),
  };
});
canvas.addEventListener('pointermove', e => {
  if (!drag || !state) return;
  const p = pointerCanvas(e);
  const b = state.level.blocks[drag.idx];
  const cur = b.horiz ? p.x : p.y;
  const d = Math.round((cur - drag.startPixel) / CELL);
  let np = drag.startAnchor + d;
  np = Math.max(drag.range[0], Math.min(drag.range[1], np));
  state.pos[drag.idx] = np;
});
function endDrag() {
  if (!drag || !state) return;
  if (state.pos[drag.idx] !== drag.startAnchor) {
    state.history.push({ pos: drag.before, moves: state.moves });
    if (state.history.length > 200) state.history.shift();
    state.moves++;
    updateHud();
    checkWin();
  }
  drag = null;
}
canvas.addEventListener('pointerup', endDrag);
canvas.addEventListener('pointercancel', endDrag);

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
document.getElementById('btn-undo').onclick = undo;
document.getElementById('btn-restart').onclick = () => startLevel(state.index);
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
