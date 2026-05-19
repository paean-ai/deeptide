// Pixel Mine Sweeper - grid logic, input, rendering and progress save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

// ---- persistent progress -------------------------------------------------
const SAVE_KEY = 'pixel-mine-sweeper-save';

function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') {
      return { unlocked: d.unlocked, stars: d.stars || {}, bestTime: d.bestTime || {} };
    }
  } catch (e) { /* ignore */ }
  return { unlocked: 1, stars: {}, bestTime: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

let state = null;
let flagMode = false;
const board = { tile: 0, x: 0, y: 0, w: 0, h: 0 };

// ---- level setup ---------------------------------------------------------
function layoutBoard() {
  const region = { x: 8, y: 52, w: VW - 16, h: 372 };
  const t = Math.floor(Math.min(region.w / state.cols, region.h / state.rows));
  board.tile = t;
  board.w = t * state.cols;
  board.h = t * state.rows;
  board.x = Math.round((VW - board.w) / 2);
  board.y = Math.round(region.y + (region.h - board.h) / 2);
}

function newGame(index) {
  const lv = LEVELS[index];
  const n = lv.cols * lv.rows;
  const cells = [];
  for (let i = 0; i < n; i++) {
    cells.push({ mine: false, revealed: false, flagged: false, adj: 0, pop: 0, flagPop: 0 });
  }
  state = {
    index, lv, cols: lv.cols, rows: lv.rows, cells, n,
    minesPlaced: false, running: false, status: 'play',
    flags: 0, revealed: 0, time: 0, scansLeft: 1, hitIndex: -1,
    flash: 0, sparkle: [],
  };
  layoutBoard();
  updateHud();
  updateControls();
}

function inBounds(c, r) { return c >= 0 && r >= 0 && c < state.cols && r < state.rows; }

function neighbors(idx) {
  const c = idx % state.cols, r = (idx / state.cols) | 0, out = [];
  for (let dy = -1; dy <= 1; dy++) {
    for (let dx = -1; dx <= 1; dx++) {
      if (!dx && !dy) continue;
      if (inBounds(c + dx, r + dy)) out.push((r + dy) * state.cols + (c + dx));
    }
  }
  return out;
}

// Mines are placed only after the first dig, keeping it (and its ring) safe.
function placeMines(safeIdx) {
  const safe = new Set([safeIdx, ...neighbors(safeIdx)]);
  const pool = [];
  for (let i = 0; i < state.n; i++) if (!safe.has(i)) pool.push(i);
  for (let i = pool.length - 1; i > 0; i--) {
    const j = (Math.random() * (i + 1)) | 0;
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  const count = Math.min(state.lv.mines, pool.length);
  for (let i = 0; i < count; i++) state.cells[pool[i]].mine = true;
  for (let i = 0; i < state.n; i++) {
    if (state.cells[i].mine) continue;
    state.cells[i].adj = neighbors(i).filter(j => state.cells[j].mine).length;
  }
  state.minesPlaced = true;
  state.running = true;
}

// ---- digging -------------------------------------------------------------
function floodFrom(startIdx) {
  const stack = [startIdx];
  while (stack.length) {
    const i = stack.pop();
    const c = state.cells[i];
    if (c.revealed || c.flagged || c.mine) continue;
    c.revealed = true;
    c.pop = 1;
    state.revealed++;
    if (c.adj === 0) {
      for (const j of neighbors(i)) {
        const nc = state.cells[j];
        if (!nc.revealed && !nc.flagged && !nc.mine) stack.push(j);
      }
    }
  }
}

function digCell(idx) {
  const c = state.cells[idx];
  if (c.revealed || c.flagged) return;
  if (!state.minesPlaced) placeMines(idx);
  if (c.mine) { loseGame(idx); return; }
  floodFrom(idx);
  checkWin();
}

// Tap a satisfied number to dig every un-flagged neighbour at once.
function chordCell(idx) {
  const c = state.cells[idx];
  if (!c.revealed || c.adj === 0) return;
  const nb = neighbors(idx);
  if (nb.filter(j => state.cells[j].flagged).length !== c.adj) return;
  for (const j of nb) {
    const nc = state.cells[j];
    if (nc.revealed || nc.flagged) continue;
    if (nc.mine) { loseGame(j); return; }
    floodFrom(j);
  }
  checkWin();
}

function toggleFlag(idx) {
  const c = state.cells[idx];
  if (c.revealed) return;
  c.flagged = !c.flagged;
  c.flagPop = c.flagged ? 1 : 0;
  state.flags += c.flagged ? 1 : -1;
  updateHud();
}

// One scan per level safely uncovers a random hidden mine-free tile.
function doScan() {
  if (!state || state.status !== 'play' || state.scansLeft <= 0) return;
  if (!state.minesPlaced) {
    state.scansLeft--;
    updateControls();
    digCell((Math.random() * state.n) | 0);
    return;
  }
  const pool = [];
  for (let i = 0; i < state.n; i++) {
    const c = state.cells[i];
    if (!c.mine && !c.revealed && !c.flagged) pool.push(i);
  }
  if (!pool.length) return;
  state.scansLeft--;
  updateControls();
  floodFrom(pool[(Math.random() * pool.length) | 0]);
  checkWin();
}

// ---- win / lose ----------------------------------------------------------
function loseGame(hitIdx) {
  state.status = 'lose';
  state.running = false;
  state.hitIndex = hitIdx;
  state.flash = 1;
  setTimeout(() => showOverlay('overlay-over'), 700);
}

function checkWin() {
  if (state.revealed !== state.n - state.lv.mines) return;
  state.status = 'win';
  state.running = false;
  for (const c of state.cells) {
    if (c.mine && !c.flagged) { c.flagged = true; c.flagPop = 1; }
  }
  state.flags = state.lv.mines;
  const tm = Math.floor(state.time);
  const lv = state.lv;
  const stars = tm <= lv.star3 ? 3 : tm <= lv.star2 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (!progress.bestTime[i] || tm < progress.bestTime[i]) progress.bestTime[i] = tm;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  state.winStars = stars;
  spawnSparkles();
  setTimeout(() => showWin(stars, tm), 650);
}

function spawnSparkles() {
  state.sparkle = [];
  for (let i = 0; i < 34; i++) {
    state.sparkle.push({
      x: board.x + Math.random() * board.w,
      y: board.y + Math.random() * board.h,
      vx: (Math.random() - 0.5) * 120,
      vy: -60 - Math.random() * 140,
      life: 0.7 + Math.random() * 0.6,
      hue: ['#f2cf3f', '#5aa9ff', '#5fd17a'][(Math.random() * 3) | 0],
    });
  }
}

function showWin(stars, tm) {
  document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
  document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
  document.getElementById('win-line').textContent = t('winLine', formatTime(tm));
  const best = progress.bestTime[state.index];
  document.getElementById('win-best').textContent = best != null ? t('bestLine', formatTime(best)) : '';
  document.getElementById('btn-next').style.display =
    state.index + 1 < LEVEL_COUNT ? '' : 'none';
  showOverlay('overlay-win');
}

// ---- HUD -----------------------------------------------------------------
function formatTime(sec) {
  const m = Math.floor(sec / 60), s = Math.floor(sec % 60);
  return m + ':' + String(s).padStart(2, '0');
}
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-mines').textContent =
    t('mines') + ' ' + (state.lv.mines - state.flags);
  document.getElementById('hud-time').textContent = formatTime(state.time);
}
function updateControls() {
  const flagBtn = document.getElementById('btn-flag');
  flagBtn.textContent = flagMode ? t('modeFlag') : t('modeDig');
  flagBtn.classList.toggle('active', flagMode);
  const scanBtn = document.getElementById('btn-scan');
  const scans = state ? state.scansLeft : 1;
  scanBtn.textContent = scans > 0 ? t('scan') + ' (' + scans + ')' : t('noScans');
  scanBtn.disabled = scans <= 0;
}

// ---- input ---------------------------------------------------------------
function cellAt(clientX, clientY) {
  const rect = canvas.getBoundingClientRect();
  const x = (clientX - rect.left) * VW / rect.width;
  const y = (clientY - rect.top) * VH / rect.height;
  const c = Math.floor((x - board.x) / board.tile);
  const r = Math.floor((y - board.y) / board.tile);
  if (!inBounds(c, r)) return -1;
  return r * state.cols + c;
}

let press = null;
let pressTimer = 0;

function gameActive() {
  return state && state.status === 'play' &&
    !document.getElementById('screen-game').classList.contains('hidden');
}

canvas.addEventListener('pointerdown', e => {
  if (!gameActive()) return;
  const idx = cellAt(e.clientX, e.clientY);
  if (idx < 0) return;
  press = { idx, x: e.clientX, y: e.clientY, consumed: false };
  pressTimer = setTimeout(() => {
    if (press) { toggleFlag(press.idx); press.consumed = true; }
  }, 420);
});
canvas.addEventListener('pointermove', e => {
  if (!press) return;
  if (Math.hypot(e.clientX - press.x, e.clientY - press.y) > 14) {
    clearTimeout(pressTimer);
    press = null;
  }
});
canvas.addEventListener('pointerup', e => {
  clearTimeout(pressTimer);
  if (!press || !gameActive()) { press = null; return; }
  if (!press.consumed) {
    const c = state.cells[press.idx];
    if (c.revealed) chordCell(press.idx);
    else if (flagMode) toggleFlag(press.idx);
    else digCell(press.idx);
  }
  press = null;
});
canvas.addEventListener('pointercancel', () => { clearTimeout(pressTimer); press = null; });
canvas.addEventListener('contextmenu', e => {
  e.preventDefault();
  if (!gameActive()) return;
  const idx = cellAt(e.clientX, e.clientY);
  if (idx >= 0) toggleFlag(idx);
});

// ---- update / render -----------------------------------------------------
function update(dt) {
  if (!state) return;
  if (state.running && state.status === 'play') state.time += dt;
  for (const c of state.cells) {
    if (c.pop > 0) c.pop = Math.max(0, c.pop - dt * 6);
    if (c.flagPop > 0) c.flagPop = Math.max(0, c.flagPop - dt * 5);
  }
  if (state.flash > 0) state.flash = Math.max(0, state.flash - dt * 1.6);
  for (const s of state.sparkle) {
    s.life -= dt;
    s.x += s.vx * dt;
    s.y += s.vy * dt;
    s.vy += 260 * dt;
  }
  state.sparkle = state.sparkle.filter(s => s.life > 0);
  updateHud();
}

function render() {
  drawBackground(ctx, VW, VH);
  if (!state) return;
  const s = board.tile;
  const exposed = state.status === 'lose';

  // board backing doubles as the 1px grid lines between tiles.
  ctx.fillStyle = '#0a111d';
  ctx.fillRect(board.x - 1, board.y - 1, board.w + 2, board.h + 2);

  for (let i = 0; i < state.n; i++) {
    const c = state.cells[i];
    const bx = board.x + (i % state.cols) * s;
    const by = board.y + ((i / state.cols) | 0) * s;
    const ts = s - 1;
    const ccx = bx + ts / 2, ccy = by + ts / 2;

    if (c.revealed) {
      drawRevealed(ctx, bx, by, ts, i === state.hitIndex);
      if (c.mine) {
        drawMine(ctx, ccx, ccy, ts * 0.3, true);
      } else if (c.adj > 0) {
        const sc = 1 - c.pop * 0.5;
        ctx.save();
        ctx.translate(ccx, ccy);
        ctx.scale(sc, sc);
        ctx.translate(-ccx, -ccy);
        drawDigit(ctx, c.adj, ccx, ccy, Math.max(2, Math.round(ts * 0.16)), NUM_COLORS[c.adj]);
        ctx.restore();
      }
    } else {
      drawCovered(ctx, bx, by, ts);
      if (exposed && c.mine && !c.flagged) {
        drawMine(ctx, ccx, ccy, ts * 0.3, false);
      } else if (c.flagged) {
        if (exposed && !c.mine) drawWrongFlag(ctx, bx, by, ts);
        else drawFlag(ctx, bx, by, ts, 1 + c.flagPop * 0.35);
      }
    }
  }

  for (const sp of state.sparkle) {
    ctx.globalAlpha = Math.min(1, sp.life * 1.8);
    ctx.fillStyle = sp.hue;
    ctx.fillRect(Math.round(sp.x), Math.round(sp.y), 3, 3);
  }
  ctx.globalAlpha = 1;

  if (state.flash > 0) {
    ctx.fillStyle = 'rgba(255, 70, 60, ' + (state.flash * 0.5) + ')';
    ctx.fillRect(0, 0, VW, VH);
  }
}

// ---- screens / overlays --------------------------------------------------
function showScreen(id) {
  hideAllOverlays();
  document.querySelectorAll('.screen').forEach(el => el.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() {
  document.querySelectorAll('.overlay').forEach(el => el.classList.add('hidden'));
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
      '</span><em>' + lv.cols + '×' + lv.rows + ' · ' + lv.mines + '</em>';
    if (!locked) btn.onclick = () => startLevel(i);
    grid.appendChild(btn);
  });
}

function startLevel(index) {
  newGame(index);
  showScreen('screen-game');
}

// ---- buttons -------------------------------------------------------------
document.getElementById('btn-play').onclick = () => {
  let next = 0;
  for (let i = 0; i < LEVEL_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startLevel(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-flag').onclick = () => { flagMode = !flagMode; updateControls(); };
document.getElementById('btn-scan').onclick = doScan;
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-over-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-over-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  updateHud();
  updateControls();
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
    drawBackground(ctx, VW, VH);
  }
  requestAnimationFrame(loop);
}
showScreen('screen-title');
requestAnimationFrame(loop);
