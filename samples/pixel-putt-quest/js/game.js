// Pixel Putt Quest - a mini-golf game: drag to aim, sink the ball, beat par.

const SAVE_KEY = 'pixel-putt-quest-save';
const REST = 0.62;             // wall bounce energy retention
const STOP_SPEED = 13;         // below this the ball is at rest
const SINK_SPEED = 215;        // max speed that still drops into the cup

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let progress = loadProgress();
let lastT = performance.now();

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

// ---- hole lifecycle ----------------------------------------------------
function loadHole(index) {
  const hole = HOLES[index];
  game = {
    index, hole, walls: borderWalls().concat(hole.walls),
    ball: { x: hole.start.x, y: hole.start.y, vx: 0, vy: 0, resting: true },
    strokes: 0, sunk: false, aiming: false, dragX: 0, dragY: 0,
    banner: null, t: 0, sinkT: 0,
  };
  updateHud();
}

function rectHas(r, x, y) {
  return x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3];
}

function shoot() {
  const b = game.ball;
  const dx = b.x - game.dragX, dy = b.y - game.dragY;
  const len = Math.hypot(dx, dy);
  if (len < 8) return;
  const power = Math.min(1, len / DRAG_MAX) * MAX_POWER;
  b.vx = dx / len * power;
  b.vy = dy / len * power;
  b.resting = false;
  game.strokes++;
  updateHud();
}

function reflect(b, nx, ny) {
  const dot = b.vx * nx + b.vy * ny;
  if (dot < 0) { b.vx -= (1 + REST) * dot * nx; b.vy -= (1 + REST) * dot * ny; }
}

function collideWall(b, w) {
  const cx = Math.max(w[0], Math.min(b.x, w[0] + w[2]));
  const cy = Math.max(w[1], Math.min(b.y, w[1] + w[3]));
  const dx = b.x - cx, dy = b.y - cy, d2 = dx * dx + dy * dy;
  if (d2 >= BALL_R * BALL_R) return;
  if (d2 > 0.05) {
    const d = Math.sqrt(d2), nx = dx / d, ny = dy / d;
    b.x = cx + nx * BALL_R; b.y = cy + ny * BALL_R;
    reflect(b, nx, ny);
  } else {
    const l = b.x - w[0], r = w[0] + w[2] - b.x, t2 = b.y - w[1], bt = w[1] + w[3] - b.y;
    const m = Math.min(l, r, t2, bt);
    if (m === l) { b.x = w[0] - BALL_R; reflect(b, -1, 0); }
    else if (m === r) { b.x = w[0] + w[2] + BALL_R; reflect(b, 1, 0); }
    else if (m === t2) { b.y = w[1] - BALL_R; reflect(b, 0, -1); }
    else { b.y = w[1] + w[3] + BALL_R; reflect(b, 0, 1); }
  }
}

function splash() {
  game.ball.x = game.hole.start.x;
  game.ball.y = game.hole.start.y;
  game.ball.vx = game.ball.vy = 0;
  game.ball.resting = true;
  game.strokes++;
  setBanner(t('splash'), '#7ad0ff');
  updateHud();
}

function setBanner(text, color) { game.banner = { text, color, life: 1.3 }; }

// ---- update ------------------------------------------------------------
function update(dt) {
  const g = game;
  g.t += dt;
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }
  if (g.sunk) { g.sinkT += dt; if (g.sinkT > 0.7) showResult(); return; }
  const b = g.ball;
  if (b.resting) return;

  const speed = Math.hypot(b.vx, b.vy);
  const steps = Math.max(1, Math.ceil(speed * dt / 4));
  const sdt = dt / steps;
  for (let s = 0; s < steps; s++) {
    const onSand = g.hole.sand.some(r => rectHas(r, b.x, b.y));
    const damp = Math.pow(onSand ? 0.012 : 0.42, sdt);
    b.vx *= damp; b.vy *= damp;
    b.x += b.vx * sdt; b.y += b.vy * sdt;
    for (const w of g.walls) collideWall(b, w);
    // water hazard
    if (g.hole.water.some(r => rectHas(r, b.x, b.y))) { splash(); return; }
    // cup
    const cd = Math.hypot(b.x - g.hole.cup.x, b.y - g.hole.cup.y);
    const sp = Math.hypot(b.vx, b.vy);
    if (cd < CUP_R && sp < SINK_SPEED) { sinkBall(); return; }
    if (cd < CUP_R + 6 && sp < SINK_SPEED * 1.5) {
      // gentle pull toward the cup near the rim
      b.vx += (g.hole.cup.x - b.x) * 6 * sdt;
      b.vy += (g.hole.cup.y - b.y) * 6 * sdt;
    }
  }
  if (Math.hypot(b.vx, b.vy) < STOP_SPEED) { b.vx = b.vy = 0; b.resting = true; }
}

function sinkBall() {
  const g = game;
  g.ball.x = g.hole.cup.x; g.ball.y = g.hole.cup.y;
  g.ball.vx = g.ball.vy = 0;
  g.sunk = true; g.sinkT = 0;
  setBanner(g.strokes === 1 ? t('holeInOne') : t('sunk'), '#ffe14d');
  const prev = progress.best[g.index];
  if (prev == null || g.strokes < prev) progress.best[g.index] = g.strokes;
  progress.unlocked = Math.max(progress.unlocked, Math.min(HOLE_COUNT, g.index + 2));
  saveProgress();
}

