// Pixel Bullet Storm - bullet-hell loop: patterns, dodging, bombs, waves.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-bullet-storm-best';
let best = parseInt(localStorage.getItem(SAVE_KEY) || '0', 10) || 0;
function saveBest() {
  if (state.score > best) {
    best = state.score;
    try { localStorage.setItem(SAVE_KEY, String(best)); } catch (e) { /* ignore */ }
  }
}

const stars = makeStars(70);
let state = null;
const keys = {};
let dragging = false;
let flameTick = 0;

// ---- setup ---------------------------------------------------------------
function newGame() {
  state = {
    status: 'play', score: 0, wave: 0, lives: 3, bombs: 3, kills: 0,
    player: { x: VW / 2, y: VH - 96, invuln: 1.4, fireT: 0 },
    enemies: [], boss: null, bullets: [], shots: [], particles: [],
    spawnQueue: [], waveTime: 0, waveState: 'clear-delay', clearT: 0.7,
    banner: null, bombFlash: 0,
  };
  startWave(1);
  state.waveState = 'spawning';
  updateHud();
  updateControls();
}

function powerLevel() { return Math.min(3, Math.floor(state.kills / 12)); }

function startWave(n) {
  state.wave = n;
  state.waveTime = 0;
  state.waveState = 'spawning';
  const plan = wavePlan(n);
  if (plan.boss) {
    state.boss = {
      def: BOSS, x: VW / 2, y: -44, hp: plan.hp, maxhp: plan.hp,
      t: 0, fireT: 1.4, spin: 0, phase: 0, patIdx: 0, hitFlash: 0, enter: true,
    };
    setBanner(t('bossWarning'), 1.7);
  } else {
    state.spawnQueue = plan.spawns.slice().sort((a, b) => a.delay - b.delay);
  }
  updateHud();
}

function setBanner(text, time) { state.banner = { text, time, max: time }; }

// ---- bullets / patterns --------------------------------------------------
function pushBullet(x, y, ang, spd, color) {
  state.bullets.push({
    x, y, vx: Math.cos(ang) * spd, vy: Math.sin(ang) * spd,
    r: 4, color, grazed: false,
  });
}
function emitRing(x, y, n, off, spd, color) {
  for (let i = 0; i < n; i++) pushBullet(x, y, off + i / n * Math.PI * 2, spd, color);
}
function emitAimed(x, y, n, spread, spd, color) {
  const base = Math.atan2(state.player.y - y, state.player.x - x);
  for (let i = 0; i < n; i++) {
    const d = n > 1 ? (i / (n - 1) - 0.5) * spread : 0;
    pushBullet(x, y, base + d, spd, color);
  }
}

function enemyFire(e) {
  const spd = bulletSpeed(state.wave);
  const col = e.def.color;
  if (e.def.pattern === 'aimed') emitAimed(e.x, e.y, 3, 0.42, spd, col);
  else if (e.def.pattern === 'ring') emitRing(e.x, e.y, 14, e.t, spd, col);
  else if (e.def.pattern === 'fan') emitAimed(e.x, e.y, 5, 1.0, spd * 0.95, col);
  else { // spiral
    e.spin += 0.4;
    pushBullet(e.x, e.y, e.spin, spd, col);
    pushBullet(e.x, e.y, e.spin + Math.PI, spd, col);
  }
}

function bossFire(b) {
  const spd = bulletSpeed(state.wave);
  if (b.phase === 0) {
    emitRing(b.x, b.y, 16, b.spin, spd, '#ff9bb5');
    b.spin += 0.32;
    if (b.patIdx % 2 === 0) emitAimed(b.x, b.y, 5, 0.7, spd * 0.9, '#ffd23f');
    b.patIdx++;
    b.fireT = 0.82;
  } else if (b.phase === 1) {
    emitRing(b.x, b.y, 22, b.spin, spd, '#ff9bb5');
    b.spin += 0.5;
    emitAimed(b.x, b.y, 5, 0.55, spd, '#ffd23f');
    b.fireT = 0.66;
  } else {
    for (let arm = 0; arm < 4; arm++) {
      pushBullet(b.x, b.y, b.spin + arm * Math.PI / 2, spd * 1.05, '#ff9bb5');
    }
    b.spin += 0.46;
    b.patIdx++;
    if (b.patIdx % 14 === 0) emitRing(b.x, b.y, 16, 0, spd * 0.8, '#ffd23f');
    b.fireT = 0.13;
  }
}

