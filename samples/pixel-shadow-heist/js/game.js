// Pixel Shadow Heist - turn-based stealth. Each move is a turn; guards then
// step their loops. Walk into a vision ray or onto a guard and you're caught.

const SAVE_KEY = 'pixel-shadow-heist-save';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const CW = 520, CH = 440;
canvas.width = CW;
canvas.height = CH;
ctx.imageSmoothingEnabled = false;

let game = null;
let progress = loadProgress();
let animT = 0;

// ---- pure stealth model (shared with the solvability check) ------------
function precomputeFaces(guard) {
  const p = guard.patrol, n = p.length;
  const faces = [];
  for (let i = 0; i < n; i++) {
    const prev = p[(i - 1 + n) % n];
    faces.push([p[i][0] - prev[0], p[i][1] - prev[1]]);
  }
  for (let i = 0; i < n; i++) {           // carry facing through any pauses
    if (faces[i][0] === 0 && faces[i][1] === 0) faces[i] = faces[(i - 1 + n) % n];
  }
  guard.faces = faces;
}

function guardPos(guard, turn) { return guard.patrol[turn % guard.patrol.length]; }

function caughtAt(px, py, turn, lvl) {
  for (const g of lvl.guards) {
    const gp = guardPos(g, turn);
    if (gp[0] === px && gp[1] === py) return true;
    const f = g.faces[turn % g.patrol.length];
    for (let k = 1; k <= VISION; k++) {
      const tx = gp[0] + f[0] * k, ty = gp[1] + f[1] * k;
      if (tx < 0 || ty < 0 || ty >= lvl.cell.h || tx >= lvl.cell.w || lvl.cell.walls[ty][tx]) break;
      if (tx === px && ty === py) return true;
    }
  }
  return false;
}

function visionTilesAt(turn, lvl) {
  const tiles = [];
  for (const g of lvl.guards) {
    const gp = guardPos(g, turn);
    const f = g.faces[turn % g.patrol.length];
    for (let k = 1; k <= VISION; k++) {
      const tx = gp[0] + f[0] * k, ty = gp[1] + f[1] * k;
      if (tx < 0 || ty < 0 || ty >= lvl.cell.h || tx >= lvl.cell.w || lvl.cell.walls[ty][tx]) break;
      tiles.push({ x: tx, y: ty });
    }
  }
  return tiles;
}

// Resolve every level's grid + guard facings once at load.
for (const lv of LEVELS) {
  lv.cell = parseHeist(lv.grid);
  lv.guards.forEach(precomputeFaces);
}

// ---- progress save -----------------------------------------------------
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, best: d.best || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, best: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}

// ---- level lifecycle ---------------------------------------------------
function loadLevel(index) {
  const lvl = LEVELS[index];
  game = {
    index, lvl,
    player: { x: lvl.cell.start.x, y: lvl.cell.start.y },
    turn: 0, caught: 0, won: false, flash: 0,
  };
  updateHud();
}

function move(dx, dy) {
  if (!game || game.won || game.flash > 0 || !overlaysClosed()) return;
  const p = game.player, c = game.lvl.cell;
  if (dx !== 0 || dy !== 0) {
    const nx = p.x + dx, ny = p.y + dy;
    if (nx < 0 || ny < 0 || nx >= c.w || ny >= c.h || c.walls[ny][nx]) return; // blocked, no turn
    p.x = nx; p.y = ny;
  }
  game.turn++;
  if (p.x === c.exit.x && p.y === c.exit.y) { winLevel(); return; }
  if (caughtAt(p.x, p.y, game.turn, game.lvl)) {
    game.caught++;
    game.flash = 0.5;
  }
  updateHud();
}

// after the spotted flash, snap back to the start and resume
function resolveFlash() {
  game.player = { x: game.lvl.cell.start.x, y: game.lvl.cell.start.y };
  game.turn = 0;
  updateHud();
}

function winLevel() {
  game.won = true;
  const idx = game.index;
  const prev = progress.best[idx];
  if (prev == null || game.turn < prev) progress.best[idx] = game.turn;
  progress.unlocked = Math.max(progress.unlocked, Math.min(LEVEL_COUNT, idx + 2));
  saveProgress();
  if (idx + 1 >= LEVEL_COUNT) {
    showOverlay('overlay-alldone');
  } else {
    document.getElementById('cleared-msg').textContent = t('clearedMsg', game.turn, game.caught);
    showOverlay('overlay-cleared');
  }
}

