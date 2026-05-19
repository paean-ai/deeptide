// Pixel Turbo Racer - endless arcade racer: weave traffic, chain near-misses,
// burn nitro. Crash and it's over; chase a higher score.

const BEST_KEY = 'pixel-turbo-racer-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();
const keys = {};
let pointerX = null;

const rand = (a, b) => a + Math.random() * (b - a);

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = {
    player: { x: laneCenter(1), nitro: 1, boost: 0, oil: 0, hit: 0 },
    traffic: [], pickups: [], hazards: [], trees: [], parts: [],
    speed: PLAYER.startSpeed, dist: 0, score: 0, combo: 0, comboT: 0,
    spawnCd: 0.8, treeCd: 0, roadScroll: 0, banner: null, t: 0, over: false,
  };
  for (let i = 0; i < 6; i++) {
    game.trees.push({ x: i % 2 ? ROAD_X + ROAD_W + rand(14, 38) : ROAD_X - rand(14, 38), y: i * 130 });
  }
}

function setBanner(text, color) { game.banner = { text, color, life: 1.1 }; }

// ---- spawning ----------------------------------------------------------
function spawnRow() {
  const d = diffOf(game.dist);
  const roll = Math.random();
  const lane = Math.floor(Math.random() * LANES);
  const x = laneCenter(lane);
  if (roll < 0.62) {
    game.traffic.push({ x, y: -70, lane, color: TRAFFIC_COLORS[Math.floor(Math.random() * TRAFFIC_COLORS.length)],
      rel: d.trafficRel * rand(0.8, 1.15), checked: false });
  } else if (roll < 0.74) {
    game.hazards.push({ x, y: -40, kind: 'cone' });
  } else if (roll < 0.82) {
    game.hazards.push({ x, y: -40, kind: 'oil' });
  } else if (roll < 0.95) {
    const lc = laneCenter(Math.floor(Math.random() * LANES));
    for (let i = 0; i < 3; i++) game.pickups.push({ x: lc, y: -40 - i * 36, kind: 'coin' });
  } else {
    game.pickups.push({ x, y: -40, kind: 'nitro' });
  }
}

// ---- particles ---------------------------------------------------------
function burst(x, y, color, n) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, sp = rand(40, 240);
    game.parts.push({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp,
      life: 1, color, size: 2 + Math.random() * 3 });
  }
}

// ---- update ------------------------------------------------------------
function update(dt) {
  const g = game, p = g.player;
  g.t += dt;
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }

  // speed
  g.speed = Math.min(PLAYER.maxSpeed, PLAYER.startSpeed + g.dist * PLAYER.accel * 0.06);
  if (p.boost > 0) { p.boost -= dt; }
  const worldSpeed = g.speed + (p.boost > 0 ? PLAYER.boostSpeed : 0);

  g.dist += worldSpeed * dt / 12;
  g.score += worldSpeed * dt * 0.05;
  g.roadScroll = (g.roadScroll + worldSpeed * dt) % 64;

  // combo decay
  if (g.comboT > 0) { g.comboT -= dt; if (g.comboT <= 0) g.combo = 0; }
  if (p.hit > 0) p.hit -= dt;
  if (p.oil > 0) p.oil -= dt;

  // steering
  const minX = ROAD_X + PLAYER.w / 2, maxX = ROAD_X + ROAD_W - PLAYER.w / 2;
  if (pointerX !== null) {
    let target = Math.max(minX, Math.min(maxX, pointerX));
    p.x += (target - p.x) * Math.min(1, dt * 14);
  } else {
    let dx = 0;
    if (keys.ArrowLeft || keys.a || keys.A) dx -= 1;
    if (keys.ArrowRight || keys.d || keys.D) dx += 1;
    p.x += dx * PLAYER.steer * dt;
  }
  if (p.oil > 0) p.x += Math.sin(g.t * 22) * 90 * dt;  // oil slick wobble
  p.x = Math.max(minX, Math.min(maxX, p.x));

  // spawning
  g.spawnCd -= dt;
  if (g.spawnCd <= 0) {
    spawnRow();
    g.spawnCd = diffOf(g.dist).spawnGap * rand(0.85, 1.2);
  }
  g.treeCd -= dt;
  if (g.treeCd <= 0) {
    g.treeCd = rand(0.4, 0.9);
    const left = Math.random() < 0.5;
    g.trees.push({ x: left ? ROAD_X - rand(14, 40) : ROAD_X + ROAD_W + rand(14, 40), y: -40 });
  }

  // scroll traffic
  for (const c of g.traffic) {
    c.y += (worldSpeed - c.rel) * dt;
    if (!c.checked && c.y - PLAYER.y > 18) {
      c.checked = true;
      const gap = Math.abs(c.x - p.x);
      if (gap < PLAYER.w / 2 + 16 + 30 && gap > PLAYER.w / 2 + 16) {
        g.combo++;
        g.comboT = 2.4;
        const bonus = 15 * g.combo;
        g.score += bonus;
        setBanner(`${t('nearMiss')} x${g.combo}`, '#7ad0ff');
      }
    }
  }
  g.traffic = g.traffic.filter(c => c.y < VH + 80);

  for (const it of g.pickups) it.y += worldSpeed * dt;
  for (const hz of g.hazards) hz.y += worldSpeed * dt;
  for (const tr of g.trees) tr.y += worldSpeed * dt;
  g.pickups = g.pickups.filter(it => it.y < VH + 40 && !it.dead);
  g.hazards = g.hazards.filter(hz => hz.y < VH + 40 && !hz.dead);
  g.trees = g.trees.filter(tr => tr.y < VH + 60);

  for (const pt of g.parts) { pt.x += pt.vx * dt; pt.y += pt.vy * dt; pt.vy += 200 * dt; pt.life -= dt * 1.8; }
  g.parts = g.parts.filter(pt => pt.life > 0);

  collisions();
}