// ---- particles -----------------------------------------------------------
function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, s = 40 + Math.random() * 160;
    state.particles.push({
      x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s,
      life: 0.4 + Math.random() * 0.5, color,
    });
  }
}

// ---- player actions ------------------------------------------------------
function playerFire() {
  const p = state.player, lv = powerLevel(), y = p.y - 10;
  const up = -470;
  if (lv === 0) {
    state.shots.push({ x: p.x, y, vx: 0, vy: up });
  } else if (lv === 1) {
    state.shots.push({ x: p.x - 5, y, vx: 0, vy: up });
    state.shots.push({ x: p.x + 5, y, vx: 0, vy: up });
  } else if (lv === 2) {
    state.shots.push({ x: p.x, y, vx: 0, vy: up });
    state.shots.push({ x: p.x - 7, y, vx: -70, vy: up });
    state.shots.push({ x: p.x + 7, y, vx: 70, vy: up });
  } else {
    state.shots.push({ x: p.x - 5, y, vx: 0, vy: up });
    state.shots.push({ x: p.x + 5, y, vx: 0, vy: up });
    state.shots.push({ x: p.x - 9, y, vx: -130, vy: up });
    state.shots.push({ x: p.x + 9, y, vx: 130, vy: up });
  }
}

function doBomb() {
  if (!state || state.status !== 'play' || state.bombs <= 0) return;
  state.bombs--;
  state.bombFlash = 1;
  state.player.invuln = Math.max(state.player.invuln, 1.5);
  for (const b of state.bullets) burst(b.x, b.y, 1, b.color);
  state.bullets.length = 0;
  for (const e of state.enemies) { e.hp -= 4; e.hitFlash = 0.12; }
  if (state.boss) { state.boss.hp -= 32; state.boss.hitFlash = 0.12; }
  resolveDeaths();
  updateControls();
}

function playerHit() {
  const p = state.player;
  if (p.invuln > 0) return;
  state.lives--;
  burst(p.x, p.y, 22, '#ff5fa8');
  for (const b of state.bullets) burst(b.x, b.y, 1, b.color);
  state.bullets.length = 0;
  updateHud();
  if (state.lives <= 0) { gameOver(); return; }
  p.invuln = 2.4;
  p.x = VW / 2;
  p.y = VH - 96;
}

// ---- death resolution ----------------------------------------------------
function resolveDeaths() {
  for (let i = state.enemies.length - 1; i >= 0; i--) {
    const e = state.enemies[i];
    if (e.hp <= 0) {
      state.score += e.def.score;
      state.kills++;
      burst(e.x, e.y, 14, e.def.color);
      state.enemies.splice(i, 1);
    }
  }
  if (state.boss && state.boss.hp <= 0) {
    const b = state.boss;
    state.score += b.def.score;
    burst(b.x, b.y, 60, '#ff6b6b');
    state.boss = null;
    state.waveState = 'clear-delay';
    state.clearT = 1.6;
    setBanner(t('waveClear'), 1.4);
  }
  updateHud();
}

function gameOver() {
  state.status = 'over';
  saveBest();
  burst(state.player.x, state.player.y, 40, '#9fd6ff');
  setTimeout(() => {
    document.getElementById('final-score').textContent = t('finalLine', state.score, state.wave);
    document.getElementById('final-best').textContent =
      (state.score >= best && state.score > 0 ? t('newBest') + '  ' : '') + t('bestLine', best);
    showScreen('screen-over');
  }, 900);
}

