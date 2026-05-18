// Pixel Town Tycoon - engine, economy, UI
(() => {
'use strict';

const SAVE_KEY = 'pixel-town-save';
const $ = id => document.getElementById(id);
const canvas = $('game');
const ctx = canvas.getContext('2d');
const FIELD_W = GRID_W * TILE, FIELD_H = GRID_H * TILE;
canvas.width = FIELD_W; canvas.height = FIELD_H;
ctx.imageSmoothingEnabled = false;

// ---- save --------------------------------------------------------------
let S = null;          // persistent state
let placing = null;    // building type selected from palette
let selected = null;   // selected placed building
let floats = [];       // floating texts
let particles = [];
let tickAccum = 0;
let animClock = 0;
let rafId = 0, lastT = 0;

function loadSave() {
  try { return JSON.parse(localStorage.getItem(SAVE_KEY)); } catch (e) { return null; }
}
function persist() {
  S.lastTime = Date.now();
  localStorage.setItem(SAVE_KEY, JSON.stringify(S));
}

function newGame() {
  const blocked = [];
  while (blocked.length < 6) {
    const gx = Math.floor(Math.random() * GRID_W);
    const gy = Math.floor(Math.random() * GRID_H);
    if (!blocked.some(b => b[0] === gx && b[1] === gy)) blocked.push([gx, gy]);
  }
  const stock = {};
  RES_IDS.forEach(r => stock[r] = 0);
  const produced = {};
  RES_IDS.forEach(r => produced[r] = 0);
  S = {
    coins: 480, coinsEarned: 0, rank: 0, questIdx: 0,
    buildings: [], stock, produced, blocked, lastTime: Date.now(),
  };
  persist();
}

// ---- derived stats -----------------------------------------------------
let D = { population: 0, workersNeeded: 0, workerFactor: 1, storageCap: BASE_STORAGE };
function recalc() {
  let pop = 0, need = 0, cap = BASE_STORAGE;
  for (const b of S.buildings) {
    const def = BUILDINGS[b.type];
    if (def.kind === 'home') pop += def.pop * b.level;
    if (def.workers) need += def.workers;
    if (def.kind === 'storage') cap += def.cap * b.level;
  }
  D.population = pop;
  D.workersNeeded = need;
  D.workerFactor = need === 0 ? 1 : Math.max(0.2, Math.min(1, pop / need));
  D.storageCap = cap;
}
function buildingAt(gx, gy) {
  return S.buildings.find(b => b.gx === gx && b.gy === gy);
}
function isBlocked(gx, gy) {
  return S.blocked.some(b => b[0] === gx && b[1] === gy);
}
function adjBonus(b) {
  const def = BUILDINGS[b.type];
  if (!def.adj) return 1;
  let n = 0;
  for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
    const nb = buildingAt(b.gx + dx, b.gy + dy);
    if (nb && nb.type === def.adj) n++;
  }
  return 1 + 0.15 * n;
}

// ---- production tick ---------------------------------------------------
function productionTick(silent) {
  recalc();
  const wf = D.workerFactor, cap = D.storageCap;
  const add = (res, amt) => {
    S.stock[res] = Math.min(cap, (S.stock[res] || 0) + amt);
  };
  // order: producers -> tier1 processors -> tier2 processors -> market
  const order = b => {
    const def = BUILDINGS[b.type];
    if (def.kind === 'producer') return 0;
    if (def.kind === 'processor') return RESOURCES[def.out].tier;
    if (def.kind === 'market') return 9;
    return 8;
  };
  const list = S.buildings.slice().sort((a, b) => order(a) - order(b));
  for (const b of list) {
    const def = BUILDINGS[b.type];
    b.active = false;
    const adj = adjBonus(b);
    if (def.kind === 'producer') {
      const amt = buildingRate(def.rate, b.level) * wf * adj * TICK_SECONDS;
      S.produced[def.out] = (S.produced[def.out] || 0) + amt;
      add(def.out, amt);
      b.active = true;
    } else if (def.kind === 'processor') {
      let ok = true;
      for (const r in def.in) if ((S.stock[r] || 0) < def.in[r]) ok = false;
      if (ok) {
        for (const r in def.in) S.stock[r] -= def.in[r];
        const amt = buildingRate(def.rate, b.level) * wf * adj * TICK_SECONDS;
        S.produced[def.out] = (S.produced[def.out] || 0) + amt;
        add(def.out, amt);
        b.active = true;
      }
    } else if (def.kind === 'market') {
      let earned = 0;
      const cando = buildingRate(def.rate, b.level) * wf * adj * TICK_SECONDS;
      for (const r of RES_IDS) {
        const price = RESOURCES[r].sell;
        if (!price) continue;
        const sell = Math.min(S.stock[r] || 0, cando);
        if (sell > 0) {
          S.stock[r] -= sell;
          earned += sell * price;
          b.active = true;
        }
      }
      if (earned > 0) {
        S.coins += earned;
        S.coinsEarned += earned;
        if (!silent) {
          floats.push({ x: b.gx * TILE + TILE / 2, y: b.gy * TILE + 6,
            str: '+' + Math.round(earned), color: '#ffd34d', life: 1.3 });
        }
      }
    }
  }
  if (!silent) { checkRank(); checkQuest(); }
}

function checkRank() {
  while (S.rank + 1 < RANKS.length && S.coinsEarned >= RANKS[S.rank + 1].coins) {
    S.rank++;
    toast(t('rankUp', rankName(S.rank)), '#ffd34d');
    renderPalette();
  }
}
function questProgress(q) {
  if (q.type === 'build') return S.buildings.filter(b => b.type === q.target).length;
  if (q.type === 'pop') { recalc(); return D.population; }
  if (q.type === 'produced') return Math.floor((S.produced && S.produced[q.target]) || 0);
  if (q.type === 'earned') return Math.floor(S.coinsEarned);
  return 0;
}
function questGoal(q) { return q.n; }
function checkQuest() {
  while (S.questIdx < QUESTS.length) {
    const q = QUESTS[S.questIdx];
    if (questProgress(q) >= questGoal(q)) {
      S.coins += q.reward;
      S.coinsEarned += q.reward;
      toast(t('questDone') + ' +' + q.reward, '#5fe07a');
      S.questIdx++;
    } else break;
  }
}

// ---- placement ---------------------------------------------------------
function placeBuilding(gx, gy, type) {
  const def = BUILDINGS[type];
  if (def.rank > S.rank) { toast(t('locked'), '#ff7a7a'); return; }
  if (gx < 0 || gy < 0 || gx >= GRID_W || gy >= GRID_H) return;
  if (isBlocked(gx, gy)) { toast(t('blocked'), '#ff7a7a'); return; }
  if (buildingAt(gx, gy)) { toast(t('occupied'), '#ff7a7a'); return; }
  if (S.coins < def.cost) { toast(t('cantAfford'), '#ff7a7a'); return; }
  S.coins -= def.cost;
  S.buildings.push({ type, gx, gy, level: 1, active: false });
  for (let i = 0; i < 10; i++) particles.push(mkParticle(gx * TILE + TILE / 2, gy * TILE + TILE / 2, '#ffe9a8'));
  recalc(); checkQuest(); renderHud(); renderPalette(); persist();
}
function upgradeBuilding(b) {
  if (b.level >= 5) return;
  const cost = upgradeCost(b.type, b.level);
  if (S.coins < cost) { toast(t('cantAfford'), '#ff7a7a'); return; }
  S.coins -= cost;
  b.level++;
  for (let i = 0; i < 12; i++) particles.push(mkParticle(b.gx * TILE + TILE / 2, b.gy * TILE + TILE / 2, '#ffd34d'));
  recalc(); renderHud(); renderPanel(); persist();
}
function demolishBuilding(b) {
  S.coins += sellValue(b.type, b.level);
  S.buildings = S.buildings.filter(x => x !== b);
  selected = null;
  $('build-panel').classList.add('hidden');
  recalc(); renderHud(); renderPalette(); persist();
}

// ---- effects -----------------------------------------------------------
function mkParticle(x, y, color) {
  const a = Math.random() * 6.28, sp = 30 + Math.random() * 90;
  return { x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 30,
    life: 0.6, max: 0.6, size: 2 + Math.random() * 3, color };
}
function toast(msg, color) {
  $('toast').textContent = msg;
  $('toast').style.color = color || '#fff';
  $('toast').classList.remove('hidden');
  clearTimeout(toast._t);
  toast._t = setTimeout(() => $('toast').classList.add('hidden'), 2600);
}

// ---- offline -----------------------------------------------------------
function applyOffline() {
  const elapsed = Math.min(OFFLINE_CAP, (Date.now() - (S.lastTime || Date.now())) / 1000);
  const ticks = Math.floor(elapsed / TICK_SECONDS);
  if (ticks < 3) return;
  const before = S.coins;
  for (let i = 0; i < ticks; i++) productionTick(true);
  checkRank(); checkQuest();
  const gained = Math.round(S.coins - before);
  if (gained > 0) toast(t('welcomeBack', gained), '#ffd34d');
}

// ---- rendering ---------------------------------------------------------
function render() {
  for (let gy = 0; gy < GRID_H; gy++)
    for (let gx = 0; gx < GRID_W; gx++)
      drawTile(ctx, gx, gy);
  for (const b of S.blocked) drawBlocked(ctx, b[0], b[1]);

  // placement hover
  if (placing && hoverTile) {
    const ok = !isBlocked(hoverTile.x, hoverTile.y) && !buildingAt(hoverTile.x, hoverTile.y)
      && S.coins >= BUILDINGS[placing].cost && BUILDINGS[placing].rank <= S.rank;
    ctx.fillStyle = ok ? 'rgba(120,255,140,0.34)' : 'rgba(255,90,90,0.36)';
    ctx.fillRect(hoverTile.x * TILE, hoverTile.y * TILE, TILE, TILE);
    // adjacency preview
    const def = BUILDINGS[placing];
    if (def.adj) {
      ctx.strokeStyle = 'rgba(255,211,77,0.8)'; ctx.lineWidth = 3;
      for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
        const nb = buildingAt(hoverTile.x + dx, hoverTile.y + dy);
        if (nb && nb.type === def.adj)
          ctx.strokeRect((hoverTile.x + dx) * TILE + 2, (hoverTile.y + dy) * TILE + 2, TILE - 4, TILE - 4);
      }
    }
  }

  // buildings (sorted by row for overlap)
  const sorted = S.buildings.slice().sort((a, b) => a.gy - b.gy);
  for (const b of sorted) {
    drawBuilding(ctx, b.type, b.gx, b.gy, b.level, b.active, animClock);
    if (BUILDINGS[b.type].adj && adjBonus(b) > 1) {
      ctx.fillStyle = '#ffd34d';
      ctx.fillRect(b.gx * TILE + TILE - 12, b.gy * TILE + 4, 8, 8);
      ctx.fillStyle = '#a8780f';
      ctx.fillRect(b.gx * TILE + TILE - 12, b.gy * TILE + 4, 8, 2);
    }
  }
  // selection ring
  if (selected) {
    ctx.strokeStyle = '#ffd34d'; ctx.lineWidth = 3;
    ctx.setLineDash([6, 4]);
    ctx.strokeRect(selected.gx * TILE + 2, selected.gy * TILE + 2, TILE - 4, TILE - 4);
    ctx.setLineDash([]);
  }

  // particles
  for (const p of particles) {
    ctx.globalAlpha = Math.max(0, p.life / p.max);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x - p.size / 2, p.y - p.size / 2, p.size, p.size);
  }
  ctx.globalAlpha = 1;
  // floats
  for (const f of floats) drawFloatText(ctx, f.x, f.y, f.str, f.color, Math.min(1, f.life * 1.4));
}

