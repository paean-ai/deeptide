// Pixel Street Brawl - a single-lane beat-'em-up: punch, kick, combo through
// escalating waves of street thugs, with a boss every fifth wave.

const BEST_KEY = 'pixel-street-brawl-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();
const keys = {};

const rand = (a, b) => a + Math.random() * (b - a);

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = {
    player: { x: VW / 2, y: GROUND, vy: 0, hp: PLAYER.maxHp, face: 1,
      atk: null, atkT: 0, atkCd: 0, combo: 0, comboT: 0, hurtT: 0, invuln: 0, kx: 0 },
    enemies: [], parts: [], hearts: [],
    wave: 0, score: 0, banner: null, spawnCd: 0, spawnQueue: [], t: 0, over: false,
  };
  startWave();
}

function startWave() {
  game.wave++;
  const boss = game.wave % BOSS_EVERY === 0;
  game.spawnQueue = (boss ? bossWaveEnemies : waveEnemies)(game.wave).slice();
  game.spawnCd = 0.4;
  setBanner(boss ? t('bossBanner') : t('waveBanner', game.wave), boss ? '#ff6b6b' : '#ffd24d');
}

function spawnEnemy(type) {
  const d = ENEMIES[type], s = enemyScale(game.wave);
  const fromLeft = Math.random() < 0.5;
  game.enemies.push({
    type, x: fromLeft ? -28 : VW + 28, y: GROUND,
    hp: Math.round(d.hp * s), maxHp: Math.round(d.hp * s),
    atkState: null, atkT: 0, atkCd: rand(0.4, 1.1), hurtT: 0, kx: 0, dead: false,
    face: fromLeft ? 1 : -1,
  });
}

function setBanner(text, color) { game.banner = { text, color, life: 1.4 }; }

// ---- player actions ----------------------------------------------------
function attack(kind) {
  const p = game.player;
  if (!game || game.over || p.atk || p.hurtT > 0 || p.atkCd > 0 || !overlaysClosed()) return;
  if (p.y < GROUND - 2 && kind === 'punch') return;          // no air punch
  const def = PLAYER[kind];
  p.atk = kind; p.atkT = 0; p.atkCd = def.cd;
  let dmg = def.dmg;
  if (kind === 'punch') {
    if (p.comboT > 0) p.combo++; else p.combo = 1;
    p.comboT = PLAYER.comboWindow;
    if (p.combo >= 3) { dmg = Math.round(dmg * 1.8); setBanner(t('combo') + ' x' + p.combo, '#ffd24d'); p.combo = 0; }
  } else { p.combo = 0; }
  hitFront(def.range, dmg, def.knock);
}

function hitFront(range, dmg, knock) {
  const p = game.player;
  for (const e of game.enemies) {
    if (e.dead) continue;
    const dx = (e.x - p.x) * p.face;
    if (dx > -8 && dx < range && Math.abs(e.y - p.y) < 30) {
      e.hp -= dmg;
      e.hurtT = 0.22;
      e.atkState = null;
      e.kx = p.face * knock;
      burst(e.x, GROUND - 30, ENEMIES[e.type].color, 9);
      if (e.hp <= 0 && !e.dead) killEnemy(e);
    }
  }
}

function killEnemy(e) {
  e.dead = true;
  game.score += Math.round(ENEMIES[e.type].score * (1 + game.wave * 0.1));
  burst(e.x, GROUND - 30, '#ffe9a0', 20);
  if (Math.random() < HEAL_DROP_CHANCE) game.hearts.push({ x: e.x, y: GROUND - 12, life: 9 });
}

function jump() {
  const p = game.player;
  if (!game || game.over || !overlaysClosed()) return;
  if (p.y >= GROUND - 2 && !p.hurtT) p.vy = -PLAYER.jump;
}

function hurtPlayer(dmg, fromX) {
  const p = game.player;
  if (p.invuln > 0) return;
  p.hp -= dmg;
  p.invuln = 0.8;
  p.hurtT = 0.4;
  p.atk = null;
  p.combo = 0;
  p.kx = (p.x < fromX ? -1 : 1) * 165;
  burst(p.x, GROUND - 30, '#ff6b6b', 12);
  if (p.hp <= 0) { p.hp = 0; gameOver(); }
}

function burst(x, y, color, n) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * 6.28, sp = rand(40, 200);
    game.parts.push({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 50, life: 1, color });
  }
}

