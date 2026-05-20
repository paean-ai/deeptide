// Pixel Deep Miner - dig-down mining game. Drill ore, manage fuel and cargo,
// fly back to the surface to sell and upgrade your rig.

const SAVE_KEY = 'pixel-deep-miner-run';
const BEST_KEY = 'pixel-deep-miner-best';

const VIEW_H = 13;
const LAVA_DPS = 26;
const ORE_BLOCK = { copper: B_COPPER, iron: B_IRON, silver: B_SILVER, cobalt: B_COBALT, gold: B_GOLD, gem: B_GEM, mythril: B_MYTHRIL };

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = COLS * TILE;
canvas.height = VIEW_H * TILE;
ctx.imageSmoothingEnabled = false;

let game = null;
let animT = 0;
let lastT = performance.now();
const held = [];                 // stack of [dx,dy] currently pressed
let pendingSwipe = null;

// ---- world generation --------------------------------------------------
function pickOre(depth) {
  const pool = [];
  const weights = { copper: 5, iron: 4, silver: 3, cobalt: 3, gold: 2, gem: 1, mythril: 1 };
  for (const id in ORES) if (ORES[id].minDepth <= depth) {
    for (let i = 0; i < (weights[id] || 1); i++) pool.push(id);
  }
  return pool.length ? pool[Math.floor(Math.random() * pool.length)] : null;
}

function genWorld() {
  const tiles = new Uint8Array(COLS * ROWS);
  for (let y = GRASS_ROW; y < ROWS; y++) {
    for (let x = 0; x < COLS; x++) {
      if (y === GRASS_ROW) { tiles[idx(x, y)] = B_GRASS; continue; }
      if (y >= BEDROCK_ROW) { tiles[idx(x, y)] = B_BEDROCK; continue; }
      const depth = y - GRASS_ROW;
      let base;
      if (depth < 10) base = B_DIRT;
      else if (depth < 40) base = Math.random() < 0.7 ? B_DIRT : B_STONE;
      else if (depth < 95) base = Math.random() < 0.6 ? B_STONE : B_HARD;
      else base = Math.random() < 0.4 ? B_STONE : B_HARD;
      tiles[idx(x, y)] = base;
    }
  }
  for (let y = GRASS_ROW + 1; y < BEDROCK_ROW; y++) {
    const depth = y - GRASS_ROW;
    for (let x = 0; x < COLS; x++) {
      if (Math.random() < 0.09) {
        const ore = pickOre(depth);
        if (ore) tiles[idx(x, y)] = ORE_BLOCK[ore];
      }
      const lavaChance = Math.min(0.07, Math.max(0, (depth - 32) * 0.0007));
      if (depth > 32 && Math.random() < lavaChance) tiles[idx(x, y)] = B_LAVA;
    }
  }
  return tiles;
}

// ---- run lifecycle -----------------------------------------------------
function newRun() {
  game = {
    tiles: genWorld(),
    player: {
      gx: COLS >> 1, gy: GRASS_ROW - 1, face: 1,
      px: (COLS >> 1) * TILE, py: (GRASS_ROW - 1) * TILE,
    },
    mode: 'idle',
    tween: null, drill: null, fallTiles: 0,
    cash: 0, fuel: 0, hull: 0,
    cargo: { copper: 0, iron: 0, silver: 0, gold: 0, gem: 0 },
    up: { drill: 0, fuel: 0, cargo: 0, hull: 0, thruster: 0 },
    parts: [], log: '', wasSurface: true, over: false,
  };
  game.fuel = fuelCap();
  game.hull = hullMax();
}

// ---- derived stats -----------------------------------------------------
function drillPower() { return UPGRADES.drill.levels[game.up.drill].power; }
function fuelCap() { return UPGRADES.fuel.levels[game.up.fuel].cap; }
function cargoCap() { return UPGRADES.cargo.levels[game.up.cargo].cap; }
function hullMax() { return UPGRADES.hull.levels[game.up.hull].hp; }
function lavaResist() { return UPGRADES.hull.levels[game.up.hull].lavaResist; }
function thrusterFuel() { return UPGRADES.thruster.levels[game.up.thruster].fuelPerTile; }

