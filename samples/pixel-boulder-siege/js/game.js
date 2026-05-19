// Pixel Boulder Siege - projectile physics, block settling, siege rounds.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-boulder-siege-best';
let best = parseInt(localStorage.getItem(SAVE_KEY) || '0', 10) || 0;

let state = null;
let particles = [];

// ---- setup ---------------------------------------------------------------
function newGame() {
  state = {
    status: 'play', phase: 'aiming', score: 0, round: 1, shots: 0,
    ents: [], proj: null, aim: { active: false, px: 0, py: 0 },
    banner: null, settleT: 0, nextT: 0,
  };
  particles = [];
  loadFortress(1);
  updateHud();
}

function loadFortress(round) {
  const f = generateFortress(round);
  state.ents = f.entities.map(c => ({
    col: c.col, type: c.type,
    hp: c.type === 'X' ? 1 : BLOCKS[c.type].hp,
    maxhp: c.type === 'X' ? 1 : BLOCKS[c.type].hp,
    x: cellX(c.col), y: cellTop(c.r), targetY: cellTop(c.r),
    vy: 0, dead: false, hitFlash: 0,
  }));
  settleTargets();
  for (const e of state.ents) e.y = e.targetY;
  state.shots = f.shots;
  state.proj = null;
  state.phase = 'aiming';
  updateHud();
}

// Per-column gravity compaction: things pack down to the ground, no gaps.
function settleTargets() {
  for (let c = 0; c < COLS; c++) {
    const col = state.ents.filter(e => e.col === c && !e.dead);
    col.sort((a, b) => b.y - a.y);
    for (let i = 0; i < col.length; i++) col[i].targetY = cellTop(ROWS - 1 - i);
  }
}

function setBanner(text) { state.banner = { text, t: 1.3, max: 1.3 }; }

function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, s = 40 + Math.random() * 150;
    particles.push({
      x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s - 40,
      life: 0.35 + Math.random() * 0.5, color,
    });
  }
}

// ---- projectile physics --------------------------------------------------
function launch() {
  const dx = state.aim.px - CANNON_X, dy = state.aim.py - (CANNON_Y - 6);
  const dist = Math.hypot(dx, dy);
  if (dist < 16) return false;
  const ang = Math.atan2(dy, Math.max(dx, 10));
  const power = Math.min(MAX_PULL, dist) / MAX_PULL;
  const speed = MIN_SPEED + power * (MAX_SPEED - MIN_SPEED);
  state.proj = {
    x: CANNON_X + Math.cos(ang) * 26, y: (CANNON_Y - 6) + Math.sin(ang) * 26,
    vx: Math.cos(ang) * speed, vy: Math.sin(ang) * speed,
    r: 8, age: 0, slowT: 0, active: true,
  };
  state.shots--;
  state.phase = 'firing';
  updateHud();
  return true;
}

function collideProj(p, e) {
  const cx = Math.max(e.x, Math.min(p.x, e.x + B));
  const cy = Math.max(e.y, Math.min(p.y, e.y + B));
  const dx = p.x - cx, dy = p.y - cy;
  const d2 = dx * dx + dy * dy;
  if (d2 >= p.r * p.r) return;

  if (e.type === 'X') {
    e.dead = true;
    burst(e.x + B / 2, e.y + B / 2, 16, '#5fb24a');
    state.score += 100 + state.round * 15;
    p.vx *= 0.88;
    p.vy *= 0.88;
    settleTargets();
    updateHud();
    return;
  }

  let d = Math.sqrt(d2), nx, ny;
  if (d < 0.001) { nx = 0; ny = -1; d = 0; }
  else { nx = dx / d; ny = dy / d; }
  p.x += nx * (p.r - d);
  p.y += ny * (p.r - d);
  const vn = p.vx * nx + p.vy * ny;
  if (vn < 0) {
    const rest = BLOCKS[e.type].bounce;
    p.vx -= (1 + rest) * vn * nx;
    p.vy -= (1 + rest) * vn * ny;
    p.vx *= 0.87;
    p.vy *= 0.87;
    e.hp -= (-vn) * 0.072;
    e.hitFlash = 0.09;
    burst(cx, cy, 3, e.type === 'G' ? '#bfe8f0' : '#caa15a');
    if (e.hp <= 0) {
      e.dead = true;
      burst(e.x + B / 2, e.y + B / 2, 12,
        e.type === 'G' ? '#bfe8f0' : e.type === 'S' ? '#9a9aa6' : '#9c6b3c');
      settleTargets();
    }
  }
}