function gameOver() {
  if (game.over) return;
  game.over = true;
  if (game.score > bestScore()) localStorage.setItem(BEST_KEY, game.score);
  document.getElementById('over-score').textContent = t('finalScore', game.score);
  document.getElementById('over-wave').textContent = t('reachedWave', game.wave);
  document.getElementById('over-best').textContent = t('bestScore', bestScore());
  showOverlay('overlay-over');
}

// ---- update ------------------------------------------------------------
function update(dt) {
  const g = game, p = g.player;
  g.t += dt;
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }

  // --- player ---
  p.atkCd -= dt; p.comboT -= dt; p.hurtT -= dt; p.invuln -= dt;
  if (p.atk) { p.atkT += dt; if (p.atkT >= PLAYER[p.atk].dur) p.atk = null; }
  const locked = p.atk || p.hurtT > 0;
  let dir = 0;
  if (keys.ArrowLeft || keys.a || keys.A) dir -= 1;
  if (keys.ArrowRight || keys.d || keys.D) dir += 1;
  if (!locked && p.y >= GROUND - 2) {
    p.x += dir * PLAYER.speed * dt;
    if (dir) p.face = dir;
  }
  p.x += p.kx * dt; p.kx *= Math.pow(0.02, dt);
  p.y += p.vy * dt; p.vy += PLAYER.gravity * dt;
  if (p.y > GROUND) { p.y = GROUND; p.vy = 0; }
  p.x = Math.max(16, Math.min(VW - 16, p.x));

  // --- enemies ---
  for (const e of g.enemies) {
    if (e.dead) continue;
    e.hurtT -= dt; e.atkCd -= dt;
    e.x += e.kx * dt; e.kx *= Math.pow(0.02, dt);
    const def = ENEMIES[e.type];
    const dx = p.x - e.x;
    e.face = dx >= 0 ? 1 : -1;
    if (e.atkState === 'windup') {
      e.atkT += dt;
      if (e.atkT >= def.windup) {
        if (Math.abs(p.x - e.x) < def.range + 8 && p.invuln <= 0) hurtPlayer(Math.round(def.dmg * enemyScale(g.wave)), e.x);
        e.atkState = null;
        e.atkCd = def.atkCd;
      }
    } else if (e.hurtT <= 0) {
      if (Math.abs(dx) > def.range - 4) {
        e.x += e.face * def.speed * dt;
      } else if (e.atkCd <= 0 && !g.over) {
        e.atkState = 'windup'; e.atkT = 0;
      }
    }
    e.x = Math.max(-40, Math.min(VW + 40, e.x));
  }
  g.enemies = g.enemies.filter(e => !e.dead);

  // --- spawning + wave flow ---
  if (g.spawnQueue.length) {
    g.spawnCd -= dt;
    const onScreen = g.enemies.length;
    if (g.spawnCd <= 0 && onScreen < 5) {
      spawnEnemy(g.spawnQueue.shift());
      g.spawnCd = rand(0.7, 1.5);
    }
  } else if (!g.enemies.length && !g.over) {
    setBanner(t('cleared'), '#7dff9f');
    startWave();
  }

  // --- hearts / particles ---
  for (const h of g.hearts) {
    h.life -= dt;
    if (Math.abs(h.x - p.x) < 24 && p.y >= GROUND - 30) {
      h.dead = true;
      p.hp = Math.min(PLAYER.maxHp, p.hp + HEAL_AMOUNT);
      burst(p.x, GROUND - 30, '#7dff9f', 8);
    }
  }
  g.hearts = g.hearts.filter(h => !h.dead && h.life > 0);
  for (const pt of g.parts) { pt.x += pt.vx * dt; pt.y += pt.vy * dt; pt.vy += 420 * dt; pt.life -= dt * 1.6; }
  g.parts = g.parts.filter(pt => pt.life > 0);
}

// ---- render ------------------------------------------------------------
function playerPose(p) {
  if (p.hurtT > 0) return 'hurt';
  if (p.atk) return p.atk;
  if (p.y < GROUND - 3) return 'jump';
  return (keys.ArrowLeft || keys.a || keys.A || keys.ArrowRight || keys.d || keys.D) ? 'walk' : 'idle';
}