function cargoUsed() {
  let n = 0;
  for (const id in game.cargo) n += game.cargo[id] * ORES[id].weight;
  return n;
}
function cargoCount() {
  let n = 0;
  for (const id in game.cargo) n += game.cargo[id];
  return n;
}
function tileAt(x, y) {
  if (x < 0 || x >= COLS || y < 0 || y >= ROWS) return B_BEDROCK;
  return game.tiles[idx(x, y)];
}
function solid(x, y) { return tileAt(x, y) !== B_EMPTY; }
function atSurface() { return game.player.gy <= GRASS_ROW - 1; }

// ---- action state machine ---------------------------------------------
function beginAction(dx, dy) {
  const p = game.player;
  if (dx !== 0) p.face = dx;
  const tx = p.gx + dx, ty = p.gy + dy;
  if (tx < 0 || tx >= COLS || ty < 0 || ty >= ROWS) return;

  if (dy < 0) { // fly up - only into empty space
    if (solid(tx, ty)) return;
    game.fuel -= thrusterFuel();
    startMove(tx, ty, 0.13, false);
    return;
  }
  const tile = tileAt(tx, ty);
  if (tile === B_EMPTY) { startMove(tx, ty, 0.14, true); return; }
  const b = BLOCKS[tile];
  if (!b || b.hardness === Infinity) return; // bedrock
  game.mode = 'drill';
  game.drill = { tx, ty, prog: 0, need: b.hardness / drillPower(), tile, dx, dy };
}

function startMove(tx, ty, dur, charge) {
  const p = game.player;
  game.mode = 'move';
  game.tween = { fromX: p.px, fromY: p.py, toX: tx * TILE, toY: ty * TILE,
    gx: tx, gy: ty, t: 0, dur, kind: 'move', charge: !!charge };
}

function startFall() {
  const p = game.player;
  game.mode = 'fall';
  game.tween = { fromX: p.px, fromY: p.py, toX: p.gx * TILE, toY: (p.gy + 1) * TILE,
    gx: p.gx, gy: p.gy + 1, t: 0, dur: 0.085, kind: 'fall' };
}

// Called once whenever the rig comes to rest: applies fall damage and, on the
// surface, free refuel / repair plus the shop.
function landIfNeeded() {
  if (game.fallTiles > FALL_SAFE) {
    damage((game.fallTiles - FALL_SAFE) * FALL_DMG, false);
  }
  game.fallTiles = 0;
  if (atSurface()) {
    game.fuel = fuelCap();
    game.hull = hullMax();
    if (!game.wasSurface) { game.wasSurface = true; openShop(); }
  } else {
    game.wasSurface = false;
  }
}

// Idle frame: gravity pulls down unless the player is actively thrusting up.
function tickIdle() {
  const p = game.player;
  const dir = pendingSwipe || held[held.length - 1];
  pendingSwipe = null;
  const thrusting = !!dir && dir[1] < 0;
  if (!solid(p.gx, p.gy + 1) && p.gy + 1 < ROWS && !thrusting) {
    startFall();
    return;
  }
  landIfNeeded();
  if (dir && !game.over) beginAction(dir[0], dir[1]);
}

function damage(amt, isLava) {
  if (isLava) amt *= (1 - lavaResist());
  game.hull -= amt;
  if (game.hull <= 0) { game.hull = 0; gameOver('hull'); }
}

function collectTile(tile) {
  const b = BLOCKS[tile];
  if (b && b.ore) {
    if (cargoUsed() + ORES[b.ore].weight <= cargoCap()) {
      game.cargo[b.ore]++;
      burst(game.drill.tx, game.drill.ty, ORES[b.ore].glow, 10);
    } else {
      game.log = t('cargoFull');
      burst(game.drill.tx, game.drill.ty, '#ff6b6b', 6);
    }
  } else {
    burst(game.drill.tx, game.drill.ty, b ? b.color : '#888', 7);
  }
}

// ---- particles ---------------------------------------------------------
function burst(gx, gy, color, count) {
  const cx = gx * TILE + TILE / 2, cy = gy * TILE + TILE / 2;
  for (let i = 0; i < count; i++) {
    const a = Math.random() * Math.PI * 2, sp = 40 + Math.random() * 130;
    game.parts.push({ x: cx, y: cy, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 40,
      life: 1, color, size: 2 + Math.random() * 3 });
  }
}