let hoverTile = null;

// ---- HUD ---------------------------------------------------------------
function renderHud() {
  recalc();
  $('hud-coins').textContent = '◆ ' + Math.floor(S.coins);
  $('hud-pop').textContent = '👥 ' + D.population;
  const staffTag = $('hud-staff');
  if (D.workersNeeded > D.population) {
    staffTag.textContent = '⚠ ' + t('understaffed');
    staffTag.classList.add('warn');
  } else {
    staffTag.textContent = '✓ ' + t('staffed');
    staffTag.classList.remove('warn');
  }
  // rank
  $('hud-rank').textContent = rankName(S.rank);
  const next = RANKS[S.rank + 1];
  const fill = $('rank-fill');
  if (next) {
    const prev = RANKS[S.rank].coins;
    fill.style.width = Math.min(100, (S.coinsEarned - prev) / (next.coins - prev) * 100) + '%';
  } else fill.style.width = '100%';
  // resources
  const bar = $('res-bar');
  bar.innerHTML = '';
  for (const r of RES_IDS) {
    const chip = document.createElement('div');
    chip.className = 'res-chip';
    const amt = Math.floor(S.stock[r] || 0);
    chip.innerHTML = `<span class="res-ico">${RESOURCES[r].icon}</span>` +
      `<span class="res-amt">${amt}</span>`;
    if (amt >= D.storageCap) chip.classList.add('full');
    bar.appendChild(chip);
  }
  // current quest
  if (S.questIdx < QUESTS.length) {
    const q = QUESTS[S.questIdx];
    $('quest-mini').textContent = '🎯 ' + questText(q) +
      '  (' + Math.min(questProgress(q), questGoal(q)) + '/' + questGoal(q) + ')';
  } else {
    $('quest-mini').textContent = '🏆 ' + t('allQuests');
  }
}