function render() {
  const g = game;
  drawStreet(ctx, g.t);
  for (const h of g.hearts) {
    if (Math.floor(g.t * 6) % 4 === 0 && h.life < 3) continue;   // blink before vanish
    ctx.fillStyle = '#ff6b8b';
    ctx.fillRect(h.x - 7, h.y - 7, 14, 14);
    ctx.fillStyle = '#ffd0da';
    ctx.fillRect(h.x - 5, h.y - 5, 5, 4);
  }
  // draw fighters back-to-front by foot y (all on ground, so by x is fine)
  const fighters = [];
  for (const e of g.enemies) {
    const def = ENEMIES[e.type];
    fighters.push({ z: e.y, draw: () => {
      const flash = e.hurtT > 0 && Math.floor(g.t * 30) % 2;
      drawFighter(ctx, e.x, e.y, e.face, flash ? '#ffffff' : def.color,
        e.hurtT > 0 ? 'hurt' : e.atkState ? 'attack' : 'walk', e.atkT, g.t, def.boss ? 1.35 : 1);
      // hp pip
      const f = Math.max(0, e.hp / e.maxHp);
      const w = def.boss ? 60 : 30;
      ctx.fillStyle = '#10131c'; ctx.fillRect(e.x - w / 2, e.y - 66 * (def.boss ? 1.35 : 1), w, 5);
      ctx.fillStyle = f > 0.4 ? '#7dff9f' : '#ff6b6b';
      ctx.fillRect(e.x - w / 2 + 1, e.y - 66 * (def.boss ? 1.35 : 1) + 1, (w - 2) * f, 3);
    } });
  }
  if (!g.over) {
    const p = g.player;
    fighters.push({ z: p.y + 0.5, draw: () => {
      if (p.invuln > 0 && Math.floor(g.t * 20) % 2) return;
      drawFighter(ctx, p.x, p.y, p.face, '#3f8fd0', playerPose(p), p.atkT, g.t, 1);
    } });
  }
  fighters.sort((a, b) => a.z - b.z);
  for (const f of fighters) f.draw();

  for (const pt of g.parts) {
    ctx.globalAlpha = Math.max(0, pt.life);
    ctx.fillStyle = pt.color;
    ctx.fillRect(pt.x | 0, pt.y | 0, 4, 4);
  }
  ctx.globalAlpha = 1;
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life);
    ctx.fillStyle = g.banner.color;
    ctx.font = '900 26px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, VW / 2, 60);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function updateHud() {
  const p = game.player;
  document.getElementById('hud-wave').textContent = `${t('wave')} ${game.wave}`;
  document.getElementById('hud-score').textContent = game.score;
  const f = Math.max(0, p.hp / PLAYER.maxHp);
  document.getElementById('hp-fill').style.width = (f * 100) + '%';
  document.getElementById('hp-fill').style.background = f > 0.4 ? '#7dff9f' : '#ff6b6b';
}

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
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  keys[e.key] = true;
  const k = e.key.toLowerCase();
  if (k === 'j') attack('punch');
  else if (k === 'k') attack('kick');
  else if (k === 'l' || e.key === 'ArrowUp' || k === 'w') jump();
  else if (e.key === 'Escape') togglePause();
  if (['ArrowLeft', 'ArrowRight', 'ArrowUp', ' '].includes(e.key)) e.preventDefault();
});
addEventListener('keyup', e => { keys[e.key] = false; });

function holdBtn(id, key) {
  const el = document.getElementById(id);
  const on = e => { e.preventDefault(); keys[key] = true; };
  const off = e => { e.preventDefault(); keys[key] = false; };
  el.addEventListener('pointerdown', on);
  el.addEventListener('pointerup', off);
  el.addEventListener('pointerleave', off);
  el.addEventListener('pointercancel', off);
}
holdBtn('btn-left', 'ArrowLeft');
holdBtn('btn-right', 'ArrowRight');
document.getElementById('btn-punch').addEventListener('pointerdown', e => { e.preventDefault(); attack('punch'); });
document.getElementById('btn-kick').addEventListener('pointerdown', e => { e.preventDefault(); attack('kick'); });
document.getElementById('btn-jump').addEventListener('pointerdown', e => { e.preventDefault(); jump(); });

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
  const dt = Math.min(0.045, (now - lastT) / 1000);
  lastT = now;
  if (game && !game.over && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    update(dt);
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
