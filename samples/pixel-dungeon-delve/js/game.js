// Pixel Dungeon Delve - turn-based roguelike crawler.
// Procedural floors, fog of war, bump combat, gear, depth-scaled difficulty.

const SAVE_KEY = 'pixel-dungeon-delve-run';
const BEST_KEY = 'pixel-dungeon-delve-best';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VIEW * TILE;
canvas.height = VIEW * TILE;
ctx.imageSmoothingEnabled = false;

let game = null;          // current run, or null on the title screen
let animT = 0;            // free-running clock for sprite idle animation
let busy = false;         // input lock while a turn resolves

const rand = n => Math.floor(Math.random() * n);
const idx = (x, y) => y * MAP_W + x;
const inBounds = (x, y) => x >= 0 && y >= 0 && x < MAP_W && y < MAP_H;

// ---- floor generation --------------------------------------------------
function overlap(a, b, pad) {
  return a.x - pad < b.x + b.w && a.x + a.w + pad > b.x &&
         a.y - pad < b.y + b.h && a.y + a.h + pad > b.y;
}

function carveCorridor(map, variant, x0, y0, x1, y1) {
  const put = (x, y) => {
    if (!inBounds(x, y)) return;
    if (map[idx(x, y)] === T_WALL) { map[idx(x, y)] = T_FLOOR; variant[idx(x, y)] = (x * 7 + y * 13) % 5; }
  };
  let x = x0, y = y0;
  while (x !== x1) { put(x, y); x += x < x1 ? 1 : -1; }
  while (y !== y1) { put(x, y); y += y < y1 ? 1 : -1; }
  put(x1, y1);
}

function freeTileInRoom(room, map, taken) {
  for (let i = 0; i < 30; i++) {
    const x = room.x + 1 + rand(room.w - 2);
    const y = room.y + 1 + rand(room.h - 2);
    if (map[idx(x, y)] === T_FLOOR && !taken.has(idx(x, y))) return { x, y };
  }
  return null;
}

function makeEnemy(glyph, x, y, depth) {
  const d = ENEMIES[glyph], s = depthScale(depth);
  return {
    glyph, x, y, boss: !!d.boss, erratic: !!d.erratic, sight: d.sight,
    hp: Math.round(d.hp * s), maxHp: Math.round(d.hp * s),
    atk: Math.round(d.atk * s), def: d.def + Math.floor(depth / 5),
    xp: d.xp, flash: false,
  };
}