function hitBox(ax, ay, aw, ah, bx, by, bw, bh) {
  return Math.abs(ax - bx) < (aw + bw) / 2 && Math.abs(ay - by) < (ah + bh) / 2;
}

function collisions() {
  const g = game, p = g.player;
  const invuln = p.boost > 0;
  // traffic
  for (const c of g.traffic) {
    if (hitBox(p.x, PLAYER.y, PLAYER.w, PLAYER.h, c.x, c.y, 32, 50)) {
      if (invuln) {
        if (!c.dead) { c.dead = true; g.score += 80; burst(c.x, c.y, c.color, 18); }
      } else { crash(); return; }
    }
  }
  g.traffic = g.traffic.filter(c => !c.dead);
  // hazards
  for (const hz of g.hazards) {
    const def = HAZARDS[hz.kind];
    if (hitBox(p.x, PLAYER.y, PLAYER.w, PLAYER.h, hz.x, hz.y, def.w, def.h)) {
      if (hz.kind === 'cone') {
        if (!invuln) { crash(); return; }
        hz.dead = true; burst(hz.x, hz.y, '#ff8a3c', 10);
      } else { // oil
        if (p.oil <= 0) {
          p.oil = 1.4; g.combo = 0; g.comboT = 0;
          setBanner('SPIN!', '#ff8a3c');
        }
        hz.dead = true;
      }
    }
  }
  // pickups
  for (const it of g.pickups) {
    if (hitBox(p.x, PLAYER.y, PLAYER.w, PLAYER.h, it.x, it.y, 22, 22)) {
      it.dead = true;
      if (it.kind === 'coin') {
        g.score += PICKUPS.coin.score;
        burst(it.x, it.y, '#ffd24d', 6);
      } else {
        g.player.nitro = Math.min(PLAYER.maxNitro, g.player.nitro + 1);
        setBanner('+NITRO', '#7ad0ff');
      }
    }
  }
}

function useBoost() {
  if (!game || game.over) return;
  const p = game.player;
  if (p.nitro <= 0 || p.boost > 0) return;
  p.nitro--;
  p.boost = PLAYER.boostTime;
  setBanner(t('boost'), '#ffd24d');
  burst(p.x, PLAYER.y + 26, '#ffd24d', 14);
}

function crash() {
  if (game.over) return;
  game.over = true;
  game.player.hit = 1;
  burst(game.player.x, PLAYER.y, '#ff7043', 40);
  burst(game.player.x, PLAYER.y, '#ffd24d', 26);
  const sc = Math.floor(game.score);
  if (sc > bestScore()) localStorage.setItem(BEST_KEY, sc);
  document.getElementById('over-score').textContent = t('finalScore', sc);
  document.getElementById('over-dist').textContent = t('distRun', Math.floor(game.dist));
  document.getElementById('over-best').textContent = t('bestScore', bestScore());
  showOverlay('overlay-over');
}

