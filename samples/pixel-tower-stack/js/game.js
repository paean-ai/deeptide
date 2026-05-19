// Pixel Tower Stack - a one-tap stacking game. Drop each sliding block onto
// the tower; misaligned overhang is sliced away. Perfect drops regrow width.

const BEST_KEY = 'pixel-tower-stack-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = FW;
canvas.height = FH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = {
    stack: [{ x: (FW - W0) / 2, w: W0, hue: hueFor(0) }],
    block: null, debris: [], sparkles: [],
    view: 0, combo: 0, score: 0, banner: null, t: 0, over: false,
  };
  spawnBlock(W0);
}

function spawnBlock(w) {
  const lvl = game.stack.length;
  const fromLeft = Math.random() < 0.5;
  game.block = {
    x: fromLeft ? 0 : FW - w, w, lvl,
    dir: fromLeft ? 1 : -1, hue: hueFor(lvl),
  };
}

function setBanner(text) { game.banner = { text, life: 1 }; }

// ---- drop --------------------------------------------------------------
function drop() {
  if (!game || game.over || !overlaysClosed()) return;
  const b = game.block, below = game.stack[game.stack.length - 1];
  const L = Math.max(b.x, below.x);
  const R = Math.min(b.x + b.w, below.x + below.w);
  const overlap = R - L;
  if (overlap <= 0) {                       // complete miss - tower topples
    game.debris.push(debrisOf(b.x, b.w, b.hue, b.lvl, b.dir * 60));
    gameOver();
    return;
  }
  const perfect = Math.abs(b.x - below.x) <= PERFECT_TOL;
  let placed;
  if (perfect) {
    placed = { x: below.x, w: below.w, hue: b.hue };
    game.combo++;
    if (game.combo >= REGROW_AT) placed.w = Math.min(W0, placed.w + REGROW);
    game.sparkles.push({ x: below.x + below.w / 2, y: levelScreenY(b.lvl) + BH / 2, life: 1 });
    setBanner(t('perfect') + (game.combo > 1 ? ' x' + game.combo : ''));
  } else {
    game.combo = 0;
    placed = { x: L, w: overlap, hue: b.hue };
    if (b.x < L) game.debris.push(debrisOf(b.x, L - b.x, b.hue, b.lvl, -80));
    if (b.x + b.w > R) game.debris.push(debrisOf(R, (b.x + b.w) - R, b.hue, b.lvl, 80));
  }
  game.stack.push(placed);
  game.score++;
  spawnBlock(placed.w);
}

function debrisOf(x, w, hue, lvl, vx) {
  return { x, y: levelScreenY(lvl), w, h: BH, hue, vx, vy: -40, life: 1 };
}

function gameOver() {
  game.over = true;
  if (game.score > best()) localStorage.setItem(BEST_KEY, game.score);
  document.getElementById('over-height').textContent = t('finalHeight', game.score);
  document.getElementById('over-best').textContent = t('bestHeight', best());
  showOverlay('overlay-over');
}

// ---- camera / coords ---------------------------------------------------
// level 0 = base. The camera keeps the working top mid-canvas.
function levelScreenY(lvl) {
  return FH - 96 - (lvl - game.view) * BH;
}

// ---- update ------------------------------------------------------------
function update(dt) {
  const g = game;
  g.t += dt;
  if (g.banner) { g.banner.life -= dt * 1.4; if (g.banner.life <= 0) g.banner = null; }

  if (g.block) {
    const sp = blockSpeed(g.stack.length);
    g.block.x += g.block.dir * sp * dt;
    if (g.block.x <= 0) { g.block.x = 0; g.block.dir = 1; }
    if (g.block.x + g.block.w >= FW) { g.block.x = FW - g.block.w; g.block.dir = -1; }
  }

  const targetView = Math.max(0, g.stack.length - BASE_LEVELS);
  g.view += (targetView - g.view) * Math.min(1, dt * 6);

  for (const d of g.debris) {
    d.x += d.vx * dt; d.y += d.vy * dt; d.vy += 900 * dt;
    d.life -= dt * 0.9;
  }
  g.debris = g.debris.filter(d => d.life > 0 && d.y < FH + 60);
  for (const s of g.sparkles) s.life -= dt * 1.8;
  g.sparkles = g.sparkles.filter(s => s.life > 0);
}

// ---- render ------------------------------------------------------------
function render() {
  const g = game;
  drawBackdrop(ctx, FW, FH, g.score, g.t);
  for (let i = 0; i < g.stack.length; i++) {
    const y = levelScreenY(i);
    if (y > FH + BH || y < -BH) continue;
    const b = g.stack[i];
    drawBlock(ctx, b.x, y, b.w, BH, b.hue, false);
  }
  for (const d of g.debris) drawDebris(ctx, d);
  if (g.block && !g.over) {
    drawBlock(ctx, g.block.x, levelScreenY(g.block.lvl), g.block.w, BH, g.block.hue, true);
    // alignment guide line down to the block below
    const below = g.stack[g.stack.length - 1];
    ctx.fillStyle = 'rgba(255,255,255,0.16)';
    ctx.fillRect(below.x, levelScreenY(g.block.lvl) + BH, 2, BH);
    ctx.fillRect(below.x + below.w - 2, levelScreenY(g.block.lvl) + BH, 2, BH);
  }
  for (const s of g.sparkles) drawSparkle(ctx, s);
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life * 1.5);
    ctx.fillStyle = '#fff2b0';
    ctx.font = '900 28px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, FW / 2, 120);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function updateHud() {
  document.getElementById('hud-height').textContent = game.score;
  document.getElementById('hud-combo').textContent = game.combo > 1 ? `x${game.combo}` : '—';
}

// ---- win / lose --------------------------------------------------------
function best() { return +(localStorage.getItem(BEST_KEY) || 0); }

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
  document.getElementById('title-best').textContent = t('bestHeight', best());
  showScreen('screen-title');
}
function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideAllOverlays();
}

// ---- input -------------------------------------------------------------
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); drop(); }
  else if (e.key === 'Escape') togglePause();
});
canvas.addEventListener('pointerdown', e => { e.preventDefault(); drop(); });

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-resume').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = startGame;
document.getElementById('btn-pause-menu').onclick = gotoTitle;
document.getElementById('btn-over-again').onclick = startGame;
document.getElementById('btn-over-menu').onclick = gotoTitle;
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestHeight', best());
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
