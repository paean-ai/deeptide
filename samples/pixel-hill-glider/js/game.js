// Pixel Hill Glider - run state, glide physics, orbs, scoring, save.

const BEST_KEY = 'pixel-hill-glider-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let run = null;
let best = +(localStorage.getItem(BEST_KEY) || 0);
let lastT = performance.now();

const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

// ---- run lifecycle -----------------------------------------------------
function newRun() {
  const y0 = terrainY(0);
  run = {
    worldX: 0, worldY: y0, vx: MIN_SPEED, vy: 0,
    grounded: true, diving: false,
    light: 1, combo: 0, fever: 0, orbScore: 0, orbsTaken: 0,
    camX: -BIRD_X, camY: y0 - VH * 0.5,
    flap: 0, t: 0, lastAlign: 1,
    orbs: [], nextOrb: 0, particles: [], over: false,
  };
  manageOrbs();
  updateHud();
}

function scoreMeters() { return Math.floor(run.worldX / 14) + Math.floor(run.orbScore); }

function endRun() {
  run.over = true;
  const total = scoreMeters();
  const isBest = total > best;
  if (isBest) { best = total; try { localStorage.setItem(BEST_KEY, best); } catch (e) {} }
  document.getElementById('final-dist').textContent = t('finalDist', total);
  document.getElementById('final-orbs').textContent = `${t('orbs')}: ${run.orbsTaken}`;
  document.getElementById('final-best').textContent = isBest ? t('newBest') : `${t('best')}: ${best}`;
  showScreen('screen-over');
  updateHud();
}

// ---- orbs --------------------------------------------------------------
function manageOrbs() {
  const r = run;
  while (true) {
    const p = orbPos(r.nextOrb);
    if (p.x < r.camX + VW + 280) { r.orbs.push({ x: p.x, y: p.y, taken: false }); r.nextOrb++; }
    else break;
  }
  r.orbs = r.orbs.filter(o => o.x > r.camX - 70);
}

// ---- particles ---------------------------------------------------------
function spawnDust(wx, wy, strength) {
  const n = 4 + Math.floor(strength * 6);
  for (let i = 0; i < n; i++) {
    const a = -Math.PI / 2 + (Math.random() - 0.5) * 2.2;
    const sp = 40 + Math.random() * 110 * (0.4 + strength);
    run.particles.push({
      x: wx, y: wy, vx: Math.cos(a) * sp - 60, vy: Math.sin(a) * sp,
      life: 0.5, max: 0.5, size: 2 + Math.random() * 3, color: '#d9c89a',
    });
  }
}
function spawnSpark(wx, wy) {
  for (let i = 0; i < 10; i++) {
    const a = Math.random() * Math.PI * 2, sp = 60 + Math.random() * 140;
    run.particles.push({
      x: wx, y: wy, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp,
      life: 0.5, max: 0.5, size: 2 + Math.random() * 3, color: '#ffe14d',
    });
  }
}
function updateParticles(dt) {
  for (const p of run.particles) {
    p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 240 * dt; p.life -= dt;
  }
  run.particles = run.particles.filter(p => p.life > 0);
}

// ---- physics -----------------------------------------------------------
function update(dt) {
  const r = run;
  r.t += dt;
  r.flap += dt * (r.grounded ? 7 : 11);

  r.light -= LIGHT_DRAIN * dt;
  if (r.light <= 0) { r.light = 0; endRun(); return; }

  if (r.grounded) {
    const slope = clamp(terrainSlope(r.worldX), -3, 3);
    const a = Math.atan(slope);
    let speed = Math.hypot(r.vx, r.vy);
    let acc = SLOPE_G * Math.sin(a);
    if (r.diving && slope > 0.02) acc += DIVE_SLIDE;
    speed += acc * dt;
    speed -= speed * GROUND_DRAG * dt;
    speed = clamp(speed, MIN_SPEED, MAX_SPEED);
    r.vx = speed * Math.cos(a);
    r.vy = speed * Math.sin(a);
  } else {
    r.vy += (r.diving ? DIVE_G : AIR_G) * dt;
  }

  r.worldX += r.vx * dt;
  r.worldY += r.vy * dt;

  const ty = terrainY(r.worldX);
  if (r.worldY >= ty) {
    if (!r.grounded) {
      // landing - redirect along the slope, bleed speed by misalignment
      const slope = clamp(terrainSlope(r.worldX), -3, 3);
      const a = Math.atan(slope);
      const speed = Math.hypot(r.vx, r.vy);
      const inAng = Math.atan2(r.vy, r.vx);
      const align = Math.max(0, Math.cos(inAng - a));
      const ns = Math.max(MIN_SPEED, speed * (0.34 + 0.66 * align));
      r.vx = ns * Math.cos(a);
      r.vy = ns * Math.sin(a);
      r.lastAlign = align;
      if (align < 0.55) { r.combo = 0; r.fever = 0; }
      spawnDust(r.worldX, ty, align);
    }
    r.worldY = ty;
    r.grounded = true;
  } else {
    if (r.grounded && Math.hypot(r.vx, r.vy) > 220) spawnDust(r.worldX, ty, 1);
    r.grounded = false;
  }

  r.camX = r.worldX - BIRD_X;
  r.camY += (r.worldY - VH * 0.5 - r.camY) * Math.min(1, 6 * dt);

  manageOrbs();
  for (const o of r.orbs) {
    if (o.taken) continue;
    if (Math.hypot(o.x - r.worldX, o.y - r.worldY) < 22) {
      o.taken = true;
      r.orbsTaken++;
      r.light = Math.min(1, r.light + ORB_LIGHT);
      r.combo++;
      r.fever = Math.min(5, Math.floor(r.combo / 3));
      r.orbScore += 14 * (1 + r.fever * 0.3);
      spawnSpark(o.x, o.y);
    }
  }

  updateParticles(dt);
  updateHud();
}