// ---- update --------------------------------------------------------------
function update(dt) {
  for (const s of stars) {
    s.y += s.sp * dt;
    if (s.y > VH) { s.y = 0; s.x = Math.random() * VW; }
  }
  if (!state || state.status !== 'play') return;

  const p = state.player;
  flameTick += dt;
  if (p.invuln > 0) p.invuln -= dt;
  if (state.bombFlash > 0) state.bombFlash = Math.max(0, state.bombFlash - dt * 2.2);
  if (state.banner) { state.banner.time -= dt; if (state.banner.time <= 0) state.banner = null; }

  // movement
  if (!dragging) {
    const sp = 244 * dt;
    if (keys.left) p.x -= sp;
    if (keys.right) p.x += sp;
    if (keys.up) p.y -= sp;
    if (keys.down) p.y += sp;
  }
  p.x = Math.max(14, Math.min(VW - 14, p.x));
  p.y = Math.max(40, Math.min(VH - 30, p.y));

  // auto-fire
  p.fireT -= dt;
  if (p.fireT <= 0) { playerFire(); p.fireT = 0.115; }

  // spawns
  if (state.waveState === 'spawning') {
    state.waveTime += dt;
    while (state.spawnQueue.length && state.spawnQueue[0].delay <= state.waveTime) {
      const s = state.spawnQueue.shift();
      const def = ENEMY_TYPES[s.type];
      state.enemies.push({
        type: s.type, def, x: s.x, y: -20, targetY: s.targetY, hp: def.hp,
        t: 0, fireT: 0.7 + Math.random() * 0.6, spin: Math.random() * 6.28,
        phase: Math.random() * 6.28, hitFlash: 0, enter: true,
      });
    }
    if (!state.boss && !state.spawnQueue.length && !state.enemies.length) {
      state.score += 220;
      state.waveState = 'clear-delay';
      state.clearT = 1.4;
      setBanner(t('waveClear'), 1.2);
    }
  } else if (state.waveState === 'clear-delay') {
    state.clearT -= dt;
    if (state.clearT <= 0) startWave(state.wave + 1);
  }

  // enemies
  for (const e of state.enemies) {
    e.t += dt;
    if (e.hitFlash > 0) e.hitFlash -= dt;
    if (e.enter) {
      e.y += e.def.speed * dt;
      if (e.y >= e.targetY) e.enter = false;
    } else {
      e.x += Math.cos(e.t * 1.3 + e.phase) * 48 * dt;
      e.x = Math.max(20, Math.min(VW - 20, e.x));
      e.fireT -= dt;
      if (e.fireT <= 0) {
        enemyFire(e);
        e.fireT = e.def.fireEvery * Math.max(0.55, 1 - state.wave * 0.025);
      }
    }
  }

  // boss
  const b = state.boss;
  if (b) {
    b.t += dt;
    if (b.hitFlash > 0) b.hitFlash -= dt;
    if (b.enter) {
      b.y += 70 * dt;
      if (b.y >= 86) b.enter = false;
    } else {
      b.x = VW / 2 + Math.sin(b.t * 0.7) * 92;
      const ratio = b.hp / b.maxhp;
      b.phase = ratio > 0.6 ? 0 : ratio > 0.3 ? 1 : 2;
      b.fireT -= dt;
      if (b.fireT <= 0) bossFire(b);
    }
  }

  // player shots
  for (const s of state.shots) { s.x += s.vx * dt; s.y += s.vy * dt; }
  state.shots = state.shots.filter(s => s.y > -12 && s.x > -12 && s.x < VW + 12);

  // shot vs enemy / boss
  for (const s of state.shots) {
    for (const e of state.enemies) {
      if (e.hp > 0 && Math.hypot(s.x - e.x, s.y - e.y) < e.def.r + 3) {
        e.hp--; e.hitFlash = 0.1; s.dead = true;
        burst(s.x, s.y, 2, '#bff4ff');
        break;
      }
    }
    if (s.dead) continue;
    if (b && b.hp > 0 && !b.enter && Math.hypot(s.x - b.x, s.y - b.y) < b.def.r) {
      b.hp--; b.hitFlash = 0.08; s.dead = true;
      burst(s.x, s.y, 2, '#bff4ff');
    }
  }
  state.shots = state.shots.filter(s => !s.dead);
  resolveDeaths();

  // enemy bullets
  for (const bl of state.bullets) { bl.x += bl.vx * dt; bl.y += bl.vy * dt; }
  state.bullets = state.bullets.filter(bl =>
    bl.x > -22 && bl.x < VW + 22 && bl.y > -22 && bl.y < VH + 22);

  // bullet vs player
  for (const bl of state.bullets) {
    const d = Math.hypot(bl.x - p.x, bl.y - p.y);
    if (d < bl.r + 3.2) { playerHit(); break; }
    if (d < bl.r + 13 && !bl.grazed) { bl.grazed = true; state.score += 20; }
  }

  // particles
  for (const pa of state.particles) {
    pa.life -= dt;
    pa.x += pa.vx * dt;
    pa.y += pa.vy * dt;
    pa.vx *= 0.93;
    pa.vy *= 0.93;
  }
  state.particles = state.particles.filter(pa => pa.life > 0);

  updateHud();
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx, VW, VH, stars);
  if (!state) return;

  for (const e of state.enemies) drawEnemy(ctx, e);
  if (state.boss) {
    drawBoss(ctx, state.boss);
    const w = VW - 40, ratio = Math.max(0, state.boss.hp / state.boss.maxhp);
    ctx.fillStyle = '#1a1a32';
    ctx.fillRect(20, 12, w, 7);
    ctx.fillStyle = '#ff6b6b';
    ctx.fillRect(20, 12, w * ratio, 7);
  }
  for (const s of state.shots) drawShot(ctx, s);
  for (const bl of state.bullets) drawBullet(ctx, bl);

  for (const pa of state.particles) {
    ctx.globalAlpha = Math.min(1, pa.life * 2.4);
    ctx.fillStyle = pa.color;
    ctx.fillRect(pa.x | 0, pa.y | 0, 3, 3);
  }
  ctx.globalAlpha = 1;

  const p = state.player;
  if (state.status === 'play') {
    const blink = p.invuln > 0 && (flameTick * 16 | 0) % 2 === 0;
    if (!blink) drawShip(ctx, p.x, p.y, (flameTick * 18 | 0) % 2 === 0);
    drawCore(ctx, p.x, p.y, p.invuln > 0);
  }

  // power pips
  const lv = powerLevel();
  for (let i = 0; i < 3; i++) {
    ctx.fillStyle = i < lv ? '#4fd6d6' : '#2a2a48';
    ctx.fillRect(8, VH - 56 - i * 7, 5, 5);
  }

  if (state.banner) {
    const a = Math.min(1, state.banner.time / state.banner.max * 2.4);
    ctx.globalAlpha = a;
    ctx.fillStyle = '#ff5fa8';
    ctx.font = '900 22px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(state.banner.text, VW / 2, VH / 2 - 40);
    ctx.globalAlpha = 1;
    ctx.textAlign = 'left';
  }
  if (state.bombFlash > 0) {
    ctx.fillStyle = 'rgba(79, 214, 214, ' + (state.bombFlash * 0.5) + ')';
    ctx.fillRect(0, 0, VW, VH);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-score').textContent = t('score') + ' ' + state.score;
  document.getElementById('hud-wave').textContent = t('wave') + ' ' + state.wave;
  document.getElementById('hud-lives').textContent =
    '♥'.repeat(Math.max(0, state.lives)) || '—';
}
function updateControls() {
  const btn = document.getElementById('btn-bomb');
  btn.textContent = 'BOMB ' + (state ? state.bombs : 0);
  btn.disabled = !state || state.bombs <= 0;
}