function genFloor(depth) {
  const map = new Uint8Array(MAP_W * MAP_H);
  const variant = new Uint8Array(MAP_W * MAP_H);
  const b = floorBudget(depth);
  const rooms = [];
  let tries = 0;
  while (rooms.length < b.rooms && tries < 240) {
    tries++;
    const w = 5 + rand(5), h = 5 + rand(4);
    const x = 1 + rand(MAP_W - w - 2), y = 1 + rand(MAP_H - h - 2);
    const room = { x, y, w, h, cx: x + (w >> 1), cy: y + (h >> 1) };
    if (rooms.some(r => overlap(r, room, 1))) continue;
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) {
      map[idx(xx, yy)] = T_FLOOR;
      variant[idx(xx, yy)] = (xx * 7 + yy * 13) % 5;
    }
    rooms.push(room);
  }
  for (let i = 1; i < rooms.length; i++) {
    carveCorridor(map, variant, rooms[i - 1].cx, rooms[i - 1].cy, rooms[i].cx, rooms[i].cy);
  }
  const start = rooms[0];
  const exit = rooms[rooms.length - 1];
  map[idx(exit.cx, exit.cy)] = T_STAIRS;

  const taken = new Set([idx(start.cx, start.cy), idx(exit.cx, exit.cy)]);
  const enemies = [], items = [];
  const scatter = (count, fn) => {
    for (let i = 0; i < count; i++) {
      const room = rooms[1 + rand(Math.max(1, rooms.length - 1))];
      const spot = freeTileInRoom(room, map, taken);
      if (spot) { taken.add(idx(spot.x, spot.y)); fn(spot); }
    }
  };

  const pool = Object.keys(ENEMIES).filter(g => !ENEMIES[g].boss && ENEMIES[g].tier <= depth);
  scatter(b.enemies, s => enemies.push(makeEnemy(pool[rand(pool.length)], s.x, s.y, depth)));
  if (depth === VICTORY_DEPTH) {
    enemies.push(makeEnemy('dragon', exit.cx, exit.cy - 1, depth));
  }
  scatter(b.potions, s => items.push({ x: s.x, y: s.y, kind: 'potion' }));
  scatter(b.gold, s => items.push({ x: s.x, y: s.y, kind: 'gold', amount: 8 + rand(14 + depth * 3) }));
  for (let i = 0; i < b.gear; i++) {
    scatter(1, s => {
      if ((i + depth) % 2 === 0) {
        const tier = Math.min(WEAPONS.length - 1, Math.max(0, Math.floor(depth / 2) - 1 + rand(2)));
        items.push({ x: s.x, y: s.y, kind: 'weapon', tier });
      } else {
        const tier = Math.min(ARMORS.length - 1, Math.max(0, Math.floor(depth / 2) - 1 + rand(2)));
        items.push({ x: s.x, y: s.y, kind: 'armor', tier });
      }
    });
  }

  return {
    map, variant,
    seen: new Uint8Array(MAP_W * MAP_H),
    visible: new Uint8Array(MAP_W * MAP_H),
    enemies, items,
    startX: start.cx, startY: start.cy,
  };
}

// ---- run lifecycle -----------------------------------------------------
function newRun() {
  game = {
    depth: 1,
    player: {
      x: 0, y: 0, facing: 1, flash: false,
      hp: PLAYER_BASE.maxHp, maxHp: PLAYER_BASE.maxHp,
      atk: PLAYER_BASE.atk, def: PLAYER_BASE.def,
      level: 1, xp: 0, gold: 0, potions: 2, weapon: 0, armor: 0,
    },
    log: [], over: false, won: false, victoryShown: false,
  };
  loadFloor(genFloor(1));
  game.player.potions = 2;
  pushLog(t('descend', 1));
  computeFOV();
}

function loadFloor(floor) {
  Object.assign(game, {
    map: floor.map, variant: floor.variant, seen: floor.seen, visible: floor.visible,
    enemies: floor.enemies, items: floor.items,
  });
  game.player.x = floor.startX;
  game.player.y = floor.startY;
}

function descend() {
  game.depth++;
  loadFloor(genFloor(game.depth));
  pushLog(t('descend', game.depth));
  computeFOV();
  saveRun();
  render();
}

// ---- field of view -----------------------------------------------------
function computeFOV() {
  game.visible.fill(0);
  const { x: px, y: py } = game.player;
  for (let ty = py - FOV_RADIUS; ty <= py + FOV_RADIUS; ty++) {
    for (let tx = px - FOV_RADIUS; tx <= px + FOV_RADIUS; tx++) {
      if (!inBounds(tx, ty)) continue;
      if ((tx - px) ** 2 + (ty - py) ** 2 > FOV_RADIUS * FOV_RADIUS + 3) continue;
      castRay(px, py, tx, ty);
    }
  }
}
function castRay(x0, y0, x1, y1) {
  let dx = Math.abs(x1 - x0), dy = Math.abs(y1 - y0);
  const sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
  let err = dx - dy, x = x0, y = y0;
  for (;;) {
    game.visible[idx(x, y)] = 1;
    game.seen[idx(x, y)] = 1;
    if (game.map[idx(x, y)] === T_WALL) return;
    if (x === x1 && y === y1) return;
    const e2 = 2 * err;
    if (e2 > -dy) { err -= dy; x += sx; }
    if (e2 < dx) { err += dx; y += sy; }
  }
}
function hasLOS(x0, y0, x1, y1) {
  let dx = Math.abs(x1 - x0), dy = Math.abs(y1 - y0);
  const sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
  let err = dx - dy, x = x0, y = y0;
  for (;;) {
    if ((x !== x0 || y !== y0) && (x !== x1 || y !== y1) && game.map[idx(x, y)] === T_WALL) return false;
    if (x === x1 && y === y1) return true;
    const e2 = 2 * err;
    if (e2 > -dy) { err -= dy; x += sx; }
    if (e2 < dx) { err += dx; y += sy; }
  }
}