// ---- build palette -----------------------------------------------------
function renderPalette() {
  const bar = $('palette');
  bar.innerHTML = '';
  for (const id of BUILDING_IDS) {
    const def = BUILDINGS[id];
    const locked = def.rank > S.rank;
    const card = document.createElement('button');
    card.className = 'pal-card' + (locked ? ' locked' : '') + (placing === id ? ' selected' : '');
    const cv = document.createElement('canvas');
    cv.width = 52; cv.height = 52;
    drawBuildingIcon(cv.getContext('2d'), id, 26, 10, 1.0);
    card.appendChild(cv);
    const nm = document.createElement('span');
    nm.className = 'pc-name';
    nm.textContent = bName(id);
    card.appendChild(nm);
    const co = document.createElement('span');
    co.className = 'pc-cost';
    co.textContent = locked ? '🔒' : '◆' + def.cost;
    card.appendChild(co);
    if (!locked) {
      card.onclick = () => {
        placing = placing === id ? null : id;
        selected = null;
        $('build-panel').classList.add('hidden');
        renderPalette();
      };
    }
    bar.appendChild(card);
  }
}

// ---- building info panel ----------------------------------------------
function renderPanel() {
  const panel = $('build-panel');
  if (!selected) { panel.classList.add('hidden'); return; }
  const b = selected, def = BUILDINGS[b.type];
  panel.classList.remove('hidden');
  recalc();
  const adj = adjBonus(b);
  let stats = '';
  if (def.kind === 'producer') {
    const r = buildingRate(def.rate, b.level) * D.workerFactor * adj;
    stats = `${t('produces')}: ${RESOURCES[def.out].icon} ${r.toFixed(1)}${t('perSec')}`;
  } else if (def.kind === 'processor') {
    const r = buildingRate(def.rate, b.level) * D.workerFactor * adj;
    const ins = Object.keys(def.in).map(k => RESOURCES[k].icon + def.in[k]).join(' ');
    stats = `${t('consumes')}: ${ins} → ${RESOURCES[def.out].icon} ${r.toFixed(1)}${t('perSec')}`;
  } else if (def.kind === 'market') {
    const r = buildingRate(def.rate, b.level) * D.workerFactor * adj;
    stats = `${t('sells')}: ${r.toFixed(1)}${t('perSec')} 🍞🔧`;
  } else if (def.kind === 'home') {
    stats = `${t('housing')}: 👥 ${def.pop * b.level}`;
  } else if (def.kind === 'storage') {
    stats = `${t('storage')}: +${def.cap * b.level}`;
  }
  const upCost = upgradeCost(b.type, b.level);
  const maxed = b.level >= 5;
  let html = `<div class="bp-head"><span class="bp-name">${bName(b.type)}</span>` +
    `<span class="bp-lvl">${t('level')}.${b.level}</span>` +
    `<button class="bp-x" id="bp-close">✕</button></div>` +
    `<div class="bp-stat">${stats}</div>`;
  if (def.adj) {
    html += `<div class="bp-adj ${adj > 1 ? 'on' : ''}">` +
      (adj > 1 ? '⭐ ' + t('adjBonus') + ' +' + Math.round((adj - 1) * 100) + '%'
               : t('adjHint', bName(def.adj))) + `</div>`;
  }
  html += `<div class="bp-actions">`;
  if (maxed) html += `<span class="bp-max">${t('maxLevel')}</span>`;
  else html += `<button class="bp-up" id="bp-up">${t('upgrade')} ◆${upCost}</button>`;
  html += `<button class="bp-sell" id="bp-sell">${t('demolish')} +${sellValue(b.type, b.level)}</button>`;
  html += `</div>`;
  panel.innerHTML = html;
  $('bp-close').onclick = () => { selected = null; panel.classList.add('hidden'); };
  if (!maxed) $('bp-up').onclick = () => upgradeBuilding(b);
  $('bp-sell').onclick = () => demolishBuilding(b);
}

