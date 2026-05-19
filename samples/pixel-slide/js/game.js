// Pixel Slide - tile sliding, slide animation, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-slide-save';
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

const SLIDE_DUR = 0.1;
let state = null;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const lv = LEVELS[index];
  const board = scramble(lv.n, lv.seed);
  const cell = Math.floor(Math.min(310, 300) / lv.n);
  const gridPx = cell * lv.n;
  state = {
    index, lv, n: lv.n, board, cell,
    gx: Math.round((VW - gridPx) / 2),
    gy: Math.round(96 + (308 - gridPx) / 2),
    empty: board.indexOf(0),
    moves: 0, won: false, sliding: null,
  };
  updateHud();
}

function cellPos(idx) {
  return {
    x: state.gx + (idx % state.n) * state.cell,
    y: state.gy + ((idx / state.n) | 0) * state.cell,
  };
}

// ---- sliding -------------------------------------------------------------
function doSlide(idx) {
  if (state.won || state.sliding) return;
  if (!neighbors(state.empty, state.n).includes(idx)) return;
  const value = state.board[idx];
  const from = idx, to = state.empty;
  state.board[to] = value;
  state.board[idx] = 0;
  state.empty = idx;
  state.sliding = { value, toIdx: to, fromIdx: from, t: SLIDE_DUR };
  state.moves++;
  updateHud();
  if (isSolved(state.board)) winLevel();
}

function winLevel() {
  state.won = true;
  const par = state.lv.par;
  const stars = state.moves <= par ? 3 : state.moves <= Math.round(par * 1.7) ? 2 : 1;
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
  }, 500);
}

// ---- update / render -----------------------------------------------------
function update(dt) {
  if (state && state.sliding) {
    state.sliding.t -= dt;
    if (state.sliding.t <= 0) state.sliding = null;
  }
}

function render() {
  drawBackground(ctx);
  if (!state) return;
  drawBoardFrame(ctx, state.gx, state.gy, state.n, state.cell);
  // empty socket
  const ep = cellPos(state.empty);
  ctx.fillStyle = '#0a0c14';
  ctx.fillRect(ep.x + 1.5, ep.y + 1.5, state.cell - 3, state.cell - 3);
  for (let i = 0; i < state.board.length; i++) {
    const v = state.board[i];
    if (v === 0) continue;
    let pos = cellPos(i);
    if (state.sliding && state.sliding.toIdx === i) {
      const k = state.sliding.t / SLIDE_DUR;
      const fp = cellPos(state.sliding.fromIdx);
      pos = { x: pos.x + (fp.x - pos.x) * k, y: pos.y + (fp.y - pos.y) * k };
    }
    drawTile(ctx, pos.x, pos.y, state.cell, v, state.n);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-moves').textContent = 'MOVES ' + state.moves;
  document.getElementById('hud-par').textContent = 'PAR ' + state.lv.par;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const c = Math.floor((px - state.gx) / state.cell);
  const r = Math.floor((py - state.gy) / state.cell);
  if (c < 0 || r < 0 || c >= state.n || r >= state.n) return;
  doSlide(r * state.n + c);
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
  LEVELS.forEach((lv, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) +
      '</span><em>' + lv.n + '×' + lv.n + '</em>';
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
document.getElementById('btn-restart').onclick = () => startLevel(state.index);
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
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