// ---- shop --------------------------------------------------------------
function sellAll() {
  let n = 0, cash = 0;
  for (const id in game.cargo) {
    n += game.cargo[id];
    cash += game.cargo[id] * ORES[id].value;
    game.cargo[id] = 0;
  }
  if (n === 0) { game.log = t('nothingToSell'); }
  else { game.cash += cash; game.log = t('sold', n, cash); }
  renderShop();
}

function buyUpgrade(key) {
  const u = UPGRADES[key], lvl = game.up[key];
  if (lvl >= u.levels.length - 1) return;
  const cost = u.cost[lvl + 1];
  if (game.cash < cost) return;
  game.cash -= cost;
  game.up[key]++;
  if (key === 'fuel') game.fuel = fuelCap();
  if (key === 'hull') game.hull = hullMax();
  game.log = t('bought', tUpName(key));
  renderShop();
}

// ---- update ------------------------------------------------------------
function update(dt) {
  const p = game.player;

  // fuel drain while underground
  if (!atSurface()) {
    game.fuel -= FUEL_IDLE * dt;
    if (game.fuel <= 0) { game.fuel = 0; gameOver('fuel'); return; }
  }

  if (game.mode === 'idle') {
    tickIdle();
  } else if (game.mode === 'move' || game.mode === 'fall') {
    const tw = game.tween;
    tw.t += dt;
    const k = Math.min(1, tw.t / tw.dur);
    const e = k * k * (3 - 2 * k); // smoothstep
    p.px = tw.fromX + (tw.toX - tw.fromX) * e;
    p.py = tw.fromY + (tw.toY - tw.fromY) * e;
    if (k >= 1) {
      p.gx = tw.gx; p.gy = tw.gy;
      p.px = tw.toX; p.py = tw.toY;
      if (tw.kind === 'fall') game.fallTiles++;
      else if (tw.charge) game.fuel -= FUEL_PER_MOVE;
      game.mode = 'idle';
      game.tween = null;
    }
  } else if (game.mode === 'drill') {
    const d = game.drill;
    d.prog += dt;
    if (d.tile === B_LAVA) damage(LAVA_DPS * dt, true);
    if (game.over) return;
    if (d.prog >= d.need) {
      collectTile(d.tile);
      game.tiles[idx(d.tx, d.ty)] = B_EMPTY;
      game.fuel -= FUEL_PER_DIG;
      startMove(d.tx, d.ty, 0.12, false);
      game.drill = null;
    }
  }

  // particles
  for (const pt of game.parts) {
    pt.x += pt.vx * dt; pt.y += pt.vy * dt; pt.vy += 420 * dt; pt.life -= dt * 1.5;
  }
  game.parts = game.parts.filter(pt => pt.life > 0);
}

// ---- render ------------------------------------------------------------
function camY() {
  return Math.max(0, Math.min(ROWS - VIEW_H, Math.round(game.player.gy - VIEW_H / 2 + 1)));
}

function render() {
  const cy = camY();
  // sky / cave background
  for (let vy = 0; vy < VIEW_H; vy++) {
    const wy = cy + vy;
    ctx.fillStyle = wy < GRASS_ROW ? '#2b4c6b' : '#100d14';
    ctx.fillRect(0, vy * TILE, canvas.width, TILE);
  }
  // sky sun
  if (cy < GRASS_ROW) {
    ctx.fillStyle = '#ffe39a';
    ctx.fillRect(canvas.width - 92, (1 - cy) * TILE + 8, 30, 30);
  }
  // tiles
  for (let vy = 0; vy < VIEW_H; vy++) {
    const wy = cy + vy;
    for (let x = 0; x < COLS; x++) {
      const tile = tileAt(x, wy);
      if (tile === B_EMPTY) continue;
      const variant = (x * 7 + wy * 13) % 5;
      const drilling = game.drill && game.drill.tx === x && game.drill.ty === wy;
      drawBlock(ctx, x * TILE, vy * TILE, tile, variant,
        drilling ? game.drill.prog / game.drill.need : 0, animT);
    }
  }
  // shop hut on the surface
  if (cy <= GRASS_ROW) {
    drawShop(ctx, 2 * TILE, (GRASS_ROW - 1 - cy) * TILE);
  }
  // miner
  const p = game.player;
  drawMiner(ctx, p.px, p.py - cy * TILE, p.face, game.mode === 'drill', animT);
  // particles
  for (const pt of game.parts) {
    ctx.globalAlpha = Math.max(0, pt.life);
    ctx.fillStyle = pt.color;
    ctx.fillRect(pt.x, pt.y - cy * TILE, pt.size, pt.size);
  }
  ctx.globalAlpha = 1;
  updateHud();
}

