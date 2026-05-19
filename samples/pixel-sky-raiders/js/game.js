// Pixel Sky Raiders - vertical shoot-'em-up: auto-firing raider, enemy waves,
// bullet dodging, power-ups, bombs and bosses every fifth wave.

const BEST_KEY = 'pixel-sky-raiders-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();
const keys = {};
let pointer = null;          // {x,y} while dragging on the canvas

const rand = (a, b) => a + Math.random() * (b - a);
const dist2 = (ax, ay, bx, by) => (ax - bx) ** 2 + (ay - by) ** 2;

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = {
    wave: 0, score: 0, over: false,
    player: {
      x: VW / 2, y: VH - 90, hp: PLAYER.maxHp, weapon: 1,
      bombs: PLAYER.startBombs, shield: 0, invuln: 0, fireCd: 0, hit: 0,
    },
    bullets: [], ebullets: [], enemies: [], powerups: [], parts: [],
    boss: null, spawnList: [], spawnCd: 0,
    stars: makeStars(), banner: null, flash: 0, t: 0,
  };
  nextWave();
}

function makeStars() {
  const s = [];
  for (let i = 0; i < 70; i++) {
    const layer = i % 3;
    s.push({ x: Math.random() * VW, y: Math.random() * VH,
      spd: 30 + layer * 55, size: layer === 2 ? 2 : 1,
      c: layer === 2 ? '#dfe7f2' : layer === 1 ? '#8fa6c4' : '#52617e' });
  }
  return s;
}

function nextWave() {
  game.wave++;
  if (game.wave % BOSS_EVERY === 0) {
    spawnBoss();
    setBanner(t('bossIncoming'), '#ff7ad0');
  } else {
    game.spawnList = waveSpawns(game.wave);
    game.spawnCd = 0.6;
    setBanner(t('waveCleared', game.wave), '#7ad0ff');
  }
}

function setBanner(text, color) { game.banner = { text, color, life: 1.8 }; }

// ---- spawning ----------------------------------------------------------
function spawnEnemy(type) {
  const d = ENEMIES[type], s = depthScale(game.wave);
  const x = rand(44, VW - 44);
  game.enemies.push({
    type, x, y: -28, spawnX: x, r: d.r,
    hp: Math.round(d.hp * s), maxHp: Math.round(d.hp * s),
    speed: d.speed * (0.85 + game.wave * 0.02),
    fireCd: d.fire ? rand(0.6, d.fire) : 0,
    score: d.score, flash: 0, hover: 80 + Math.random() * 70,
  });
}

function spawnBoss() {
  const b = bossStats(game.wave);
  game.boss = {
    x: VW / 2, y: -60, r: b.r, hp: b.hp, maxHp: b.hp, score: b.score,
    fireCd: 2.4, pattern: 0, patternT: 0, flash: 0, dir: 1, entered: false,
  };
}

// ---- player ------------------------------------------------------------
function fire() {
  const p = game.player, w = WEAPONS[p.weapon];
  for (const sh of w.shots) {
    game.bullets.push({
      x: p.x + sh.dx, y: p.y - 14,
      vx: Math.sin(sh.a) * 540, vy: -Math.cos(sh.a) * 540, dmg: 5,
    });
  }
  p.fireCd = w.cd;
}

function useBomb() {
  if (!game || game.over || game.player.bombs <= 0) return;
  game.player.bombs--;
  game.ebullets = [];
  game.flash = 0.5;
  for (const e of game.enemies) { e.hp -= 70; e.flash = 0.12; }
  if (game.boss) { game.boss.hp -= 260; game.boss.flash = 0.12; }
  for (let i = 0; i < 40; i++) {
    game.parts.push({ x: rand(0, VW), y: rand(0, VH * 0.7), vx: rand(-60, 60), vy: rand(-60, 60),
      life: 1, color: '#ffd24d', size: 3 });
  }
}

function hurtPlayer(amt) {
  const p = game.player;
  if (p.invuln > 0 || p.shield > 0) return;
  p.hp -= amt;
  p.invuln = PLAYER.invulnTime;
  p.hit = 0.3;
  spawnParts(p.x, p.y, '#ff6b6b', 14);
  if (p.hp <= 0) { p.hp = 0; endGame(); }
}

function spawnParts(x, y, color, n) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, sp = rand(50, 220);
    game.parts.push({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp,
      life: 1, color, size: 2 + Math.random() * 3 });
  }
}

function dropPowerup(x, y, forceList) {
  const kind = forceList || (Math.random() < 0.62 ? 'power' : Math.random() < 0.5 ? 'bomb' : 'shield');
  game.powerups.push({ x, y, kind, vy: 95 });
}