// ---- render ------------------------------------------------------------
function render() {
  const r = run;
  drawSky(ctx, clamp(r.light, 0, 1));
  drawHills(ctx, r.camX, r.camY, clamp(r.light, 0, 1));

  for (const o of r.orbs) {
    if (o.taken) continue;
    drawOrb(ctx, o.x - r.camX, o.y - r.camY, r.t);
  }
  for (const p of r.particles) {
    ctx.globalAlpha = Math.max(0, p.life / p.max);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x - r.camX - p.size / 2, p.y - r.camY - p.size / 2, p.size, p.size);
  }
  ctx.globalAlpha = 1;

  const ang = r.grounded
    ? Math.atan(clamp(terrainSlope(r.worldX), -3, 3))
    : Math.atan2(r.vy, r.vx);
  drawBird(ctx, BIRD_X, r.worldY - r.camY, ang, r.diving, r.flap);

  // light meter
  const mw = VW - 24;
  ctx.fillStyle = 'rgba(8,14,9,0.6)';
  ctx.fillRect(12, 12, mw, 9);
  ctx.fillStyle = r.light > 0.3 ? '#ffe14d' : '#ff6d4d';
  ctx.fillRect(12, 12, mw * clamp(r.light, 0, 1), 9);
  ctx.strokeStyle = 'rgba(255,255,255,0.45)';
  ctx.lineWidth = 1;
  ctx.strokeRect(12.5, 12.5, mw, 9);
  // fever pips
  for (let i = 0; i < 5; i++) {
    ctx.fillStyle = i < r.fever ? '#ff8a3d' : 'rgba(255,255,255,0.22)';
    ctx.fillRect(12 + i * 12, 26, 9, 6);
  }
}

function updateHud() {
  if (!run) return;
  document.getElementById('hud-dist').textContent = `${t('dist')} ${scoreMeters()}`;
  document.getElementById('hud-best').textContent = `${t('best')} ${Math.max(best, scoreMeters())}`;
}

// ---- screens -----------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}

// ---- input -------------------------------------------------------------
function setDive(on) { if (run && !run.over) run.diving = on; }
canvas.addEventListener('pointerdown', e => { e.preventDefault(); setDive(true); });
window.addEventListener('pointerup', () => setDive(false));
canvas.addEventListener('pointercancel', () => setDive(false));
window.addEventListener('keydown', e => {
  if (e.code === 'Space') { e.preventDefault(); setDive(true); }
});
window.addEventListener('keyup', e => { if (e.code === 'Space') setDive(false); });

document.getElementById('btn-play').onclick = () => { newRun(); showScreen('screen-game'); };
document.getElementById('btn-again').onclick = () => { newRun(); showScreen('screen-game'); };
document.getElementById('btn-menu').onclick = () => {
  document.getElementById('title-best').textContent = best ? `${t('best')}: ${best}` : '';
  showScreen('screen-title');
};
setupLanguageToggle(() => { if (run) updateHud(); });

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.034, (now - lastT) / 1000);
  lastT = now;
  if (run && !document.getElementById('screen-game').classList.contains('hidden') && !run.over) {
    update(dt);
  }
  if (run) render();
  requestAnimationFrame(loop);
}

document.getElementById('title-best').textContent = best ? `${t('best')}: ${best}` : '';
newRun();
showScreen('screen-title');
requestAnimationFrame(loop);