// ---- combat / stats ----------------------------------------------------
function totalAtk() { return game.player.atk + WEAPONS[game.player.weapon].atk; }
function totalDef() { return game.player.def + ARMORS[game.player.armor].def; }

function rollDamage(atk, def) {
  const base = atk - def;
  const variance = Math.round(atk * 0.18);
  return Math.max(1, base + rand(variance + 1) - rand(variance + 1));
}

function enemyAt(x, y) { return game.enemies.find(e => e.x === x && e.y === y); }

function playerAttack(e) {
  const dmg = rollDamage(totalAtk(), e.def);
  e.hp -= dmg;
  e.flash = true;
  pushLog(t('hitFor', tEnemy(e.glyph), dmg));
  if (e.hp <= 0) {
    game.enemies = game.enemies.filter(o => o !== e);
    pushLog(t('slain', tEnemy(e.glyph)));
    gainXp(e.xp);
    if (e.boss) winRun();
  }
}

function enemyAttack(e) {
  const dmg = rollDamage(e.atk, totalDef());
  game.player.hp -= dmg;
  game.player.flash = true;
  pushLog(t('tookHit', tEnemy(e.glyph), dmg));
  if (game.player.hp <= 0) { game.player.hp = 0; loseRun(); }
}

function gainXp(amount) {
  const p = game.player;
  p.xp += amount;
  while (p.xp >= xpForLevel(p.level)) {
    p.xp -= xpForLevel(p.level);
    p.level++;
    p.maxHp += 14; p.atk += 2; p.def += 1;
    p.hp = p.maxHp;
    pushLog(t('levelUp', p.level));
  }
}

// ---- turn loop ---------------------------------------------------------
function step(dx, dy) {
  if (busy || !game || game.over || !overlaysClosed()) return;
  const p = game.player;
  if (dx === 0 && dy === 0) { pushLog(t('waitTurn')); resolveTurn(); return; }
  p.facing = dx !== 0 ? Math.sign(dx) : p.facing;
  const nx = p.x + dx, ny = p.y + dy;
  if (!inBounds(nx, ny) || game.map[idx(nx, ny)] === T_WALL) return; // bump wall = free
  const e = enemyAt(nx, ny);
  if (e) { playerAttack(e); resolveTurn(); return; }
  p.x = nx; p.y = ny;
  pickup();
  if (game.over) { render(); return; }
  if (game.map[idx(nx, ny)] === T_STAIRS) { descend(); return; }
  resolveTurn();
}

function usePotion() {
  if (busy || !game || game.over || !overlaysClosed()) return;
  const p = game.player;
  if (p.potions <= 0) { pushLog(t('needPotion')); render(); return; }
  p.potions--;
  p.hp = Math.min(p.maxHp, p.hp + Math.floor(p.maxHp * 0.45));
  pushLog(t('usedPotion'));
  resolveTurn();
}