// ---- render ------------------------------------------------------------
function render() {
  const g = game;
  // grass
  ctx.fillStyle = '#1d3a22';
  ctx.fillRect(0, 0, VW, VH);
  // tarmac
  ctx.fillStyle = '#2d2f3a';
  ctx.fillRect(ROAD_X, 0, ROAD_W, VH);
  // shoulder rumble strips
  for (let y = -((g.roadScroll) % 40); y < VH; y += 40) {
    ctx.fillStyle = '#d8d8e0';
    ctx.fillRect(ROAD_X - 6, y, 6, 20);
    ctx.fillStyle = '#d8453c';
    ctx.fillRect(ROAD_X - 6, y + 20, 6, 20);
    ctx.fillStyle = '#d8d8e0';
    ctx.fillRect(ROAD_X + ROAD_W, y, 6, 20);
    ctx.fillStyle = '#d8453c';
    ctx.fillRect(ROAD_X + ROAD_W, y + 20, 6, 20);
  }
  // lane dashes
  ctx.fillStyle = '#c9ccd8';
  for (let lane = 1; lane < LANES; lane++) {
    const lx = ROAD_X + LANE_W * lane;
    for (let y = -(g.roadScroll % 64); y < VH; y += 64) ctx.fillRect(lx - 2, y, 4, 34);
  }
  // trees
  for (const tr of g.trees) drawTree(ctx, tr.x, tr.y);
  // pickups + hazards
  for (const hz of g.hazards) {
    if (hz.kind === 'cone') drawCone(ctx, hz.x, hz.y);
    else drawOil(ctx, hz.x, hz.y, g.t);
  }
  for (const it of g.pickups) {
    if (it.kind === 'coin') drawCoin(ctx, it.x, it.y, g.t);
    else drawNitro(ctx, it.x, it.y, g.t);
  }
  // traffic
  for (const c of g.traffic) drawCar(ctx, c.x, c.y, c.color, false, 0);
  // player
  const p = g.player;
  if (!g.over) {
    if (p.boost > 0) {
      ctx.globalAlpha = 0.6;
      for (let i = 1; i <= 3; i++) {
        ctx.fillStyle = i % 2 ? '#ffd24d' : '#ff8a3c';
        ctx.fillRect(p.x - 9, PLAYER.y + 26 + i * 12, 18, 8);
      }
      ctx.globalAlpha = 1;
    }
    drawCar(ctx, p.x, PLAYER.y, '#3fc7e0', true, p.hit);
  }
  // particles
  for (const pt of g.parts) {
    ctx.globalAlpha = Math.max(0, pt.life);
    ctx.fillStyle = pt.color;
    ctx.fillRect(pt.x | 0, pt.y | 0, pt.size, pt.size);
  }
  ctx.globalAlpha = 1;
  // banner
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life * 1.6);
    ctx.fillStyle = g.banner.color;
    ctx.font = '900 26px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, VW / 2, 130);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function updateHud() {
  const g = game;
  document.getElementById('hud-score').textContent = Math.floor(g.score);
  document.getElementById('hud-dist').textContent = `${Math.floor(g.dist)}${t('metres')}`;
  document.getElementById('hud-speed').textContent = `${Math.round(g.speed / 3 + (g.player.boost > 0 ? 90 : 0))}`;
  document.getElementById('hud-combo').textContent = g.combo > 0 ? `x${g.combo}` : '—';
  document.getElementById('btn-nitro').textContent = `${t('nitro')} ${g.player.nitro}`;
  document.getElementById('btn-nitro').disabled = g.player.nitro <= 0 || g.player.boost > 0;
}

// ---- win / lose --------------------------------------------------------
function bestScore() { return +(localStorage.getItem(BEST_KEY) || 0); }

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

function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideOverlay('overlay-pause');
}

// ---- input -------------------------------------------------------------
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  keys[e.key] = true;
  if (e.key === ' ' || e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W') { e.preventDefault(); useBoost(); }
  else if (e.key === 'Escape') togglePause();
  else if (['ArrowLeft', 'ArrowRight', 'ArrowDown'].includes(e.key)) e.preventDefault();
});
addEventListener('keyup', e => { keys[e.key] = false; });

function canvasX(e) {
  const r = canvas.getBoundingClientRect();
  return (e.clientX - r.left) / r.width * VW;
}
canvas.addEventListener('pointerdown', e => { e.preventDefault(); pointerX = canvasX(e); });
canvas.addEventListener('pointermove', e => { if (pointerX !== null) pointerX = canvasX(e); });
canvas.addEventListener('pointerup', () => { pointerX = null; });
canvas.addEventListener('pointercancel', () => { pointerX = null; });

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-nitro').onclick = useBoost;
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
