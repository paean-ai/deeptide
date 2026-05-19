// Pixel Orb Fusion - 2048-style slide-and-merge puzzle with a one-step undo.

const BEST_KEY = 'pixel-orb-fusion-best';
const ANIM_DUR = 0.1;

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = BOARD;
canvas.height = BOARD;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();

function makeOrb(tier, r, c, popFrom) {
  return { tier, r, c, px: c * CELL, py: r * CELL, tx: c * CELL, ty: r * CELL,
    spx: c * CELL, spy: r * CELL, pop: popFrom == null ? 1 : popFrom, dead: false, pendingPop: false };
}

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = { orbs: [], score: 0, anim: 0, prev: null, t: 0, over: false, won2048: false, banner: null };
  spawnOrb();
  spawnOrb();
}

function emptyCells() {
  const taken = new Set(game.orbs.map(o => o.r + ',' + o.c));
  const out = [];
  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++) {
    if (!taken.has(r + ',' + c)) out.push({ r, c });
  }
  return out;
}

function spawnOrb() {
  const empty = emptyCells();
  if (!empty.length) return;
  const cell = empty[(Math.random() * empty.length) | 0];
  game.orbs.push(makeOrb(Math.random() < 0.1 ? 2 : 1, cell.r, cell.c, 0.1));
}

function setBanner(text) { game.banner = { text, life: 1.4 }; }

// ---- the slide / merge move -------------------------------------------
function move(dir) {
  if (!game || game.over || game.anim > 0) return;
  const snap = game.orbs.map(o => ({ tier: o.tier, r: o.r, c: o.c }));
  const snapScore = game.score;

  const grid = [];
  for (let r = 0; r < GRID; r++) grid.push(new Array(GRID).fill(null));
  for (const o of game.orbs) grid[o.r][o.c] = o;

  let moved = false, gained = 0;
  for (const line of linesFor(dir)) {
    let writeIdx = 0, lastOrb = null, lastMerged = false;
    for (let k = 0; k < line.length; k++) {
      const cell = line[k];
      const orb = grid[cell.r][cell.c];
      if (!orb) continue;
      grid[cell.r][cell.c] = null;
      if (lastOrb && lastOrb.tier === orb.tier && !lastMerged) {
        orb.r = lastOrb.r; orb.c = lastOrb.c;
        orb.dead = true;
        lastOrb.tier++;
        lastOrb.pendingPop = true;
        gained += orbValue(lastOrb.tier);
        lastMerged = true;
        moved = true;
      } else {
        const dest = line[writeIdx];
        if (orb.r !== dest.r || orb.c !== dest.c) moved = true;
        orb.r = dest.r; orb.c = dest.c;
        grid[dest.r][dest.c] = orb;
        lastOrb = orb; lastMerged = false; writeIdx++;
      }
    }
  }
  if (!moved) return;

  game.prev = { snap, score: snapScore };
  game.score += gained;
  if (game.score > bestScore()) localStorage.setItem(BEST_KEY, game.score);
  // begin slide animation
  for (const o of game.orbs) {
    o.spx = o.px; o.spy = o.py;
    o.tx = o.c * CELL; o.ty = o.r * CELL;
  }
  game.anim = ANIM_DUR;
}

function finalizeMove() {
  game.orbs = game.orbs.filter(o => !o.dead);
  for (const o of game.orbs) {
    o.px = o.tx; o.py = o.ty;
    if (o.pendingPop) { o.pop = 1.28; o.pendingPop = false; }
  }
  spawnOrb();
  if (!game.won2048 && game.orbs.some(o => o.tier >= WIN_TIER)) {
    game.won2048 = true;
    document.getElementById('result-msg').textContent = t('win2048');
    showOverlay('overlay-win');
  }
  if (isGameOver()) gameOver();
}

function isGameOver() {
  if (emptyCells().length) return false;
  const grid = [];
  for (let r = 0; r < GRID; r++) grid.push(new Array(GRID).fill(0));
  for (const o of game.orbs) grid[o.r][o.c] = o.tier;
  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++) {
    if (c < GRID - 1 && grid[r][c] === grid[r][c + 1]) return false;
    if (r < GRID - 1 && grid[r][c] === grid[r + 1][c]) return false;
  }
  return true;
}