function pickup() {
  const p = game.player;
  const item = game.items.find(it => it.x === p.x && it.y === p.y);
  if (!item) return;
  game.items = game.items.filter(it => it !== item);
  if (item.kind === 'potion') { p.potions++; pushLog(t('quaff')); }
  else if (item.kind === 'gold') { p.gold += item.amount; pushLog(t('gotGold', item.amount)); }
  else if (item.kind === 'weapon') {
    if (WEAPONS[item.tier].atk > WEAPONS[p.weapon].atk) {
      p.weapon = item.tier; pushLog(t('foundWeapon', tGear(WEAPONS[item.tier].id)));
    } else { p.gold += 12; pushLog(t('gotGold', 12)); }
  } else if (item.kind === 'armor') {
    if (ARMORS[item.tier].def > ARMORS[p.armor].def) {
      p.armor = item.tier; pushLog(t('foundArmor', tGear(ARMORS[item.tier].id)));
    } else { p.gold += 12; pushLog(t('gotGold', 12)); }
  }
}

function enemiesAct() {
  const p = game.player;
  for (const e of game.enemies) {
    if (game.over) break;
    const dist2 = (e.x - p.x) ** 2 + (e.y - p.y) ** 2;
    const aware = dist2 <= e.sight * e.sight && hasLOS(e.x, e.y, p.x, p.y);
    if (aware && Math.max(Math.abs(e.x - p.x), Math.abs(e.y - p.y)) === 1) {
      enemyAttack(e);
      continue;
    }
    let mx = 0, my = 0;
    if (aware && !(e.erratic && rand(2))) {
      mx = Math.sign(p.x - e.x);
      my = Math.sign(p.y - e.y);
    } else if (e.erratic || (aware && rand(3) === 0)) {
      mx = rand(3) - 1; my = rand(3) - 1;
    }
    if (mx === 0 && my === 0) continue;
    const tryMoves = [[mx, my], [mx, 0], [0, my]];
    for (const [dx, dy] of tryMoves) {
      if (dx === 0 && dy === 0) continue;
      const tx = e.x + dx, ty = e.y + dy;
      if (!inBounds(tx, ty) || game.map[idx(tx, ty)] === T_WALL) continue;
      if (tx === p.x && ty === p.y) continue;
      if (enemyAt(tx, ty)) continue;
      e.x = tx; e.y = ty;
      break;
    }
  }
}

function resolveTurn() {
  enemiesAct();
  computeFOV();
  render();
  saveRun();
  // brief hit flash, then clear
  if (game.player.flash || game.enemies.some(e => e.flash)) {
    busy = true;
    setTimeout(() => {
      game.player.flash = false;
      game.enemies.forEach(e => e.flash = false);
      busy = false;
      if (game) render();
    }, 110);
  }
}

function overlaysClosed() {
  return document.querySelectorAll('.overlay:not(.hidden)').length === 0;
}

// ---- win / lose --------------------------------------------------------
function bestDepth() { return +(localStorage.getItem(BEST_KEY) || 1); }
function recordDepth() {
  if (game.depth > bestDepth()) localStorage.setItem(BEST_KEY, game.depth);
}

function loseRun() {
  game.over = true;
  recordDepth();
  localStorage.removeItem(SAVE_KEY);
  document.getElementById('over-msg').textContent = t('reachedDepth', game.depth);
  document.getElementById('over-best').textContent = t('bestDepth', bestDepth());
  showOverlay('overlay-over');
}

function winRun() {
  if (game.victoryShown) return;
  game.victoryShown = true;
  recordDepth();
  document.getElementById('victory-msg').textContent = t('victoryMsg');
  showOverlay('overlay-victory');
}

