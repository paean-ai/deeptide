// Pixel Fruit Slash - swipe slicing, spawning, combos, lives.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-fruit-slash-best';
let best = parseInt(localStorage.getItem(SAVE_KEY) || '0', 10) || 0;

let state = null;
const blade = { down: false, points: [], px: 0, py: 0, sliced: 0 };
let tick = 0;

// ---- setup ---------------------------------------------------------------
function newGame() {
  state = {
    status: 'play', score: 0, lives: 3, time: 0,
    objects: [], halves: [], particles: [], popups: [],
    spawnT: 0.6, flash: 0,
  };
  blade.down = false;
  blade.points = [];
  blade.sliced = 0;
  updateHud();
}

function spawnObject() {
  const time = state.time;
  const x = 44 + Math.random() * (VW - 88);
  const isBomb = Math.random() < bombChance(time);
  const isGold = !isBomb && Math.random() < goldChance(time);
  const vy = -(580 + Math.random() * 140);
  const vx = (VW / 2 - x) * (0.4 + Math.random() * 0.5) + (Math.random() - 0.5) * 70;
  const def = isBomb ? BOMB : isGold ? GOLD : FRUITS[(Math.random() * FRUITS.length) | 0];
  state.objects.push({
    kind: isBomb ? 'bomb' : isGold ? 'gold' : 'fruit', def, x, y: VH + def.r + 8,
    vx, vy, r: def.r, rot: 0, vrot: (Math.random() - 0.5) * 5, t: 0, sliced: false,
  });
}

// ---- slicing -------------------------------------------------------------
function segHitsCircle(x1, y1, x2, y2, cx, cy, r) {
  const dx = x2 - x1, dy = y2 - y1;
  const L2 = dx * dx + dy * dy;
  let s = L2 > 0 ? ((cx - x1) * dx + (cy - y1) * dy) / L2 : 0;
  s = Math.max(0, Math.min(1, s));
  const px = x1 + dx * s, py = y1 + dy * s;
  return (px - cx) ** 2 + (py - cy) ** 2 <= r * r;
}

function sliceSegment(x1, y1, x2, y2) {
  if ((x2 - x1) ** 2 + (y2 - y1) ** 2 < 9) return;
  for (const o of state.objects) {
    if (o.sliced) continue;
    if (segHitsCircle(x1, y1, x2, y2, o.x, o.y, o.r + 4)) sliceObject(o, x2 - x1, y2 - y1);
  }
}

function sliceObject(o, sdx, sdy) {
  o.sliced = true;
  if (o.kind === 'bomb') {
    burst(o.x, o.y, 30, '#ff8f4a');
    burst(o.x, o.y, 20, '#ffd23f');
    state.flash = 1;
    gameOver('boom');
    return;
  }
  blade.sliced++;
  const base = 10 + Math.floor(state.time / 15) * 2;
  const points = o.kind === 'gold' ? base * GOLD_MULT : base;
  state.score += points;
  // Extra particle splash for a gold slice.
  burst(o.x, o.y, o.kind === 'gold' ? 20 : 11, o.def.inner);
  if (o.kind === 'gold') burst(o.x, o.y, 14, '#fff0c8');
  const sl = Math.hypot(sdx, sdy) || 1;
  const nx = sdx / sl, ny = sdy / sl;
  for (const side of ['L', 'R']) {
    const dir = side === 'L' ? -1 : 1;
    state.halves.push({
      x: o.x, y: o.y,
      vx: o.vx * 0.4 - ny * dir * (90 + Math.random() * 70),
      vy: o.vy * 0.45 + nx * dir * (90 + Math.random() * 70) - 40,
      rot: o.rot, vrot: dir * (4 + Math.random() * 4),
      color: o.def.color, inner: o.def.inner, r: o.r, side, life: 1.35,
    });
  }
  updateHud();
}

function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, sp = 50 + Math.random() * 190;
    state.particles.push({
      x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 30,
      life: 0.35 + Math.random() * 0.5, color,
    });
  }
}

function loseLife() {
  state.lives--;
  state.popups.push({ x: VW / 2, y: VH - 70, text: '✕', life: 0.9, color: '#ff6b6b' });
  updateHud();
  if (state.lives <= 0) gameOver('miss');
}

function gameOver(reason) {
  if (state.status !== 'play') return;
  state.status = 'over';
  if (state.score > best) {
    best = state.score;
    try { localStorage.setItem(SAVE_KEY, String(best)); } catch (e) { /* ignore */ }
  }
  setTimeout(() => {
    document.getElementById('over-title').textContent = reason === 'boom' ? t('boom') : t('gameOver');
    document.getElementById('final-score').textContent = t('finalScore', state.score);
    document.getElementById('final-best').textContent =
      (state.score >= best && state.score > 0 ? t('newBest') + '  ' : '') + t('best') + ' ' + best;
    showScreen('screen-over');
  }, 850);
}

