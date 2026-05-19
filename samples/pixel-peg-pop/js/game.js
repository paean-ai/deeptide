// Pixel Peg Pop - launcher aiming, ball physics, peg clearing, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-peg-pop-save';
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

const REST = 0.72;
const BUCKET_Y = VH - 32;
let state = null;
let aiming = false;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const lv = LEVELS[index];
  const pegs = [];
  lv.grid.forEach((row, r) => {
    for (let c = 0; c < row.length; c++) {
      const ch = row[c];
      if (ch === 'o' || ch === 'x') {
        pegs.push({ x: pegX(c, r), y: pegY(r), target: ch === 'x', lit: false, cleared: false });
      }
    }
  });
  state = {
    index, lv, pegs,
    targetsLeft: pegs.filter(p => p.target).length,
    balls: lv.balls, score: 0,
    phase: 'aim', ball: null, aimAngle: Math.PI / 2,
    bucket: { x: VW / 2, dir: 1 },
    particles: [], turnT: 0, stuckT: 0, status: 'play',
  };
  updateHud();
}

// ---- firing --------------------------------------------------------------
function fireBall() {
  const a = state.aimAngle;
  state.ball = {
    x: LAUNCH_X + Math.cos(a) * 14, y: LAUNCH_Y + Math.sin(a) * 14,
    vx: Math.cos(a) * LAUNCH_SPEED, vy: Math.sin(a) * LAUNCH_SPEED,
  };
  state.balls--;
  state.phase = 'fire';
  state.turnT = 0;
  state.stuckT = 0;
  updateHud();
}

function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, s = 40 + Math.random() * 150;
    state.particles.push({
      x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s - 30,
      life: 0.35 + Math.random() * 0.45, color,
    });
  }
}

function stepBall(dt) {
  const b = state.ball;
  const STEPS = 5, h = dt / STEPS;
  for (let s = 0; s < STEPS; s++) {
    b.vy += GRAVITY * h;
    b.x += b.vx * h;
    b.y += b.vy * h;
    if (b.x < BALL_R) { b.x = BALL_R; b.vx = Math.abs(b.vx) * 0.9; }
    if (b.x > VW - BALL_R) { b.x = VW - BALL_R; b.vx = -Math.abs(b.vx) * 0.9; }
    if (b.y < BALL_R) { b.y = BALL_R; b.vy = Math.abs(b.vy) * 0.9; }
    for (const peg of state.pegs) {
      if (peg.cleared) continue;
      const dx = b.x - peg.x, dy = b.y - peg.y;
      const d2 = dx * dx + dy * dy;
      const rr = BALL_R + PEG_R;
      if (d2 < rr * rr) {
        const d = Math.sqrt(d2) || 0.001;
        const nx = dx / d, ny = dy / d;
        b.x = peg.x + nx * rr;
        b.y = peg.y + ny * rr;
        const vn = b.vx * nx + b.vy * ny;
        if (vn < 0) {
          b.vx -= (1 + REST) * vn * nx;
          b.vy -= (1 + REST) * vn * ny;
        }
        if (!peg.lit) { peg.lit = true; burst(peg.x, peg.y, 3, peg.target ? '#ff9e42' : '#4a8be0'); }
      }
    }
  }
  // bucket catch
  const bk = state.bucket;
  if (b.y + BALL_R > BUCKET_Y + 4 && b.y - BALL_R < BUCKET_Y + 16 &&
      b.x > bk.x - 16 && b.x < bk.x + 16 && b.vy > 0) {
    endTurn(true);
    return;
  }
  state.turnT += dt;
  const spd = Math.hypot(b.vx, b.vy);
  state.stuckT = spd < 26 ? state.stuckT + dt : 0;
  if (b.y > VH + BALL_R || state.stuckT > 0.9 || state.turnT > 13) endTurn(false);
}

function endTurn(caught) {
  state.ball = null;
  if (caught) {
    state.balls++;
    state.particles.push({ x: state.bucket.x, y: BUCKET_Y, vx: 0, vy: -90, life: 0.9, color: '#6fd0d0' });
  }
  let cleared = 0;
  for (const peg of state.pegs) {
    if (peg.lit && !peg.cleared) {
      peg.cleared = true;
      cleared++;
      state.score += peg.target ? 120 : 10;
      burst(peg.x, peg.y, 10, peg.target ? '#ff9e42' : '#4a8be0');
      if (peg.target) state.targetsLeft--;
    }
  }
  if (cleared >= 3) state.score += cleared * cleared * 3;
  updateHud();
  if (state.targetsLeft <= 0) { winLevel(); return; }
  if (state.balls <= 0) { loseLevel(); return; }
  state.phase = 'aim';
}

function winLevel() {
  state.status = 'win';
  const stars = state.balls >= 3 ? 3 : state.balls >= 1 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', state.score, state.balls);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 700);
}
function loseLevel() {
  state.status = 'lose';
  setTimeout(() => showOverlay('overlay-lose'), 700);
}

// ---- update / render -----------------------------------------------------
function update(dt) {
  if (!state) return;
  const bk = state.bucket;
  bk.x += bk.dir * 86 * dt;
  if (bk.x < 40) { bk.x = 40; bk.dir = 1; }
  if (bk.x > VW - 40) { bk.x = VW - 40; bk.dir = -1; }
  for (const p of state.particles) {
    p.life -= dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 300 * dt;
  }
  state.particles = state.particles.filter(p => p.life > 0);
  if (state.phase === 'fire' && state.ball && state.status === 'play') stepBall(dt);
}

function render() {
  drawBackground(ctx);
  if (!state) return;
  for (const peg of state.pegs) drawPeg(ctx, peg);
  drawBucket(ctx, state.bucket.x, BUCKET_Y);
  drawLauncher(ctx, state.aimAngle);
  if (state.phase === 'aim' && state.status === 'play') drawAimDots(ctx, state.aimAngle);
  if (state.ball) drawBall(ctx, state.ball);
  for (const p of state.particles) {
    ctx.globalAlpha = Math.min(1, p.life * 2.6);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x | 0, p.y | 0, 3, 3);
  }
  ctx.globalAlpha = 1;
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-orange').textContent = t('orange') + ' ' + state.targetsLeft;
  document.getElementById('hud-balls').textContent = '●'.repeat(Math.max(0, state.balls)) || '—';
}

// ---- input ---------------------------------------------------------------
function pointerPos(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
function aimAt(p) {
  const dx = p.x - LAUNCH_X;
  const dy = Math.max(p.y - LAUNCH_Y, 24);
  state.aimAngle = Math.atan2(dy, dx);
}
function gameActive() {
  return state && state.status === 'play' && state.phase === 'aim' &&
    !document.getElementById('screen-game').classList.contains('hidden');
}
canvas.addEventListener('pointerdown', e => {
  if (!gameActive()) return;
  aiming = true;
  aimAt(pointerPos(e));
});
canvas.addEventListener('pointermove', e => {
  if (aiming && gameActive()) aimAt(pointerPos(e));
});
function release() {
  if (!aiming) return;
  aiming = false;
  if (gameActive()) fireBall();
}
canvas.addEventListener('pointerup', release);
canvas.addEventListener('pointercancel', () => { aiming = false; });

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
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-lose-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-lose-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

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