function showResult() {
  const g = game;
  if (g.resultShown) return;
  g.resultShown = true;
  const diff = g.strokes - g.hole.par;
  document.getElementById('result-strokes').textContent =
    `${t('strokes')}: ${g.strokes}  ·  ${t('par')}: ${g.hole.par}`;
  document.getElementById('result-diff').textContent =
    diff < 0 ? t('underPar', -diff) : diff > 0 ? t('overPar', diff) : t('onPar');
  if (g.index + 1 >= HOLE_COUNT) {
    let total = 0;
    for (let i = 0; i < HOLE_COUNT; i++) total += progress.best[i] || HOLES[i].par;
    const prevBest = +(localStorage.getItem(SAVE_KEY + '-total') || 0);
    if (!prevBest || total < prevBest) {
      try { localStorage.setItem(SAVE_KEY + '-total', total); } catch (e) { /* ignore */ }
    }
    document.getElementById('alldone-msg').textContent = t('allDoneMsg', total);
    showOverlay('overlay-alldone');
  } else {
    showOverlay('overlay-result');
  }
}

// ---- render ------------------------------------------------------------
function render() {
  const g = game;
  drawCourse(ctx, g.hole, g.t);
  drawWalls(ctx, g.hole.walls);
  drawCup(ctx, g.hole.cup, g.t);
  if (!g.sunk || g.sinkT < 0.1) drawBall(ctx, g.ball.x, g.ball.y);
  if (g.aiming && g.ball.resting && !g.sunk) drawAim(ctx, g.ball, g.dragX, g.dragY);
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life);
    ctx.fillStyle = g.banner.color;
    ctx.font = '900 26px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, VW / 2, 54);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function updateHud() {
  if (!game) return;
  document.getElementById('hud-hole').textContent = `${t('hole')} ${game.index + 1}`;
  document.getElementById('hud-par').textContent = `${t('par')} ${game.hole.par}`;
  document.getElementById('hud-strokes').textContent = `${t('strokes')} ${game.strokes}`;
  const b = progress.best[game.index];
  document.getElementById('hud-best').textContent = `${t('best')} ${b == null ? '—' : b}`.replace(/Best total:?\s*/i, '');
}

// ---- level select ------------------------------------------------------
function buildHoleGrid() {
  const grid = document.getElementById('hole-grid');
  grid.innerHTML = '';
  for (let i = 0; i < HOLE_COUNT; i++) {
    const unlocked = i < progress.unlocked;
    const btn = document.createElement('button');
    btn.className = 'hole-cell' + (unlocked ? '' : ' locked');
    const b = progress.best[i];
    btn.innerHTML = `<b>${i + 1}</b><span>${unlocked ? (b == null ? 'P' + HOLES[i].par : b + '/' + HOLES[i].par) : '🔒'}</span>`;
    btn.disabled = !unlocked;
    btn.onclick = () => { loadHole(i); showScreen('screen-game'); };
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

function gotoTitle() {
  hideAllOverlays();
  const total = +(localStorage.getItem(SAVE_KEY + '-total') || 0);
  document.getElementById('title-best').textContent = total ? t('best', total) : '';
  showScreen('screen-title');
}
function gotoHoles() { hideAllOverlays(); buildHoleGrid(); showScreen('screen-levels'); }

// ---- input -------------------------------------------------------------
function canvasPt(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * VW, y: (e.clientY - r.top) / r.height * VH };
}
canvas.addEventListener('pointerdown', e => {
  if (!game || game.sunk || !game.ball.resting || !overlaysClosed()) return;
  e.preventDefault();
  const p = canvasPt(e);
  game.aiming = true; game.dragX = p.x; game.dragY = p.y;
});
canvas.addEventListener('pointermove', e => {
  if (!game || !game.aiming) return;
  const p = canvasPt(e);
  game.dragX = p.x; game.dragY = p.y;
});
canvas.addEventListener('pointerup', e => {
  if (!game || !game.aiming) return;
  const p = canvasPt(e);
  game.dragX = p.x; game.dragY = p.y;
  game.aiming = false;
  if (game.ball.resting && !game.sunk) shoot();
});
canvas.addEventListener('pointercancel', () => { if (game) game.aiming = false; });

document.getElementById('btn-play').onclick = () => {
  loadHole(Math.min(progress.unlocked - 1, HOLE_COUNT - 1));
  showScreen('screen-game');
};
document.getElementById('btn-levels').onclick = gotoHoles;
document.getElementById('btn-levels-back').onclick = gotoTitle;
document.getElementById('btn-game-menu').onclick = gotoTitle;
document.getElementById('btn-restart').onclick = () => { if (game) loadHole(game.index); };
document.getElementById('btn-result-menu').onclick = gotoHoles;
document.getElementById('btn-next').onclick = () => {
  hideAllOverlays();
  loadHole(Math.min(game.index + 1, HOLE_COUNT - 1));
  showScreen('screen-game');
};
document.getElementById('btn-alldone-menu').onclick = gotoTitle;
setupLanguageToggle(() => { if (game) updateHud(); buildHoleGrid(); });

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.032, (now - lastT) / 1000);
  lastT = now;
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) {
    if (overlaysClosed()) update(dt);
    render();
  }
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