function stepPhysics(dt) {
  const p = state.proj;
  if (!p || !p.active) return;
  const STEPS = 4, h = dt / STEPS;
  for (let s = 0; s < STEPS; s++) {
    p.vy += GRAVITY * h;
    p.x += p.vx * h;
    p.y += p.vy * h;
    if (p.x < p.r) { p.x = p.r; p.vx = Math.abs(p.vx) * 0.5; }
    if (p.x > VW - p.r) { p.x = VW - p.r; p.vx = -Math.abs(p.vx) * 0.5; }
    if (p.y > GROUND_Y - p.r) {
      p.y = GROUND_Y - p.r;
      p.vy = -Math.abs(p.vy) * 0.4;
      p.vx *= 0.78;
      if (Math.abs(p.vy) < 46) p.vy = 0;
    }
    for (const e of state.ents) {
      if (!e.dead) collideProj(p, e);
    }
  }
  p.age += dt;
  const spd = Math.hypot(p.vx, p.vy);
  p.slowT = spd < 32 ? p.slowT + dt : 0;
  if (p.slowT > 0.4 || p.age > 7 || p.y > VH + 40) endProjectile();
}

function endProjectile() {
  state.proj.active = false;
  state.phase = 'settling';
  state.settleT = 0;
}

// ---- entity settling -----------------------------------------------------
function updateEnts(dt) {
  for (const e of state.ents) {
    if (e.dead) continue;
    if (e.hitFlash > 0) e.hitFlash -= dt;
    if (e.y < e.targetY - 0.5) {
      e.vy += GRAVITY * dt;
      e.y += e.vy * dt;
      if (e.y >= e.targetY) { e.y = e.targetY; e.vy = 0; }
    } else {
      e.y = e.targetY;
      e.vy = 0;
    }
  }
}
function allRest() {
  return state.ents.every(e => e.dead || e.y >= e.targetY - 0.6);
}

// ---- round flow ----------------------------------------------------------
function evaluate() {
  const remaining = state.ents.filter(e => e.type === 'X' && !e.dead).length;
  if (remaining === 0) {
    state.score += 220 + Math.max(0, state.shots) * 60;
    state.round++;
    setBanner(t('roundClear'));
    state.phase = 'roundend';
    state.nextT = 1.4;
    updateHud();
  } else if (state.shots <= 0) {
    gameOver();
  } else {
    state.phase = 'aiming';
    state.proj = null;
  }
}

function gameOver() {
  state.status = 'over';
  if (state.score > best) {
    best = state.score;
    try { localStorage.setItem(SAVE_KEY, String(best)); } catch (e) { /* ignore */ }
  }
  setTimeout(() => {
    document.getElementById('final-score').textContent = t('finalLine', state.score, state.round);
    document.getElementById('final-best').textContent =
      (state.score >= best && state.score > 0 ? t('newBest') + '  ' : '') + t('bestLine', best);
    showScreen('screen-over');
  }, 850);
}

// ---- update --------------------------------------------------------------
function update(dt) {
  for (const pa of particles) {
    pa.life -= dt;
    pa.x += pa.vx * dt;
    pa.y += pa.vy * dt;
    pa.vy += 420 * dt;
    pa.vx *= 0.94;
  }
  particles = particles.filter(pa => pa.life > 0);

  if (!state || state.status !== 'play') return;
  if (state.banner) { state.banner.t -= dt; if (state.banner.t <= 0) state.banner = null; }
  updateEnts(dt);

  if (state.phase === 'firing') {
    stepPhysics(dt);
  } else if (state.phase === 'settling') {
    if (allRest()) {
      state.settleT += dt;
      if (state.settleT > 0.32) evaluate();
    }
  } else if (state.phase === 'roundend') {
    state.nextT -= dt;
    if (state.nextT <= 0) loadFortress(state.round);
  }
}

// ---- render --------------------------------------------------------------
function aimAngle() {
  if (state.phase === 'aiming' && state.aim.active) {
    const dx = state.aim.px - CANNON_X, dy = state.aim.py - (CANNON_Y - 6);
    if (Math.hypot(dx, dy) >= 16) return Math.atan2(dy, Math.max(dx, 10));
  }
  return -0.62;
}