function gameOver() {
  game.over = true;
  document.getElementById('over-score').textContent = t('finalScore', game.score);
  document.getElementById('over-best').textContent = t('bestScore', bestScore());
  showOverlay('overlay-over');
}

function undo() {
  if (!game || game.over || game.anim > 0 || !game.prev) return;
  game.orbs = game.prev.snap.map(s => makeOrb(s.tier, s.r, s.c, 1));
  game.score = game.prev.score;
  game.prev = null;
}

// ---- update / render ---------------------------------------------------
function update(dt) {
  game.t += dt;
  if (game.banner) { game.banner.life -= dt; if (game.banner.life <= 0) game.banner = null; }
  if (game.anim > 0) {
    game.anim -= dt;
    const k = Math.max(0, Math.min(1, 1 - game.anim / ANIM_DUR));
    const e = k * k * (3 - 2 * k);
    for (const o of game.orbs) {
      o.px = o.spx + (o.tx - o.spx) * e;
      o.py = o.spy + (o.ty - o.spy) * e;
    }
    if (game.anim <= 0) { game.anim = 0; finalizeMove(); }
  }
  for (const o of game.orbs) {
    if (o.pop !== 1) o.pop += (1 - o.pop) * Math.min(1, dt * 12);
    if (Math.abs(o.pop - 1) < 0.02) o.pop = 1;
  }
}

function render() {
  drawBoard(ctx);
  // dead (merging) orbs first so survivors draw on top
  for (const o of game.orbs) if (o.dead) drawOrb(ctx, o.px, o.py, o.tier, o.pop);
  for (const o of game.orbs) if (!o.dead) drawOrb(ctx, o.px, o.py, o.tier, o.pop);
  if (game.banner) {
    ctx.globalAlpha = Math.min(1, game.banner.life);
    ctx.fillStyle = '#ffe9a0';
    ctx.font = '900 30px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(game.banner.text, BOARD / 2, BOARD / 2);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  document.getElementById('hud-score').textContent = game.score;
}

function bestScore() { return +(localStorage.getItem(BEST_KEY) || 0); }

// ---- screens / overlays -----------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideOverlay(id) { document.getElementById(id).classList.add('hidden'); }
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
const KEYS = {
  ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right',
  w: 'up', s: 'down', a: 'left', d: 'right',
  W: 'up', S: 'down', A: 'left', D: 'right',
};
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden') || !overlaysClosed()) {
    if (e.key === 'Escape') togglePause();
    return;
  }
  if (KEYS[e.key]) { e.preventDefault(); move(KEYS[e.key]); }
  else if (e.key === 'z' || e.key === 'Z') undo();
  else if (e.key === 'Escape') togglePause();
});
[['btn-up', 'up'], ['btn-down', 'down'], ['btn-left', 'left'], ['btn-right', 'right']].forEach(([id, dir]) => {
  document.getElementById(id).addEventListener('click', () => { if (overlaysClosed()) move(dir); });
});
document.getElementById('btn-undo').addEventListener('click', undo);
document.getElementById('btn-restart').addEventListener('click', startGame);

let touch = null;
canvas.addEventListener('touchstart', e => {
  touch = { x: e.touches[0].clientX, y: e.touches[0].clientY };
}, { passive: true });
canvas.addEventListener('touchend', e => {
  if (!touch || !overlaysClosed()) { touch = null; return; }
  const dx = e.changedTouches[0].clientX - touch.x;
  const dy = e.changedTouches[0].clientY - touch.y;
  touch = null;
  if (Math.abs(dx) < 22 && Math.abs(dy) < 22) return;
  if (Math.abs(dx) > Math.abs(dy)) move(dx > 0 ? 'right' : 'left');
  else move(dy > 0 ? 'down' : 'up');
}, { passive: true });

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-resume').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = startGame;
document.getElementById('btn-pause-menu').onclick = gotoTitle;
document.getElementById('btn-over-again').onclick = startGame;
document.getElementById('btn-over-menu').onclick = gotoTitle;
document.getElementById('btn-win-continue').onclick = () => hideOverlay('overlay-win');
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestScore', bestScore());
});

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  if (game && !game.over && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    update(dt);
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