// ---- rendering ---------------------------------------------------------
function render() {
  ctx.fillStyle = '#07060c';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  const half = (VIEW - 1) >> 1;
  const camX = game.player.x - half, camY = game.player.y - half;

  for (let vy = 0; vy < VIEW; vy++) {
    for (let vx = 0; vx < VIEW; vx++) {
      const wx = camX + vx, wy = camY + vy;
      if (!inBounds(wx, wy) || !game.seen[idx(wx, wy)]) continue;
      const ox = vx * TILE, oy = vy * TILE;
      const tile = game.map[idx(wx, wy)];
      if (tile === T_WALL) drawWall(ctx, ox, oy);
      else if (tile === T_STAIRS) drawStairs(ctx, ox, oy);
      else drawFloor(ctx, ox, oy, game.variant[idx(wx, wy)]);
      if (!game.visible[idx(wx, wy)]) {
        ctx.fillStyle = 'rgba(7,6,12,0.62)';
        ctx.fillRect(ox, oy, TILE, TILE);
      }
    }
  }
  for (const it of game.items) {
    if (!game.visible[idx(it.x, it.y)]) continue;
    drawItem(ctx, (it.x - camX) * TILE, (it.y - camY) * TILE, it.kind, animT);
  }
  for (const e of game.enemies) {
    if (!game.visible[idx(e.x, e.y)]) continue;
    const ox = (e.x - camX) * TILE, oy = (e.y - camY) * TILE;
    drawEnemy(ctx, ox, oy, e.glyph, e.flash, animT);
    drawHpPip(ox, oy, e.hp / e.maxHp);
  }
  drawHero(ctx, half * TILE, half * TILE, game.player.facing, game.player.flash);
  updateHud();
}

function drawHpPip(ox, oy, frac) {
  if (frac >= 1) return;
  ctx.fillStyle = '#1a1420';
  ctx.fillRect(ox + 4, oy - 5, TILE - 8, 4);
  ctx.fillStyle = frac > 0.5 ? '#62d879' : frac > 0.25 ? '#f4c85a' : '#ff5d5d';
  ctx.fillRect(ox + 5, oy - 4, Math.max(1, (TILE - 10) * frac), 2);
}

function updateHud() {
  const p = game.player;
  document.getElementById('hud-depth').textContent = `${t('depth')} ${game.depth}`;
  document.getElementById('hud-lvl').textContent = `${t('lvl')} ${p.level}`;
  document.getElementById('hud-gold').textContent = `◆ ${p.gold}`;
  const hpFrac = p.hp / p.maxHp;
  document.getElementById('hp-fill').style.width = (hpFrac * 100) + '%';
  document.getElementById('hp-fill').style.background =
    hpFrac > 0.5 ? '#62d879' : hpFrac > 0.25 ? '#f4c85a' : '#ff5d5d';
  document.getElementById('hp-text').textContent = `${p.hp}/${p.maxHp}`;
  document.getElementById('xp-fill').style.width = (p.xp / xpForLevel(p.level) * 100) + '%';
  document.getElementById('hud-weapon').textContent = `⚔ ${tGear(WEAPONS[p.weapon].id)} +${totalAtk()}`;
  document.getElementById('hud-armor').textContent = `🛡 ${tGear(ARMORS[p.armor].id)} +${totalDef()}`;
  document.getElementById('btn-potion').textContent = `🧪 ${t('potionTag')} ${p.potions}`;
  document.getElementById('btn-potion').disabled = p.potions <= 0;
  const logEl = document.getElementById('log');
  logEl.innerHTML = game.log.slice(-4).map(m => `<div>${m}</div>`).join('');
}

function pushLog(msg) {
  game.log.push(msg);
  if (game.log.length > 30) game.log.shift();
}

// ---- save / load -------------------------------------------------------
function saveRun() {
  if (!game || game.over) return;
  try {
    localStorage.setItem(SAVE_KEY, JSON.stringify({
      depth: game.depth, player: game.player, log: game.log.slice(-8),
      victoryShown: game.victoryShown,
      map: Array.from(game.map), variant: Array.from(game.variant),
      seen: Array.from(game.seen),
      enemies: game.enemies, items: game.items,
    }));
  } catch (e) { /* storage unavailable */ }
}
function hasSavedRun() {
  try { return !!localStorage.getItem(SAVE_KEY); } catch (e) { return false; }
}
function loadRun() {
  let d;
  try { d = JSON.parse(localStorage.getItem(SAVE_KEY)); } catch (e) { return false; }
  if (!d || !d.map) return false;
  game = {
    depth: d.depth, player: d.player, log: d.log || [],
    over: false, won: false, victoryShown: d.victoryShown || false,
    map: Uint8Array.from(d.map), variant: Uint8Array.from(d.variant),
    seen: Uint8Array.from(d.seen), visible: new Uint8Array(MAP_W * MAP_H),
    enemies: d.enemies, items: d.items,
  };
  computeFOV();
  return true;
}