function collectPowerup(pu) {
  const p = game.player;
  if (pu.kind === 'power') {
    if (p.weapon < MAX_WEAPON) { p.weapon++; setBanner(t('powerUp'), '#ffd24d'); }
    else { game.score += 500; setBanner(t('weaponMax'), '#ffd24d'); }
  } else if (pu.kind === 'bomb') {
    p.bombs++; setBanner(t('gotBomb'), '#ff8a3c');
  } else {
    p.shield = PLAYER.shieldTime; setBanner(t('shieldOn'), '#7ad0ff');
  }
}

// ---- enemy / boss firing ----------------------------------------------
function enemyShoot(e) {
  const p = game.player;
  const ang = Math.atan2(p.y - e.y, p.x - e.x);
  if (e.type === 'tank') {
    for (let i = -1; i <= 1; i++) {
      const a = ang + i * 0.28;
      game.ebullets.push({ x: e.x, y: e.y + 10, vx: Math.cos(a) * 180, vy: Math.sin(a) * 180 });
    }
  } else {
    game.ebullets.push({ x: e.x, y: e.y + 10, vx: Math.cos(ang) * 200, vy: Math.sin(ang) * 200 });
  }
}

function bossShoot(b) {
  const p = game.player;
  if (b.pattern === 0) { // downward fan
    for (let i = -3; i <= 3; i++) {
      const a = Math.PI / 2 + i * 0.22;
      game.ebullets.push({ x: b.x, y: b.y + 20, vx: Math.cos(a) * 165, vy: Math.sin(a) * 165 });
    }
  } else if (b.pattern === 1) { // aimed triple burst
    const ang = Math.atan2(p.y - b.y, p.x - b.x);
    for (let i = -1; i <= 1; i++) {
      const a = ang + i * 0.16;
      game.ebullets.push({ x: b.x, y: b.y + 20, vx: Math.cos(a) * 230, vy: Math.sin(a) * 230 });
    }
  } else { // dual side spray
    for (let i = 0; i < 5; i++) {
      const a = Math.PI / 2 + rand(-0.5, 0.5);
      const sx = i % 2 ? b.x - 26 : b.x + 26;
      game.ebullets.push({ x: sx, y: b.y + 12, vx: Math.cos(a) * 190, vy: Math.sin(a) * 190 });
    }
  }
}

// ---- update ------------------------------------------------------------
function update(dt) {
  game.t += dt;
  if (game.flash > 0) game.flash -= dt;
  if (game.banner) { game.banner.life -= dt; if (game.banner.life <= 0) game.banner = null; }

  for (const s of game.stars) {
    s.y += s.spd * dt;
    if (s.y > VH) { s.y = 0; s.x = Math.random() * VW; }
  }

  updatePlayer(dt);
  if (game.over) return;

  // spawn queue
  if (game.spawnList.length) {
    game.spawnCd -= dt;
    if (game.spawnCd <= 0) {
      spawnEnemy(game.spawnList.shift());
      game.spawnCd = Math.max(0.32, 1.0 - game.wave * 0.03);
    }
  }

  updateBullets(dt);
  updateEnemies(dt);
  updateBoss(dt);
  updatePowerups(dt);
  for (const pt of game.parts) { pt.x += pt.vx * dt; pt.y += pt.vy * dt; pt.life -= dt * 1.6; }
  game.parts = game.parts.filter(pt => pt.life > 0);

  // wave clear
  if (!game.spawnList.length && !game.enemies.length && !game.boss && !game.banner) {
    nextWave();
  }
}

function updatePlayer(dt) {
  const p = game.player;
  if (p.invuln > 0) p.invuln -= dt;
  if (p.shield > 0) p.shield -= dt;
  if (p.hit > 0) p.hit -= dt;

  if (pointer) {
    const tx = pointer.x, ty = pointer.y - 42;
    p.x += (tx - p.x) * Math.min(1, dt * 16);
    p.y += (ty - p.y) * Math.min(1, dt * 16);
  } else {
    let dx = 0, dy = 0;
    if (keys.ArrowLeft || keys.a || keys.A) dx -= 1;
    if (keys.ArrowRight || keys.d || keys.D) dx += 1;
    if (keys.ArrowUp || keys.w || keys.W) dy -= 1;
    if (keys.ArrowDown || keys.s || keys.S) dy += 1;
    if (dx && dy) { dx *= 0.707; dy *= 0.707; }
    p.x += dx * PLAYER.speed * dt;
    p.y += dy * PLAYER.speed * dt;
  }
  p.x = Math.max(16, Math.min(VW - 16, p.x));
  p.y = Math.max(40, Math.min(VH - 30, p.y));

  p.fireCd -= dt;
  if (p.fireCd <= 0) fire();
}