// ---- quests overlay ----------------------------------------------------
function renderQuests() {
  const list = $('quest-list');
  list.innerHTML = '';
  QUESTS.forEach((q, i) => {
    const row = document.createElement('div');
    const done = i < S.questIdx;
    const active = i === S.questIdx;
    row.className = 'quest-row' + (done ? ' done' : '') + (active ? ' active' : '');
    const prog = done ? questGoal(q) : Math.min(questProgress(q), questGoal(q));
    row.innerHTML = `<span class="q-check">${done ? '✓' : active ? '▶' : '·'}</span>` +
      `<span class="q-text">${questText(q)}</span>` +
      `<span class="q-prog">${done ? '✓' : prog + '/' + questGoal(q)}</span>` +
      `<span class="q-reward">◆${q.reward}</span>`;
    list.appendChild(row);
  });
}

// ---- input -------------------------------------------------------------
let rect = null;
function updateRect() { rect = canvas.getBoundingClientRect(); }
function toField(cx, cy) {
  if (!rect) updateRect();
  return {
    x: (cx - rect.left) / rect.width * FIELD_W,
    y: (cy - rect.top) / rect.height * FIELD_H,
  };
}
canvas.addEventListener('pointerdown', e => {
  e.preventDefault();
  const p = toField(e.clientX, e.clientY);
  const gx = Math.floor(p.x / TILE), gy = Math.floor(p.y / TILE);
  if (gx < 0 || gy < 0 || gx >= GRID_W || gy >= GRID_H) return;
  const b = buildingAt(gx, gy);
  if (placing) {
    placeBuilding(gx, gy, placing);
    if (S.coins < BUILDINGS[placing].cost) { placing = null; renderPalette(); }
  } else if (b) {
    selected = b;
    renderPanel();
  } else {
    selected = null;
    $('build-panel').classList.add('hidden');
  }
});
canvas.addEventListener('pointermove', e => {
  if (!placing) { hoverTile = null; return; }
  const p = toField(e.clientX, e.clientY);
  hoverTile = { x: Math.floor(p.x / TILE), y: Math.floor(p.y / TILE) };
});
canvas.addEventListener('pointerleave', () => { hoverTile = null; });