function updateHud() {
  const p = game.player;
  document.getElementById('hud-depth').textContent = `${t('depth')} ${depthOf(p.gy)}m`;
  document.getElementById('hud-cash').textContent = `◆ ${game.cash}`;
  const fFrac = game.fuel / fuelCap();
  document.getElementById('fuel-fill').style.width = (fFrac * 100) + '%';
  document.getElementById('fuel-fill').style.background = fFrac > 0.3 ? '#f4c85a' : '#ff5d5d';
  const hFrac = game.hull / hullMax();
  document.getElementById('hull-fill').style.width = (hFrac * 100) + '%';
  document.getElementById('hull-fill').style.background = hFrac > 0.3 ? '#62d879' : '#ff5d5d';
  document.getElementById('cargo-text').textContent = `${t('cargo')} ${cargoUsed()}/${cargoCap()}`;
  document.getElementById('log-line').textContent = game.log;
}

// ---- shop UI -----------------------------------------------------------
function renderShop() {
  let html = '';
  const cv = { copper: game.cargo.copper, iron: game.cargo.iron, silver: game.cargo.silver,
    gold: game.cargo.gold, gem: game.cargo.gem };
  html += '<div class="shop-cargo">';
  for (const id in cv) {
    html += `<span style="color:${ORES[id].color}">${tOre(id)} ×${cv[id]}</span>`;
  }
  html += '</div>';
  document.getElementById('shop-cargo').innerHTML = html;
  document.getElementById('shop-cash').textContent = `◆ ${game.cash}`;

  let rows = '';
  for (const key in UPGRADES) {
    const u = UPGRADES[key], lvl = game.up[key], max = lvl >= u.levels.length - 1;
    const cost = max ? 0 : u.cost[lvl + 1];
    const can = !max && game.cash >= cost;
    rows += `<div class="up-row">
      <div class="up-info"><b>${tUpName(key)}</b> <i>Lv.${lvl + 1}</i><br><span>${tUpDesc(key)}</span></div>
      <button class="up-buy" data-key="${key}" ${max || !can ? 'disabled' : ''}>
        ${max ? t('maxed') : '◆ ' + cost}</button></div>`;
  }
  document.getElementById('shop-upgrades').innerHTML = rows;
  document.querySelectorAll('.up-buy').forEach(b => {
    b.onclick = () => buyUpgrade(b.dataset.key);
  });
}

function openShop() {
  renderShop();
  showOverlay('overlay-shop');
}

// ---- win / lose --------------------------------------------------------
function bestDepth() { return +(localStorage.getItem(BEST_KEY) || 0); }

function gameOver(cause) {
  if (game.over) return;
  game.over = true;
  const d = depthOf(game.player.gy);
  if (d > bestDepth()) localStorage.setItem(BEST_KEY, d);
  localStorage.removeItem(SAVE_KEY);
  document.getElementById('over-cause').textContent =
    cause === 'fuel' ? t('strandedFuel') : t('strandedHull');
  document.getElementById('over-depth').textContent = t('reachedDepth', d);
  document.getElementById('over-best').textContent = t('bestDepth', bestDepth());
  showOverlay('overlay-over');
}