// ---- input ---------------------------------------------------------------
function pointerPos(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
function moveTo(e) {
  if (!state || state.status !== 'play') return;
  const pos = pointerPos(e);
  state.player.x = Math.max(14, Math.min(VW - 14, pos.x));
  state.player.y = Math.max(40, Math.min(VH - 30, pos.y - 46));
}
canvas.addEventListener('pointerdown', e => { dragging = true; moveTo(e); });
canvas.addEventListener('pointermove', e => { if (dragging) moveTo(e); });
canvas.addEventListener('pointerup', () => { dragging = false; });
canvas.addEventListener('pointercancel', () => { dragging = false; });

const KEYMAP = {
  ArrowLeft: 'left', a: 'left', A: 'left',
  ArrowRight: 'right', d: 'right', D: 'right',
  ArrowUp: 'up', w: 'up', W: 'up',
  ArrowDown: 'down', s: 'down', S: 'down',
};
addEventListener('keydown', e => {
  if (KEYMAP[e.key]) { keys[KEYMAP[e.key]] = true; e.preventDefault(); }
  if (e.key === ' ' && !e.repeat) { doBomb(); e.preventDefault(); }
});
addEventListener('keyup', e => { if (KEYMAP[e.key]) keys[KEYMAP[e.key]] = false; });

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
document.getElementById('btn-bomb').onclick = doBomb;

setupLanguageToggle(() => {
  refreshTitle();
  updateHud();
  updateControls();
});

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