// ---- loop --------------------------------------------------------------
function loop(now) {
  rafId = requestAnimationFrame(loop);
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.25) dt = 0.25;
  if (!S || $('screen-game').classList.contains('hidden')) return;
  animClock += dt;
  tickAccum += dt;
  while (tickAccum >= TICK_SECONDS) {
    tickAccum -= TICK_SECONDS;
    productionTick(false);
  }
  for (const p of particles) { p.life -= dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 200 * dt; }
  particles = particles.filter(p => p.life > 0);
  for (const f of floats) { f.life -= dt; f.y -= 26 * dt; }
  floats = floats.filter(f => f.life > 0);
  render();
  renderHud();
}

// ---- resize ------------------------------------------------------------
function resize() {
  const stage = $('stage');
  const sw = stage.clientWidth, sh = stage.clientHeight;
  const scl = Math.min(sw / FIELD_W, sh / FIELD_H);
  canvas.style.width = Math.floor(FIELD_W * scl) + 'px';
  canvas.style.height = Math.floor(FIELD_H * scl) + 'px';
  updateRect();
}
window.addEventListener('resize', resize);
window.addEventListener('orientationchange', () => setTimeout(resize, 200));

// ---- screens -----------------------------------------------------------
function showScreen(id) {
  $('screen-title').classList.toggle('hidden', id !== 'title');
  $('screen-game').classList.toggle('hidden', id !== 'game');
}
function startGame() {
  if (!S) newGame();
  // migrate older saves
  if (!S.produced) { S.produced = {}; RES_IDS.forEach(r => S.produced[r] = 0); }
  if (!S.stock) { S.stock = {}; RES_IDS.forEach(r => S.stock[r] = 0); }
  showScreen('game');
  applyOffline();
  renderPalette();
  renderHud();
  resize();
  persist();
}

function bindUI() {
  $('btn-play').onclick = startGame;
  $('btn-quests').onclick = () => { renderQuests(); $('overlay-quests').classList.remove('hidden'); };
  $('btn-quests-close').onclick = () => $('overlay-quests').classList.add('hidden');
  setupLanguageToggle(() => {
    renderPalette(); renderHud(); renderPanel();
    if (!$('overlay-quests').classList.contains('hidden')) renderQuests();
  });
  window.addEventListener('beforeunload', () => { if (S) persist(); });
  // periodic autosave
  setInterval(() => { if (S) persist(); }, 15000);
}

// ---- boot --------------------------------------------------------------
S = loadSave();
bindUI();
applyStaticText();
showScreen('title');
lastT = performance.now();
rafId = requestAnimationFrame(loop);

})();