function updateBullets(dt) {
  for (const b of game.bullets) { b.x += b.vx * dt; b.y += b.vy * dt; }
  game.bullets = game.bullets.filter(b => b.y > -16 && b.x > -16 && b.x < VW + 16);

  for (const b of game.ebullets) { b.x += b.vx * dt; b.y += b.vy * dt; }
  game.ebullets = game.ebullets.filter(b => b.y < VH + 16 && b.y > -16 && b.x > -16 && b.x < VW + 16);

  // player bullets vs enemies / boss
  for (const b of game.bullets) {
    for (const e of game.enemies) {
      if (e.dead) continue;
      if (dist2(b.x, b.y, e.x, e.y) < e.r * e.r) {
        e.hp -= b.dmg; e.flash = 0.1; b.dead = true;
        spawnParts(b.x, b.y, ENEMIES[e.type].color, 3);
        break;
      }
    }
    if (b.dead) continue;
    const bs = game.boss;
    if (bs && dist2(b.x, b.y, bs.x, bs.y) < (bs.r + 4) ** 2) {
      bs.hp -= b.dmg; bs.flash = 0.08; b.dead = true;
      spawnParts(b.x, b.y, '#ff7ad0', 3);
    }
  }
  game.bullets = game.bullets.filter(b => !b.dead);

  // enemy bullets vs player
  const p = game.player;
  for (const b of game.ebullets) {
    if (dist2(b.x, b.y, p.x, p.y) < (PLAYER.r + 4) ** 2) {
      b.dead = true;
      hurtPlayer(12);
    }
  }
  game.ebullets = game.ebullets.filter(b => !b.dead);
}

function updateEnemies(dt) {
  const p = game.player;
  for (const e of game.enemies) {
    if (e.move === undefined) e.move = ENEMIES[e.type].move;
    if (e.move === 'dive') {
      e.y += e.speed * dt;
    } else if (e.move === 'sine') {
      e.y += e.speed * dt;
      e.x = e.spawnX + Math.sin(e.y * 0.035) * 64;
    } else if (e.move === 'hover') {
      if (e.y < e.hover) e.y += e.speed * dt;
      else e.x += Math.sin(game.t * 1.4 + e.spawnX) * 40 * dt;
    }
    e.x = Math.max(20, Math.min(VW - 20, e.x));
    if (e.flash > 0) e.flash -= dt;

    if (ENEMIES[e.type].fire && e.y > 0) {
      e.fireCd -= dt;
      if (e.fireCd <= 0) {
        enemyShoot(e);
        e.fireCd = ENEMIES[e.type].fire * rand(0.8, 1.3);
      }
    }
    // body collision with player
    if (dist2(e.x, e.y, p.x, p.y) < (e.r + PLAYER.r) ** 2) {
      hurtPlayer(22);
      e.hp -= 40; e.flash = 0.1;
    }
    if (e.hp <= 0 && !e.dead) {
      e.dead = true;
      game.score += e.score;
      spawnParts(e.x, e.y, ENEMIES[e.type].color, 16);
      if (Math.random() < 0.17) dropPowerup(e.x, e.y);
    }
  }
  game.enemies = game.enemies.filter(e => !e.dead && e.y < VH + 44);
}

function updateBoss(dt) {
  const b = game.boss;
  if (!b) return;
  if (b.flash > 0) b.flash -= dt;
  if (!b.entered) {
    b.y += 70 * dt;
    if (b.y >= 96) b.entered = true;
    return;
  }
  b.x += b.dir * 58 * dt;
  if (b.x < 60) b.dir = 1;
  if (b.x > VW - 60) b.dir = -1;

  b.patternT += dt;
  if (b.patternT > 4) { b.patternT = 0; b.pattern = (b.pattern + 1) % 3; }
  b.fireCd -= dt;
  if (b.fireCd <= 0) {
    bossShoot(b);
    b.fireCd = b.pattern === 1 ? 0.5 : 0.85;
  }
  if (dist2(b.x, b.y, game.player.x, game.player.y) < (b.r + PLAYER.r) ** 2) hurtPlayer(20);

  if (b.hp <= 0) {
    game.score += b.score;
    for (let i = 0; i < 60; i++) spawnParts(b.x + rand(-40, 40), b.y + rand(-24, 24), '#ff7ad0', 1);
    for (let i = 0; i < 3; i++) dropPowerup(b.x + rand(-30, 30), b.y, i === 0 ? 'power' : null);
    setBanner(t('bossDown'), '#ffd24d');
    game.boss = null;
  }
}