// ---- rendering ---------------------------------------------------------
function render() {
  ctx.fillStyle = '#0c0e16';
  ctx.fillRect(0, 0, CW, CH);
  if (!game) return;
  const c = game.lvl.cell;
  const s = Math.floor(Math.min(CW / c.w, CH / c.h));
  const ox = Math.floor((CW - s * c.w) / 2);
  const oy = Math.floor((CH - s * c.h) / 2);

  for (let y = 0; y < c.h; y++) {
    for (let x = 0; x < c.w; x++) {
      const px = ox + x * s, py = oy + y * s;
      if (c.walls[y][x]) drawWall(ctx, px, py, s);
      else if (x === c.exit.x && y === c.exit.y) drawExit(ctx, px, py, s, animT);
      else drawFloor(ctx, px, py, s);
    }
  }
  // vision cones at the current turn
  for (const v of visionTilesAt(game.turn, game.lvl)) {
    drawVision(ctx, ox + v.x * s, oy + v.y * s, s);
  }
  // guards
  for (const g of game.lvl.guards) {
    const gp = guardPos(g, game.turn);
    drawGuard(ctx, ox + gp[0] * s, oy + gp[1] * s, s, faceVec(g, game.turn), animT);
  }
  // thief (hidden during the spotted flash blink)
  if (game.flash <= 0 || Math.floor(animT * 16) % 2) {
    drawThief(ctx, ox + game.player.x * s, oy + game.player.y * s, s, animT);
  }
  if (game.flash > 0) {
    ctx.fillStyle = `rgba(255,70,70,${game.flash * 0.5})`;
    ctx.fillRect(0, 0, CW, CH);
    ctx.fillStyle = '#ff5a5a';
    ctx.font = '900 30px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t('spotted'), CW / 2, CH / 2);
    ctx.textAlign = 'left';
  }
}

function faceVec(g, turn) {
  const f = g.faces[turn % g.patrol.length];
  return { x: f[0], y: f[1] };
}

function updateHud() {
  if (!game) return;
  document.getElementById('hud-level').textContent = `${t('level')} ${game.index + 1}`;
  document.getElementById('hud-turns').textContent = `${t('turns')} ${game.turn}`;
  document.getElementById('hud-caught').textContent = `${t('caught')} ${game.caught}`;
}

// ---- level select ------------------------------------------------------
function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const unlocked = i < progress.unlocked;
    const btn = document.createElement('button');
    btn.className = 'level-cell' + (unlocked ? '' : ' locked');
    const b = progress.best[i];
    btn.innerHTML = `<b>${i + 1}</b><span>${unlocked ? (b == null ? '—' : b + 't') : '🔒'}</span>`;
    btn.disabled = !unlocked;
    btn.onclick = () => { loadLevel(i); showScreen('screen-game'); };
    grid.appendChild(btn);
  }
}

// ---- screens / overlays -----------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() { document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden')); }
function overlaysClosed() { return document.querySelectorAll('.overlay:not(.hidden)').length === 0; }

function gotoTitle() { hideAllOverlays(); showScreen('screen-title'); }
function gotoLevels() { hideAllOverlays(); buildLevelGrid(); showScreen('screen-levels'); }

// ---- input -------------------------------------------------------------
const KEYS = {
  ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0],
  w: [0, -1], s: [0, 1], a: [-1, 0], d: [1, 0],
  W: [0, -1], S: [0, 1], A: [-1, 0], D: [1, 0],
};
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  if (KEYS[e.key]) { e.preventDefault(); move(KEYS[e.key][0], KEYS[e.key][1]); }
  else if (e.key === ' ') { e.preventDefault(); move(0, 0); }
  else if (e.key === 'r' || e.key === 'R') { if (game) loadLevel(game.index); }
});
[['btn-up', 0, -1], ['btn-down', 0, 1], ['btn-left', -1, 0], ['btn-right', 1, 0]].forEach(([id, dx, dy]) => {
  document.getElementById(id).addEventListener('click', () => move(dx, dy));
});
document.getElementById('btn-wait').addEventListener('click', () => move(0, 0));
document.getElementById('btn-restart').addEventListener('click', () => { if (game) loadLevel(game.index); });

let touch = null;
canvas.addEventListener('touchstart', e => {
  touch = { x: e.touches[0].clientX, y: e.touches[0].clientY };
}, { passive: true });
canvas.addEventListener('touchend', e => {
  if (!touch) return;
  const dx = e.changedTouches[0].clientX - touch.x;
  const dy = e.changedTouches[0].clientY - touch.y;
  touch = null;
  if (Math.abs(dx) < 20 && Math.abs(dy) < 20) { move(0, 0); return; }
  if (Math.abs(dx) > Math.abs(dy)) move(Math.sign(dx), 0);
  else move(0, Math.sign(dy));
}, { passive: true });

// ---- wiring ------------------------------------------------------------
document.getElementById('btn-play').onclick = () => {
  loadLevel(Math.min(progress.unlocked - 1, LEVEL_COUNT - 1));
  showScreen('screen-game');
};
document.getElementById('btn-levels').onclick = gotoLevels;
document.getElementById('btn-levels-back').onclick = gotoTitle;
document.getElementById('btn-game-menu').onclick = gotoTitle;
document.getElementById('btn-cleared-menu').onclick = gotoLevels;
document.getElementById('btn-alldone-menu').onclick = gotoTitle;
document.getElementById('btn-next').onclick = () => {
  hideAllOverlays();
  loadLevel(Math.min(game.index + 1, LEVEL_COUNT - 1));
  showScreen('screen-game');
};
setupLanguageToggle(() => { if (game) updateHud(); buildLevelGrid(); });

// ---- loop --------------------------------------------------------------
let lastT = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  animT += dt;
  if (game && game.flash > 0) {
    game.flash -= dt;
    if (game.flash <= 0) { game.flash = 0; resolveFlash(); }
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
