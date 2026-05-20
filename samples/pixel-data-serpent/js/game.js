// Pixel Data Serpent - a modern grid snake: collect data nodes, clear sectors,
// dodge accumulating firewalls, and slip through portal pairs.

const BEST_KEY = 'pixel-data-serpent-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = CW;
canvas.height = CW;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();

const key = (x, y) => x + ',' + y;
const foodGrow = k => (k === 'golden' ? 2 : k === 'shrink' ? 0 : 1);

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  const mid = GRID >> 1;
  game = {
    snake: [{ x: mid + 1, y: mid }, { x: mid, y: mid }, { x: mid - 1, y: mid }],
    dir: { x: 1, y: 0 }, nextDir: { x: 1, y: 0 },
    firewalls: new Set(), portals: null,
    food: null, grow: 0, trim: 0,
    sector: 1, progress: 0, score: 0,
    stepAcc: 0, slow: 0, shields: 0, t: 0, banner: null, over: false,
  };
  spawnFood();
}

function occupied(x, y) {
  if (game.firewalls.has(key(x, y))) return true;
  if (game.food && game.food.x === x && game.food.y === y) return true;
  if (game.portals && game.portals.some(p => p.x === x && p.y === y)) return true;
  return game.snake.some(s => s.x === x && s.y === y);
}

function randomEmpty(awayFromHead) {
  const head = game.snake[0];
  for (let i = 0; i < 400; i++) {
    const x = (Math.random() * GRID) | 0, y = (Math.random() * GRID) | 0;
    if (occupied(x, y)) continue;
    if (awayFromHead && Math.max(Math.abs(x - head.x), Math.abs(y - head.y)) < 4) continue;
    return { x, y };
  }
  return null;
}

function spawnFood() {
  const spot = randomEmpty(false);
  if (spot) game.food = { x: spot.x, y: spot.y, kind: rollFoodKind(game.sector) };
}

function setBanner(text, color) { game.banner = { text, color, life: 1.3 }; }

function advanceSector() {
  game.sector++;
  game.progress = 0;
  setBanner(t('sectorUp', game.sector), '#5fd9c0');
  for (let i = 0; i < 2 && game.firewalls.size < 34; i++) {
    const spot = randomEmpty(true);
    if (spot) game.firewalls.add(key(spot.x, spot.y));
  }
  if (game.sector >= 3 && !game.portals) {
    const a = randomEmpty(true), b = randomEmpty(true);
    if (a && b) game.portals = [a, b];
  }
}

// ---- step --------------------------------------------------------------
function step() {
  const g = game;
  g.dir = g.nextDir;
  let nx = g.snake[0].x + g.dir.x;
  let ny = g.snake[0].y + g.dir.y;

  // portal teleport
  if (g.portals) {
    if (g.portals[0].x === nx && g.portals[0].y === ny) { nx = g.portals[1].x; ny = g.portals[1].y; }
    else if (g.portals[1].x === nx && g.portals[1].y === ny) { nx = g.portals[0].x; ny = g.portals[0].y; }
  }

  // fatal: walls and firewalls — but a stacked shield absorbs the hit
  // and clears the offending firewall cell (walls are still fatal).
  if (nx < 0 || nx >= GRID || ny < 0 || ny >= GRID) { die(); return; }
  if (g.firewalls.has(key(nx, ny))) {
    if (g.shields > 0) {
      g.shields--;
      g.firewalls.delete(key(nx, ny));
      setBanner(t('shield'), '#ff8fd0');
    } else { die(); return; }
  }

  const ate = g.food && g.food.x === nx && g.food.y === ny;
  const keepTail = g.grow > 0 || (ate && foodGrow(g.food.kind) > 0);

  // fatal: self-collision (the tail cell vacates unless growing). A
  // shield absorbs and trims four segments behind the head so the player
  // doesn't immediately re-collide.
  const checkLen = keepTail ? g.snake.length : g.snake.length - 1;
  for (let i = 0; i < checkLen; i++) {
    if (g.snake[i].x === nx && g.snake[i].y === ny) {
      if (g.shields > 0) {
        g.shields--;
        g.trim = 4;
        setBanner(t('shield'), '#ff8fd0');
        break;
      } else { die(); return; }
    }
  }

  g.snake.unshift({ x: nx, y: ny });
  if (ate) eatFood();
  if (g.grow > 0) g.grow--; else g.snake.pop();
  while (g.trim > 0 && g.snake.length > 3) { g.snake.pop(); g.trim--; }
  g.trim = 0;
}

function eatFood() {
  const g = game, f = FOOD[g.food.kind], kind = g.food.kind;
  g.score += f.score + g.sector * 2;
  g.progress++;
  g.grow += foodGrow(kind);
  if (kind === 'golden') setBanner(t('golden'), '#f4c85a');
  else if (kind === 'shrink') { g.trim = 3; setBanner(t('shrink'), '#7aa0ff'); }
  else if (kind === 'slow') { g.slow = 4; setBanner(t('slow'), '#b87ae0'); }
  else if (kind === 'shield') { g.shields = Math.min(2, g.shields + 1); setBanner(t('shieldUp'), '#ff8fd0'); }
  spawnFood();
  if (g.progress >= GOAL_PER_SECTOR) advanceSector();
}

