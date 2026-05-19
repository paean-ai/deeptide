// Pixel Sky Climber - an endless vertical bouncer. Steer left/right; the
// climber bounces on its own. Climb as high as you can without falling.

const BEST_KEY = 'pixel-sky-climber-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();
const keys = {};
let touchDir = 0;            // -1 / 0 / 1 from screen-half touches

const rand = (a, b) => a + Math.random() * (b - a);

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = {
    player: { x: VW / 2, y: VH - 130, vx: 0, vy: -PHYS.jump * 0.4, w: 20, h: 24,
      squash: 0, facing: 1, jet: 0 },
    platforms: [], coins: [], monsters: [], jets: [],
    scroll: 0, coinCount: 0, banner: null, t: 0, over: false,
  };
  // a guaranteed wide starting platform under the player
  game.platforms.push({ x: VW / 2 - 44, y: VH - 96, w: 88, type: 'normal', vx: 0, broken: false });
  let y = VH - 96;
  while (y > -160) {
    y -= gapFor(0);
    game.platforms.push(makePlatform(y));
  }
}

function makePlatform(y) {
  const h = game ? game.scroll : 0;
  const types = PLAT_TYPES;
  // weight breakable/moving up with height, springs stay rare
  let pool = [];
  for (const k in types) {
    let w = types[k].weight;
    if (k === 'breakable') w += Math.min(24, h / 1600);
    if (k === 'moving') w += Math.min(20, h / 2000);
    if (k === 'normal') w -= Math.min(34, h / 1400);
    for (let i = 0; i < Math.max(1, w); i++) pool.push(k);
  }
  const type = pool[(Math.random() * pool.length) | 0];
  const w = PLAT_W;
  const x = rand(6, VW - w - 6);
  const p = { x, y, w, type, vx: type === 'moving' ? (Math.random() < 0.5 ? -1 : 1) * rand(50, 100) : 0, broken: false };
  // optional rider: coin / monster / jetpack
  if (Math.random() < jetChance(h)) {
    game && game.jets.push({ x: x + w / 2, y: y - 22 });
  } else if (Math.random() < monsterChance(h) && type !== 'spring') {
    game && game.monsters.push({ x: x + w / 2, y: y - 24, alive: true });
  } else if (Math.random() < coinChance(h)) {
    game && game.coins.push({ x: x + w / 2, y: y - 24, taken: false });
  }
  return p;
}

function setBanner(text, color) { game.banner = { text, color, life: 1 }; }

// ---- update ------------------------------------------------------------
function update(dt) {
  const g = game, p = g.player;
  g.t += dt;
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }

  // input
  let dir = 0;
  if (keys.ArrowLeft || keys.a || keys.A) dir -= 1;
  if (keys.ArrowRight || keys.d || keys.D) dir += 1;
  if (touchDir) dir = touchDir;
  p.vx = dir * PHYS.move;
  if (dir > 0) p.facing = 1; else if (dir < 0) p.facing = -1;

  // vertical physics
  if (p.jet > 0) {
    p.jet -= dt;
    p.vy = -PHYS.jump * 1.05;
  } else {
    p.vy += PHYS.gravity * dt;
    if (p.vy > 980) p.vy = 980;
  }
  const prevFeet = p.y + p.h / 2;
  p.x += p.vx * dt;
  p.y += p.vy * dt;
  if (p.x < -p.w / 2) p.x = VW + p.w / 2;
  if (p.x > VW + p.w / 2) p.x = -p.w / 2;
  p.squash += (0 - p.squash) * Math.min(1, dt * 9);

  // moving platforms
  for (const plat of g.platforms) {
    if (plat.type === 'moving' && !plat.broken) {
      plat.x += plat.vx * dt;
      if (plat.x < 4) { plat.x = 4; plat.vx *= -1; }
      if (plat.x > VW - plat.w - 4) { plat.x = VW - plat.w - 4; plat.vx *= -1; }
    }
    if (plat.broken) { plat.brokenT = (plat.brokenT || 0) + dt; plat.y += 240 * dt; }
  }

  // platform landing (only while falling, and not jetpacking)
  const feet = p.y + p.h / 2;
  if (p.vy > 0 && p.jet <= 0) {
    for (const plat of g.platforms) {
      if (plat.broken) continue;
      // swept check: land whenever the feet cross the platform top this frame
      if (prevFeet <= plat.y + 6 && feet >= plat.y &&
          p.x + p.w / 2 > plat.x + 3 && p.x - p.w / 2 < plat.x + plat.w - 3) {
        if (plat.type === 'breakable') {
          plat.broken = true;                       // crumbles, no bounce
        } else {
          p.y = plat.y - p.h / 2;
          p.vy = -PHYS.jump * (plat.type === 'spring' ? PHYS.springMul : 1);
          p.squash = -0.7;
          if (plat.type === 'spring') setBanner(t('spring'), '#f4c85a');
          break;
        }
      }
    }
  }

  // coins
  for (const c of g.coins) {
    if (!c.taken && Math.abs(c.x - p.x) < 18 && Math.abs(c.y - p.y) < 20) {
      c.taken = true; g.coinCount++;
    }
  }
  // jetpacks
  for (const j of g.jets) {
    if (!j.taken && Math.abs(j.x - p.x) < 20 && Math.abs(j.y - p.y) < 22) {
      j.taken = true; p.jet = 1.7; setBanner(t('jetpack'), '#5fd9ff');
    }
  }
  // monsters
  for (const m of g.monsters) {
    if (!m.alive) continue;
    if (Math.abs(m.x - p.x) < 18 && Math.abs(m.y - p.y) < 20) {
      if (p.jet > 0) { m.alive = false; }
      else if (p.vy > 0 && prevFeet <= m.y - 2) {
        m.alive = false; p.vy = -PHYS.jump; p.squash = -0.6;
      } else { gameOver(); return; }
    }
  }

  // scroll the world down as the climber rises
  const limit = VH * PHYS.scrollAt;
  if (p.y < limit) {
    const dy = limit - p.y;
    p.y = limit;
    g.scroll += dy;
    for (const plat of g.platforms) plat.y += dy;
    for (const c of g.coins) c.y += dy;
    for (const m of g.monsters) m.y += dy;
    for (const j of g.jets) j.y += dy;
  }

  // generate above, cull below
  let topY = Infinity;
  for (const plat of g.platforms) if (plat.y < topY) topY = plat.y;
  while (topY > -150) {
    topY -= gapFor(g.scroll);
    game.platforms.push(makePlatform(topY));
  }
  g.platforms = g.platforms.filter(plat => plat.y < VH + 80 && !(plat.broken && (plat.brokenT || 0) > 1.2));
  g.coins = g.coins.filter(c => !c.taken && c.y < VH + 40);
  g.monsters = g.monsters.filter(m => m.alive && m.y < VH + 40);
  g.jets = g.jets.filter(j => !j.taken && j.y < VH + 40);

  if (p.y > VH + 60) gameOver();
}