function drawAimPreview() {
  const dx = state.aim.px - CANNON_X, dy = state.aim.py - (CANNON_Y - 6);
  const dist = Math.hypot(dx, dy);
  if (dist < 16) return;
  const ang = Math.atan2(dy, Math.max(dx, 10));
  const power = Math.min(MAX_PULL, dist) / MAX_PULL;
  const speed = MIN_SPEED + power * (MAX_SPEED - MIN_SPEED);
  let x = CANNON_X + Math.cos(ang) * 26, y = (CANNON_Y - 6) + Math.sin(ang) * 26;
  let vx = Math.cos(ang) * speed, vy = Math.sin(ang) * speed;
  ctx.fillStyle = 'rgba(255, 255, 255, 0.66)';
  for (let i = 0; i < 30; i++) {
    for (let s = 0; s < 3; s++) { vy += GRAVITY * 0.016; x += vx * 0.016; y += vy * 0.016; }
    if (y > GROUND_Y || x > VW || x < 0) break;
    if (i % 2 === 0) ctx.fillRect(x - 1.5, y - 1.5, 3, 3);
  }
  // power gauge by the cannon
  ctx.fillStyle = '#1a1018';
  ctx.fillRect(14, GROUND_Y - 60, 8, 44);
  ctx.fillStyle = power > 0.8 ? '#ff8f6e' : '#ffc46e';
  ctx.fillRect(14, GROUND_Y - 16 - 44 * power, 8, 44 * power);
}

function render() {
  drawBackground(ctx);
  if (!state) return;

  for (const e of state.ents) {
    if (e.dead) continue;
    if (e.type === 'X') drawGoblin(ctx, e);
    else {
      drawBlock(ctx, e);
      if (e.hitFlash > 0) {
        ctx.fillStyle = 'rgba(255, 255, 255, 0.4)';
        ctx.fillRect(e.x, e.y, B, B);
      }
    }
  }

  if (state.proj) drawProjectile(ctx, state.proj);
  drawCannon(ctx, aimAngle(), state.aim.active);
  if (state.phase === 'aiming' && state.aim.active) drawAimPreview();

  for (const pa of particles) {
    ctx.globalAlpha = Math.min(1, pa.life * 2.6);
    ctx.fillStyle = pa.color;
    ctx.fillRect(pa.x | 0, pa.y | 0, 3, 3);
  }
  ctx.globalAlpha = 1;

  if (state.banner) {
    ctx.globalAlpha = Math.min(1, state.banner.t / state.banner.max * 2.4);
    ctx.fillStyle = '#ffc46e';
    ctx.font = '900 22px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(state.banner.text, VW / 2, 150);
    ctx.globalAlpha = 1;
    ctx.textAlign = 'left';
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-score').textContent = t('score') + ' ' + state.score;
  document.getElementById('hud-round').textContent = t('round') + ' ' + state.round;
  document.getElementById('hud-shots').textContent =
    '●'.repeat(Math.max(0, state.shots)) || '—';
}

// ---- input ---------------------------------------------------------------
function pointerPos(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
canvas.addEventListener('pointerdown', e => {
  if (!state || state.status !== 'play' || state.phase !== 'aiming') return;
  const pos = pointerPos(e);
  state.aim.active = true;
  state.aim.px = pos.x;
  state.aim.py = pos.y;
});
canvas.addEventListener('pointermove', e => {
  if (!state || !state.aim.active) return;
  const pos = pointerPos(e);
  state.aim.px = pos.x;
  state.aim.py = pos.y;
});
function releaseAim() {
  if (!state || !state.aim.active) return;
  state.aim.active = false;
  if (state.phase === 'aiming') launch();
}
canvas.addEventListener('pointerup', releaseAim);
canvas.addEventListener('pointercancel', () => { if (state) state.aim.active = false; });

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function refreshTitle() {
  document.getElementById('title-best').textContent = best > 0 ? t('bestLine', best) : '';
}

document.getElementById('btn-play').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-again').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };
document.getElementById('btn-back').onclick = () => { refreshTitle(); showScreen('screen-title'); };

setupLanguageToggle(() => { refreshTitle(); updateHud(); });

// ---- main loop -----------------------------------------------------------
let last = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  update(dt);
  render();
  requestAnimationFrame(loop);
}
refreshTitle();
showScreen('screen-title');
requestAnimationFrame(loop);