// ---- update --------------------------------------------------------------
function update(dt) {
  tick += dt;
  for (const p of blade.points) p.life -= dt * 3;
  blade.points = blade.points.filter(p => p.life > 0);

  for (const pa of state.particles) {
    pa.life -= dt;
    pa.x += pa.vx * dt;
    pa.y += pa.vy * dt;
    pa.vy += 360 * dt;
    pa.vx *= 0.95;
  }
  state.particles = state.particles.filter(pa => pa.life > 0);
  for (const pp of state.popups) { pp.life -= dt; pp.y -= 34 * dt; }
  state.popups = state.popups.filter(pp => pp.life > 0);
  for (const h of state.halves) {
    h.life -= dt * 0.78;
    h.vy += GRAVITY * dt;
    h.x += h.vx * dt;
    h.y += h.vy * dt;
    h.rot += h.vrot * dt;
  }
  state.halves = state.halves.filter(h => h.life > 0);

  if (state.flash > 0) state.flash = Math.max(0, state.flash - dt * 2);
  if (state.status !== 'play') return;

  state.time += dt;
  state.spawnT -= dt;
  if (state.spawnT <= 0) {
    const n = waveSize(state.time);
    for (let i = 0; i < n; i++) spawnObject();
    state.spawnT = spawnInterval(state.time);
  }

  for (const o of state.objects) {
    o.vy += GRAVITY * dt;
    o.x += o.vx * dt;
    o.y += o.vy * dt;
    o.rot += o.vrot * dt;
    o.t += dt;
    if (o.y > VH + o.r * 2 && o.vy > 0) {
      o.gone = true;
      if (o.kind === 'fruit' && !o.sliced) loseLife();
    } else if (o.x < -80 || o.x > VW + 80) {
      o.gone = true;
    }
  }
  state.objects = state.objects.filter(o => !o.gone && !o.sliced);
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;

  for (const o of state.objects) {
    if (o.kind === 'bomb') drawBomb(ctx, o, tick);
    else {
      ctx.save();
      ctx.translate(o.x, o.y);
      ctx.rotate(o.rot);
      ctx.translate(-o.x, -o.y);
      drawFruit(ctx, o);
      ctx.restore();
    }
  }
  for (const h of state.halves) drawHalf(ctx, h);

  for (const pa of state.particles) {
    ctx.globalAlpha = Math.min(1, pa.life * 2.5);
    ctx.fillStyle = pa.color;
    ctx.fillRect(pa.x | 0, pa.y | 0, 4, 4);
  }
  ctx.globalAlpha = 1;

  drawBlade(ctx, blade.points);

  ctx.textAlign = 'center';
  for (const pp of state.popups) {
    ctx.globalAlpha = Math.min(1, pp.life * 2);
    ctx.fillStyle = pp.color;
    ctx.font = '900 20px ui-monospace, monospace';
    ctx.fillText(pp.text, pp.x, pp.y);
  }
  ctx.globalAlpha = 1;
  ctx.textAlign = 'left';

  if (state.flash > 0) {
    ctx.fillStyle = 'rgba(255, 120, 60, ' + (state.flash * 0.55) + ')';
    ctx.fillRect(0, 0, VW, VH);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-score').textContent = t('score') + ' ' + state.score;
  document.getElementById('hud-best').textContent = t('best') + ' ' + Math.max(best, state.score);
  document.getElementById('hud-lives').textContent = '♥'.repeat(Math.max(0, state.lives)) || '—';
}

// ---- input ---------------------------------------------------------------
function pointerPos(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
function addPoint(x, y) {
  blade.points.push({ x, y, life: 1 });
  if (blade.points.length > 16) blade.points.shift();
}
function gameActive() {
  return state && state.status === 'play' &&
    !document.getElementById('screen-game').classList.contains('hidden');
}
canvas.addEventListener('pointerdown', e => {
  if (!gameActive()) return;
  const p = pointerPos(e);
  blade.down = true;
  blade.sliced = 0;
  blade.points = [];
  blade.px = p.x;
  blade.py = p.y;
  addPoint(p.x, p.y);
});
canvas.addEventListener('pointermove', e => {
  if (!blade.down || !gameActive()) return;
  const p = pointerPos(e);
  addPoint(p.x, p.y);
  sliceSegment(blade.px, blade.py, p.x, p.y);
  blade.px = p.x;
  blade.py = p.y;
});
function endStroke() {
  if (!blade.down) return;
  blade.down = false;
  if (state && state.status === 'play' && blade.sliced >= 3) {
    const bonus = blade.sliced * blade.sliced * 6;
    state.score += bonus;
    state.popups.push({
      x: blade.px, y: blade.py, text: t('comboLabel', blade.sliced) + ' +' + bonus,
      life: 1.1, color: '#f2cf3f',
    });
    updateHud();
  }
}
canvas.addEventListener('pointerup', endStroke);
canvas.addEventListener('pointercancel', endStroke);

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function refreshTitle() {
  document.getElementById('title-best').textContent = best > 0 ? t('best') + ' ' + best : '';
}
document.getElementById('btn-play').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-again').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };

setupLanguageToggle(() => { refreshTitle(); updateHud(); });

// ---- main loop -----------------------------------------------------------
let last = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  if (state) update(dt);
  render();
  requestAnimationFrame(loop);
}
refreshTitle();
showScreen('screen-title');
requestAnimationFrame(loop);
