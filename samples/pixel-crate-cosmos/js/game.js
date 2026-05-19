// Pixel Crate Cosmos - a space-station box-pushing puzzle: slide on ice,
// push every power-core onto a socket, undo freely, clear all stations.

const SAVE_KEY = 'pixel-crate-cosmos-save';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const CW = 480, CH = 432;
canvas.width = CW;
canvas.height = CH;
ctx.imageSmoothingEnabled = false;

let game = null;          // active level state
let progress = loadProgress();
let animT = 0;

// ---- pure move simulation (shared with the puzzle, undo, level checks) --
function stepSim(tiles, player, crates, dx, dy) {
  const wall = (x, y) => y < 0 || y >= tiles.length || x < 0 || x >= tiles[y].length || tiles[y][x] === T_WALL;
  let cx = player.x, cy = player.y, moved = false;
  const out = crates.map(c => ({ x: c.x, y: c.y }));
  for (let guard = 0; guard < 256; guard++) {
    const nx = cx + dx, ny = cy + dy;
    if (wall(nx, ny)) break;
    const ci = out.findIndex(c => c.x === nx && c.y === ny);
    if (ci >= 0) {
      const tx = nx + dx, ty = ny + dy;
      if (wall(tx, ty) || out.some(c => c.x === tx && c.y === ty)) break;
      out[ci] = { x: tx, y: ty };
      cx = nx; cy = ny; moved = true;
      break;                       // a push always ends the slide
    }
    cx = nx; cy = ny; moved = true;
    if (tiles[ny][nx] === T_ICE) continue;
    break;
  }
  return { px: cx, py: cy, crates: out, moved };
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
  const p = parseLevel(LEVELS[index]);
  game = {
    index, w: p.w, h: p.h, tiles: p.tiles, targets: p.targets,
    crates: p.crates, player: p.player, facing: 2, moves: 0, history: [], won: false,
  };
  updateHud();
}

function dirCode(dx, dy) { return dx ? (dx > 0 ? 1 : -1) : (dy > 0 ? 2 : -2); }

function move(dx, dy) {
  if (!game || game.won || !overlaysClosed()) return;
  game.facing = dirCode(dx, dy);
  const res = stepSim(game.tiles, game.player, game.crates, dx, dy);
  if (!res.moved) return;
  game.history.push({
    player: { x: game.player.x, y: game.player.y },
    crates: game.crates.map(c => ({ x: c.x, y: c.y })),
    moves: game.moves,
  });
  if (game.history.length > 400) game.history.shift();
  game.player = { x: res.px, y: res.py };
  game.crates = res.crates;
  game.moves++;
  updateHud();
  if (isSolved()) winLevel();
}

function isSolved() {
  return game.targets.every(tg => game.crates.some(c => c.x === tg.x && c.y === tg.y));
}

function undo() {
  if (!game || game.won || !game.history.length) return;
  const s = game.history.pop();
  game.player = s.player;
  game.crates = s.crates;
  game.moves = s.moves;
  updateHud();
}

function restart() {
  if (!game) return;
  loadLevel(game.index);
}

function winLevel() {
  game.won = true;
  const idx = game.index;
  const prevBest = progress.best[idx];
  const isBest = prevBest == null || game.moves < prevBest;
  if (isBest) progress.best[idx] = game.moves;
  progress.unlocked = Math.max(progress.unlocked, Math.min(LEVEL_COUNT, idx + 2));
  saveProgress();
  if (idx + 1 >= LEVEL_COUNT) {
    showOverlay('overlay-alldone');
  } else {
    document.getElementById('cleared-msg').textContent = t('clearedMsg', game.moves);
    document.getElementById('cleared-best').textContent = isBest
      ? t('newBest') : t('bestMoves', progress.best[idx]);
    showOverlay('overlay-cleared');
  }
}

// ---- rendering ---------------------------------------------------------
function render() {
  ctx.fillStyle = '#0b0e18';
  ctx.fillRect(0, 0, CW, CH);
  if (!game) return;
  const s = Math.floor(Math.min(CW / game.w, CH / game.h));
  const ox = Math.floor((CW - s * game.w) / 2);
  const oy = Math.floor((CH - s * game.h) / 2);

  for (let y = 0; y < game.h; y++) {
    for (let x = 0; x < game.w; x++) {
      const px = ox + x * s, py = oy + y * s;
      const tile = game.tiles[y][x];
      if (tile === T_WALL) drawWall(ctx, px, py, s);
      else if (tile === T_ICE) drawIce(ctx, px, py, s, animT);
      else drawFloor(ctx, px, py, s);
    }
  }
  for (const tg of game.targets) drawSocket(ctx, ox + tg.x * s, oy + tg.y * s, s, animT);
  for (const c of game.crates) {
    const onTarget = game.targets.some(tg => tg.x === c.x && tg.y === c.y);
    drawCrate(ctx, ox + c.x * s, oy + c.y * s, s, onTarget, animT);
  }
  drawRobot(ctx, ox + game.player.x * s, oy + game.player.y * s, s, game.facing, animT);
}

function updateHud() {
  if (!game) return;
  document.getElementById('hud-level').textContent = `${t('level')} ${game.index + 1}`;
  document.getElementById('hud-moves').textContent = `${t('moves')} ${game.moves}`;
  const b = progress.best[game.index];
  document.getElementById('hud-best').textContent = `${t('best')} ${b == null ? '—' : b}`;
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
    btn.innerHTML = `<b>${i + 1}</b><span>${unlocked ? (b == null ? '—' : b + 'm') : '🔒'}</span>`;
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
  else if (e.key === 'z' || e.key === 'Z' || e.key === 'u' || e.key === 'U') undo();
  else if (e.key === 'r' || e.key === 'R') restart();
});

[['btn-up', 0, -1], ['btn-down', 0, 1], ['btn-left', -1, 0], ['btn-right', 1, 0]].forEach(([id, dx, dy]) => {
  document.getElementById(id).addEventListener('click', () => move(dx, dy));
});
document.getElementById('btn-undo').addEventListener('click', undo);
document.getElementById('btn-restart').addEventListener('click', restart);

let touch = null;
canvas.addEventListener('touchstart', e => {
  touch = { x: e.touches[0].clientX, y: e.touches[0].clientY };
}, { passive: true });
canvas.addEventListener('touchend', e => {
  if (!touch) return;
  const dx = e.changedTouches[0].clientX - touch.x;
  const dy = e.changedTouches[0].clientY - touch.y;
  touch = null;
  if (Math.abs(dx) < 22 && Math.abs(dy) < 22) return;
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
setupLanguageToggle(() => { updateHud(); buildLevelGrid(); });

// ---- loop --------------------------------------------------------------
function loop() {
  animT += 0.05;
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