function die() {
  if (game.over) return;
  game.over = true;
  if (game.score > bestScore()) localStorage.setItem(BEST_KEY, game.score);
  document.getElementById('over-score').textContent = t('finalScore', game.score);
  document.getElementById('over-sector').textContent = t('reachedSector', game.sector);
  document.getElementById('over-best').textContent = t('bestScore', bestScore());
  showOverlay('overlay-over');
}

// ---- update / render ---------------------------------------------------
function update(dt) {
  const g = game;
  g.t += dt;
  if (g.slow > 0) g.slow -= dt;
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }
  const iv = stepInterval(g.sector) * (g.slow > 0 ? 1.5 : 1);
  g.stepAcc += dt;
  let guard = 0;
  while (g.stepAcc >= iv && !g.over && guard < 8) {
    g.stepAcc -= iv;
    step();
    guard++;
  }
}

function render() {
  const g = game;
  drawGrid(ctx);
  for (const fw of g.firewalls) {
    const [x, y] = fw.split(',').map(Number);
    drawFirewall(ctx, x, y, g.t);
  }
  if (g.portals) {
    drawPortal(ctx, g.portals[0].x, g.portals[0].y, g.t, 0);
    drawPortal(ctx, g.portals[1].x, g.portals[1].y, g.t, 1);
  }
  if (g.food) drawFood(ctx, g.food.x, g.food.y, g.food.kind, g.t);
  const n = g.snake.length;
  for (let i = n - 1; i >= 0; i--) {
    drawSegment(ctx, g.snake[i].x, g.snake[i].y, i / Math.max(1, n - 1), i === 0, g.dir, g.t);
  }
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life * 1.4);
    ctx.fillStyle = g.banner.color;
    ctx.font = '900 30px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, CW / 2, CW / 2);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function updateHud() {
  document.getElementById('hud-score').textContent = game.score;
  document.getElementById('hud-sector').textContent = `${t('sector')} ${game.sector}`;
  document.getElementById('hud-len').textContent = `${t('length')} ${game.snake.length}`;
}

// ---- win / lose --------------------------------------------------------
function bestScore() { return +(localStorage.getItem(BEST_KEY) || 0); }

// ---- screens / overlays -----------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() { document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden')); }
function overlaysClosed() { return document.querySelectorAll('.overlay:not(.hidden)').length === 0; }

function startGame() { newGame(); hideAllOverlays(); showScreen('screen-game'); }
function gotoTitle() {
  hideAllOverlays();
  document.getElementById('title-best').textContent = t('bestScore', bestScore());
  showScreen('screen-title');
}
function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideAllOverlays();
}

// ---- input -------------------------------------------------------------
function turn(dx, dy) {
  if (!game || game.over) return;
  // ignore a 180-degree reversal
  if (dx === -game.dir.x && dy === -game.dir.y) return;
  game.nextDir = { x: dx, y: dy };
}
const KEYS = {
  ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0],
  w: [0, -1], s: [0, 1], a: [-1, 0], d: [1, 0],
  W: [0, -1], S: [0, 1], A: [-1, 0], D: [1, 0],
};
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  if (KEYS[e.key]) { e.preventDefault(); turn(KEYS[e.key][0], KEYS[e.key][1]); }
  else if (e.key === 'Escape') togglePause();
});
[['btn-up', 0, -1], ['btn-down', 0, 1], ['btn-left', -1, 0], ['btn-right', 1, 0]].forEach(([id, dx, dy]) => {
  document.getElementById(id).addEventListener('click', () => turn(dx, dy));
});
let touch = null;
canvas.addEventListener('touchstart', e => {
  touch = { x: e.touches[0].clientX, y: e.touches[0].clientY };
}, { passive: true });
canvas.addEventListener('touchend', e => {
  if (!touch) return;
  const dx = e.changedTouches[0].clientX - touch.x;
  const dy = e.changedTouches[0].clientY - touch.y;
  touch = null;
  if (Math.abs(dx) < 18 && Math.abs(dy) < 18) return;
  if (Math.abs(dx) > Math.abs(dy)) turn(Math.sign(dx), 0);
  else turn(0, Math.sign(dy));
}, { passive: true });

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-resume').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = startGame;
document.getElementById('btn-pause-menu').onclick = gotoTitle;
document.getElementById('btn-over-again').onclick = startGame;
document.getElementById('btn-over-menu').onclick = gotoTitle;
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestScore', bestScore());
});

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.1, (now - lastT) / 1000);
  lastT = now;
  if (game && !game.over && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    update(dt);
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