function updatePowerups(dt) {
  const p = game.player;
  for (const pu of game.powerups) {
    pu.y += pu.vy * dt;
    if (dist2(pu.x, pu.y, p.x, p.y) < 24 * 24) { pu.dead = true; collectPowerup(pu); }
  }
  game.powerups = game.powerups.filter(pu => !pu.dead && pu.y < VH + 20);
}

// ---- render ------------------------------------------------------------
function render() {
  ctx.fillStyle = '#0a0c16';
  ctx.fillRect(0, 0, VW, VH);
  for (const s of game.stars) { ctx.fillStyle = s.c; ctx.fillRect(s.x | 0, s.y | 0, s.size, s.size); }

  for (const pu of game.powerups) drawPowerup(ctx, pu.x, pu.y, pu.kind, game.t);
  for (const e of game.enemies) drawEnemy(ctx, e.x, e.y, e.type, game.t, e.flash);
  if (game.boss) {
    drawBoss(ctx, game.boss.x, game.boss.y, game.t, game.boss.flash);
    if (game.boss.entered) {
      const f = game.boss.hp / game.boss.maxHp;
      ctx.fillStyle = '#10131c'; ctx.fillRect(40, 14, VW - 80, 9);
      ctx.fillStyle = '#ff5d5d'; ctx.fillRect(41, 15, (VW - 82) * Math.max(0, f), 7);
    }
  }
  for (const b of game.bullets) drawBullet(ctx, b);
  for (const b of game.ebullets) drawEnemyBullet(ctx, b, game.t);
  for (const pt of game.parts) {
    ctx.globalAlpha = Math.max(0, pt.life);
    ctx.fillStyle = pt.color;
    ctx.fillRect(pt.x | 0, pt.y | 0, pt.size, pt.size);
  }
  ctx.globalAlpha = 1;
  if (!game.over) drawPlayer(ctx, game.player.x, game.player.y, game.t, game.player.invuln, game.player.shield);

  if (game.flash > 0) {
    ctx.globalAlpha = game.flash;
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, VW, VH);
    ctx.globalAlpha = 1;
  }
  if (game.banner) {
    ctx.globalAlpha = Math.min(1, game.banner.life);
    ctx.fillStyle = game.banner.color;
    ctx.font = '900 30px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(game.banner.text, VW / 2, VH / 2 - 30);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function updateHud() {
  const p = game.player;
  document.getElementById('hud-score').textContent = game.score;
  document.getElementById('hud-wave').textContent = `${t('wave')} ${game.wave}`;
  const f = p.hp / PLAYER.maxHp;
  document.getElementById('hp-fill').style.width = (f * 100) + '%';
  document.getElementById('hp-fill').style.background = f > 0.5 ? '#62d879' : f > 0.25 ? '#ffd24d' : '#ff5d5d';
  document.getElementById('hud-weapon').textContent = `⚡${p.weapon}`;
  document.getElementById('btn-bomb').textContent = `${t('bomb')} ${p.bombs}`;
  document.getElementById('btn-bomb').disabled = p.bombs <= 0;
}

// ---- win / lose --------------------------------------------------------
function bestScore() { return +(localStorage.getItem(BEST_KEY) || 0); }

function endGame() {
  if (game.over) return;
  game.over = true;
  spawnParts(game.player.x, game.player.y, '#dfe7f2', 36);
  if (game.score > bestScore()) localStorage.setItem(BEST_KEY, game.score);
  document.getElementById('over-score').textContent = t('finalScore', game.score);
  document.getElementById('over-wave').textContent = t('reachedWave', game.wave);
  document.getElementById('over-best').textContent = t('bestScore', bestScore());
  showOverlay('overlay-over');
}

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

// ---- input -------------------------------------------------------------
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  keys[e.key] = true;
  if (e.key === ' ' || e.key === 'b' || e.key === 'B') { e.preventDefault(); useBomb(); }
  else if (e.key === 'Escape') togglePause();
  else if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) e.preventDefault();
});
addEventListener('keyup', e => { keys[e.key] = false; });

function canvasPoint(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * VW, y: (e.clientY - r.top) / r.height * VH };
}
canvas.addEventListener('pointerdown', e => { e.preventDefault(); pointer = canvasPoint(e); });
canvas.addEventListener('pointermove', e => { if (pointer) pointer = canvasPoint(e); });
canvas.addEventListener('pointerup', () => { pointer = null; });
canvas.addEventListener('pointercancel', () => { pointer = null; });

function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideOverlay('overlay-pause');
}

// ---- wiring ------------------------------------------------------------
document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-bomb').onclick = useBomb;
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