// ---- screens / overlays ------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideOverlay(id) { document.getElementById(id).classList.add('hidden'); }
function hideAllOverlays() { document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden')); }

function enterGame() {
  hideAllOverlays();
  showScreen('screen-game');
  fitCanvas();
  render();
}
function gotoTitle() {
  hideAllOverlays();
  document.getElementById('btn-resume').classList.toggle('hidden', !hasSavedRun());
  document.getElementById('title-best').textContent = t('bestDepth', bestDepth());
  showScreen('screen-title');
}

// ---- responsive canvas -------------------------------------------------
function fitCanvas() {
  const stage = document.getElementById('stage');
  const size = Math.min(stage.clientWidth, stage.clientHeight);
  canvas.style.width = size + 'px';
  canvas.style.height = size + 'px';
}
addEventListener('resize', () => { if (game && !document.getElementById('screen-game').classList.contains('hidden')) fitCanvas(); });

// ---- input -------------------------------------------------------------
const KEYS = {
  ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0],
  w: [0, -1], s: [0, 1], a: [-1, 0], d: [1, 0],
  W: [0, -1], S: [0, 1], A: [-1, 0], D: [1, 0],
};
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  if (KEYS[e.key]) { e.preventDefault(); step(KEYS[e.key][0], KEYS[e.key][1]); }
  else if (e.key === ' ' || e.key === '.') { e.preventDefault(); step(0, 0); }
  else if (e.key === 'q' || e.key === 'Q') usePotion();
  else if (e.key === 'Escape') togglePause();
});

// d-pad + action buttons
[['btn-up', 0, -1], ['btn-down', 0, 1], ['btn-left', -1, 0], ['btn-right', 1, 0]].forEach(([id, dx, dy]) => {
  document.getElementById(id).addEventListener('click', () => step(dx, dy));
});
document.getElementById('btn-wait').addEventListener('click', () => step(0, 0));
document.getElementById('btn-potion').addEventListener('click', usePotion);

// swipe on canvas
let touch = null;
canvas.addEventListener('touchstart', e => {
  touch = { x: e.touches[0].clientX, y: e.touches[0].clientY };
}, { passive: true });
canvas.addEventListener('touchend', e => {
  if (!touch) return;
  const dx = e.changedTouches[0].clientX - touch.x;
  const dy = e.changedTouches[0].clientY - touch.y;
  touch = null;
  if (Math.abs(dx) < 24 && Math.abs(dy) < 24) { step(0, 0); return; }
  if (Math.abs(dx) > Math.abs(dy)) step(Math.sign(dx), 0);
  else step(0, Math.sign(dy));
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
document.getElementById('btn-over-again').onclick = () => { newRun(); enterGame(); };
document.getElementById('btn-over-menu').onclick = gotoTitle;
document.getElementById('btn-victory-continue').onclick = () => hideOverlay('overlay-victory');
document.getElementById('btn-victory-menu').onclick = () => { saveRun(); gotoTitle(); };
setupLanguageToggle(() => { if (game) render(); gotoTitleTextRefresh(); });

function gotoTitleTextRefresh() {
  document.getElementById('title-best').textContent = t('bestDepth', bestDepth());
  if (game && game.over) {
    document.getElementById('over-msg').textContent = t('reachedDepth', game.depth);
    document.getElementById('over-best').textContent = t('bestDepth', bestDepth());
  }
}

// idle sprite animation - cheap rAF that only repaints the game screen
function tick() {
  animT += 0.05;
  if (game && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    render();
  }
  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);

gotoTitle();