// ---- render ------------------------------------------------------------
function render() {
  const g = game;
  drawSky(ctx, g.scroll, g.t);
  for (const plat of g.platforms) drawPlatform(ctx, plat);
  for (const c of g.coins) if (!c.taken) drawCoin(ctx, c.x, c.y, g.t);
  for (const j of g.jets) if (!j.taken) drawJetpack(ctx, j.x, j.y, g.t);
  for (const m of g.monsters) if (m.alive) drawMonster(ctx, m.x, m.y, g.t);
  if (!g.over) {
    drawClimber(ctx, g.player.x, g.player.y, g.player.squash, g.player.facing, g.player.jet > 0, g.t);
  }
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life * 1.5);
    ctx.fillStyle = g.banner.color;
    ctx.font = '900 26px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, VW / 2, 90);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function heightM() { return Math.floor(game.scroll / 10); }

function updateHud() {
  document.getElementById('hud-height').textContent = `${heightM()}${currentLang === 'zh' ? '米' : 'm'}`;
  document.getElementById('hud-coins').textContent = `◆ ${game.coinCount}`;
}

// ---- win / lose --------------------------------------------------------
function bestHeight() { return +(localStorage.getItem(BEST_KEY) || 0); }

function gameOver() {
  if (game.over) return;
  game.over = true;
  const h = heightM();
  if (h > bestHeight()) localStorage.setItem(BEST_KEY, h);
  document.getElementById('over-msg').textContent = t('reachedHeight', h);
  document.getElementById('over-best').textContent = t('bestHeight', bestHeight());
  showOverlay('overlay-over');
}

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
  document.getElementById('title-best').textContent = t('bestHeight', bestHeight());
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
  if (e.key === 'Escape') togglePause();
  if (['ArrowLeft', 'ArrowRight'].includes(e.key)) e.preventDefault();
});
addEventListener('keyup', e => { keys[e.key] = false; });

function touchSide(e) {
  const r = canvas.getBoundingClientRect();
  const x = (e.clientX - r.left) / r.width;
  touchDir = x < 0.5 ? -1 : 1;
}
canvas.addEventListener('pointerdown', e => { e.preventDefault(); touchSide(e); });
canvas.addEventListener('pointermove', e => { if (touchDir) touchSide(e); });
canvas.addEventListener('pointerup', () => { touchDir = 0; });
canvas.addEventListener('pointercancel', () => { touchDir = 0; });

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-resume').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = startGame;
document.getElementById('btn-pause-menu').onclick = gotoTitle;
document.getElementById('btn-over-again').onclick = startGame;
document.getElementById('btn-over-menu').onclick = gotoTitle;
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestHeight', bestHeight());
});

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.04, (now - lastT) / 1000);
  lastT = now;
  if (game && !game.over && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    update(dt);
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