// ---- save / load -------------------------------------------------------
function saveRun() {
  if (!game || game.over) return;
  try {
    localStorage.setItem(SAVE_KEY, JSON.stringify({
      tiles: Array.from(game.tiles),
      gx: game.player.gx, gy: game.player.gy, face: game.player.face,
      cash: game.cash, fuel: game.fuel, hull: game.hull,
      cargo: game.cargo, up: game.up,
    }));
  } catch (e) { /* storage unavailable */ }
}
function hasSavedRun() {
  try { return !!localStorage.getItem(SAVE_KEY); } catch (e) { return false; }
}
function loadRun() {
  let d;
  try { d = JSON.parse(localStorage.getItem(SAVE_KEY)); } catch (e) { return false; }
  if (!d || !d.tiles) return false;
  newRun();
  game.tiles = Uint8Array.from(d.tiles);
  game.player.gx = d.gx; game.player.gy = d.gy; game.player.face = d.face;
  game.player.px = d.gx * TILE; game.player.py = d.gy * TILE;
  game.cash = d.cash; game.up = d.up;
  game.fuel = d.fuel; game.hull = d.hull;
  game.cargo = d.cargo;
  game.wasSurface = game.player.gy <= GRASS_ROW - 1;
  return true;
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

function enterGame() {
  hideAllOverlays();
  showScreen('screen-game');
  render();
}
function gotoTitle() {
  hideAllOverlays();
  document.getElementById('btn-resume').classList.toggle('hidden', !hasSavedRun());
  document.getElementById('title-best').textContent = t('bestDepth', bestDepth());
  showScreen('screen-title');
}

// ---- input -------------------------------------------------------------
const KEYDIR = {
  ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0],
  w: [0, -1], s: [0, 1], a: [-1, 0], d: [1, 0],
  W: [0, -1], S: [0, 1], A: [-1, 0], D: [1, 0],
};
function pushDir(v) {
  if (!held.some(h => h[0] === v[0] && h[1] === v[1])) held.push(v);
}
function popDir(v) {
  const i = held.findIndex(h => h[0] === v[0] && h[1] === v[1]);
  if (i >= 0) held.splice(i, 1);
}
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  if (KEYDIR[e.key]) { e.preventDefault(); pushDir(KEYDIR[e.key]); }
  else if (e.key === 'Escape') togglePause();
});
addEventListener('keyup', e => { if (KEYDIR[e.key]) popDir(KEYDIR[e.key]); });

[['btn-up', [0, -1]], ['btn-down', [0, 1]], ['btn-left', [-1, 0]], ['btn-right', [1, 0]]].forEach(([id, v]) => {
  const el = document.getElementById(id);
  const down = e => { e.preventDefault(); pushDir(v); };
  const up = e => { e.preventDefault(); popDir(v); };
  el.addEventListener('pointerdown', down);
  el.addEventListener('pointerup', up);
  el.addEventListener('pointerleave', up);
  el.addEventListener('pointercancel', up);
});

let touch = null;
canvas.addEventListener('touchstart', e => {
  touch = { x: e.touches[0].clientX, y: e.touches[0].clientY };
}, { passive: true });
canvas.addEventListener('touchend', e => {
  if (!touch) return;
  const dx = e.changedTouches[0].clientX - touch.x;
  const dy = e.changedTouches[0].clientY - touch.y;
  touch = null;
  if (Math.abs(dx) < 20 && Math.abs(dy) < 20) return;
  pendingSwipe = Math.abs(dx) > Math.abs(dy) ? [Math.sign(dx), 0] : [0, Math.sign(dy)];
}, { passive: true });

// ---- pause -------------------------------------------------------------
function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideOverlay('overlay-pause');
}

// ---- wiring ------------------------------------------------------------
document.getElementById('btn-play').onclick = () => { newRun(); enterGame(); };
document.getElementById('btn-resume').onclick = () => { if (loadRun()) enterGame(); };
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-close').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = () => { newRun(); enterGame(); };
document.getElementById('btn-pause-menu').onclick = () => { saveRun(); gotoTitle(); };
document.getElementById('btn-shop').onclick = openShop;
document.getElementById('btn-sell').onclick = sellAll;
document.getElementById('btn-shop-close').onclick = () => { hideOverlay('overlay-shop'); saveRun(); };
document.getElementById('btn-over-again').onclick = () => { newRun(); enterGame(); };
document.getElementById('btn-over-menu').onclick = gotoTitle;
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestDepth', bestDepth());
  if (game && !overlaysClosed()) { if (!document.getElementById('overlay-shop').classList.contains('hidden')) renderShop(); }
});

// ---- loop --------------------------------------------------------------
function frame(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  animT += dt;
  if (game && !game.over && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    update(dt);
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(frame);
}

setInterval(() => { if (game && !game.over && overlaysClosed()) saveRun(); }, 5000);
addEventListener('beforeunload', saveRun);

gotoTitle();
requestAnimationFrame(frame);
