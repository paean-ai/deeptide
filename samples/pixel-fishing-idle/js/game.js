// Pixel Fishing Idle - delta-timed fishing idle with a stop-the-marker catch
// minigame, zones, orders, crew idle income, offline progress and save.

const SAVE_KEY = 'pixel-fishing-idle-save';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = W;
canvas.height = H;

const state = {
  coins: 0, rod: 1, bait: 1, boat: 1, crew: 1, zone: 0,
  weather: 'Calm', collection: {}, order: null,
  mode: 'ready', biteDelay: 0, waitT: 0, biteT: 0,
  markerPos: 0.5, markerDir: 1,
  ripples: [], sparkles: [], swimmers: [], caught: null, caughtFlash: 0,
  log: [], crewTimer: 0, weatherTimer: 0, lastSave: Date.now(),
};

// ---- costs -------------------------------------------------------------
function cost(key) { return Math.floor(UPGRADE[key].base * state[key] * state[key]); }
function rodCost() { return cost('rod'); }
function baitCost() { return cost('bait'); }
function boatCost() { return cost('boat'); }
function crewCost() { return cost('crew'); }

// ---- helpers -----------------------------------------------------------
function log(text) {
  state.log.unshift(text);
  state.log = state.log.slice(0, 4);
  document.getElementById('log').innerHTML = state.log.map(x => `<div>${x}</div>`).join('');
}

function newOrder() {
  const avail = FISH.filter(f => f.zone <= state.zone && f.rarity !== 'mythic');
  const f = avail[Math.floor(Math.random() * avail.length)];
  state.order = { id: f.id, need: 2 + Math.floor(state.boat / 2), have: 0,
    reward: Math.floor(f.value * (4 + state.zone)) };
}

function weatherBonus() {
  return state.weather === 'Lucky' ? 1.35 : state.weather === 'Storm' ? 1.15 : 1;
}
function rollWeather() {
  const r = Math.random();
  state.weather = r > 0.88 ? 'Lucky' : r > 0.72 ? 'Storm' : 'Calm';
}

function weightedFish() {
  const luck = 1 + state.bait * 0.16 + state.zone * 0.12 + (state.weather === 'Lucky' ? 0.35 : 0);
  const pool = FISH.filter(f => f.zone <= state.zone + (state.weather === 'Storm' ? 1 : 0) && f.zone <= ZONES.length - 1)
    .map(f => ({ ...f, rollWeight: Math.max(1, f.weight / (1 + RARITY[f.rarity].rank * luck * 0.55)) }));
  const total = pool.reduce((s, f) => s + f.rollWeight, 0);
  let roll = Math.random() * total;
  for (const f of pool) { roll -= f.rollWeight; if (roll <= 0) return f; }
  return pool[0];
}

function catchValue(f, mult) {
  return Math.floor(f.value * RARITY[f.rarity].mult * (1 + state.rod * 0.1) *
    (1 + state.boat * 0.08) * weatherBonus() * mult);
}

// ---- catch minigame ----------------------------------------------------
function bandHalf() { return Math.min(0.3, 0.11 + state.rod * 0.013); }
function markerSpeed() { return 0.82 + state.zone * 0.13; }

function reel() {
  if (state.mode === 'ready') {
    state.mode = 'waiting';
    state.waitT = 0;
    state.biteDelay = Math.max(0.5, 2.6 - state.rod * 0.13 - Math.random() * 1.4);
    state.caught = null;
    log(t('castIn', tZone(ZONES[state.zone].id), tWeather(state.weather)));
  } else if (state.mode === 'waiting') {
    state.mode = 'ready';
    log(t('early'));
  } else if (state.mode === 'bite') {
    const d = Math.abs(state.markerPos - 0.5);
    const half = bandHalf();
    if (d > half) { state.mode = 'ready'; log(`${t('miss')} ${t('tooLate')}`); return; }
    const perfect = d < half * 0.42;
    landFish(weightedFish(), perfect ? 1.6 : 1, perfect ? 'perfect' : 'good');
  }
}

function landFish(f, mult, quality) {
  const value = catchValue(f, mult);
  state.coins += value;
  state.mode = 'ready';
  state.caught = f;
  state.caughtFlash = 1;
  state.collection[f.id] = (state.collection[f.id] || 0) + 1;
  creditOrder(f);
  log(t('caught', t(quality), tFish(f.id), tRarity(f.rarity), value));
  splash(560, 360, f.color, f.rarity === 'mythic' ? 44 : quality === 'perfect' ? 30 : 20);
  updateBook();
  save();
}

function creditOrder(f) {
  if (state.order && state.order.id === f.id) {
    state.order.have++;
    if (state.order.have >= state.order.need) {
      state.coins += state.order.reward;
      log(t('orderDone', tFish(f.id), state.order.reward));
      newOrder();
    }
  }
}

function crewCatch() {
  const f = weightedFish();
  const value = Math.floor(catchValue(f, 1) * (0.22 + state.crew * 0.08));
  state.coins += value;
  state.collection[f.id] = (state.collection[f.id] || 0) + 1;
  creditOrder(f);
  splash(260 + Math.random() * 160, 392, f.color, 8);
  log(t('crewHaul', tFish(f.id), value));
  updateBook();
}

function splash(x, y, color, count) {
  for (let i = 0; i < count; i++) {
    state.ripples.push({ x: x + Math.random() * 80 - 40, y: y + Math.random() * 24 - 12, r: 2, life: 1, color });
    state.sparkles.push({ x, y, vx: (Math.random() - 0.5) * 260, vy: -Math.random() * 250 - 60, life: 1, color });
  }
}

// ---- ambient swimmers --------------------------------------------------
function spawnSwimmers() {
  state.swimmers = [];
  const pool = FISH.filter(f => f.zone <= state.zone);
  for (let i = 0; i < 5; i++) {
    const f = pool[Math.floor(Math.random() * pool.length)];
    state.swimmers.push({
      fish: f, x: Math.random() * W, y: 400 + Math.random() * 170,
      dir: Math.random() < 0.5 ? -1 : 1, speed: 18 + Math.random() * 34,
    });
  }
}

// ---- update ------------------------------------------------------------
function update(dt) {
  if (state.mode === 'waiting') {
    state.waitT += dt;
    if (state.waitT >= state.biteDelay) {
      state.mode = 'bite';
      state.biteT = 0;
      state.markerPos = Math.random();
      state.markerDir = Math.random() < 0.5 ? -1 : 1;
      log(t('bite'));
    }
  } else if (state.mode === 'bite') {
    state.biteT += dt;
    state.markerPos += state.markerDir * markerSpeed() * dt;
    if (state.markerPos <= 0) { state.markerPos = 0; state.markerDir = 1; }
    if (state.markerPos >= 1) { state.markerPos = 1; state.markerDir = -1; }
    if (state.biteT > 3.6) { state.mode = 'ready'; log(t('tooLate')); }
  }

  state.crewTimer += dt;
  const crewInterval = Math.max(2.4, 9 - state.boat * 0.6 - state.crew * 0.55);
  if (state.crewTimer >= crewInterval) {
    state.crewTimer = 0;
    if (state.crew > 1 || state.boat > 1) crewCatch();
  }
  state.weatherTimer += dt;
  if (state.weatherTimer >= 12) { state.weatherTimer = 0; if (Math.random() < 0.5) rollWeather(); }

  for (const r of state.ripples) { r.r += 27 * dt; r.life -= dt * 1.6; }
  state.ripples = state.ripples.filter(r => r.life > 0);
  for (const p of state.sparkles) { p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 520 * dt; p.life -= dt * 1.3; }
  state.sparkles = state.sparkles.filter(p => p.life > 0);
  for (const s of state.swimmers) {
    s.x += s.dir * s.speed * dt;
    if (s.x < -60) { s.x = W + 60; s.dir = -1; }
    if (s.x > W + 60) { s.x = -60; s.dir = 1; }
  }
  if (state.caughtFlash > 0) state.caughtFlash -= dt;
}

// ---- draw --------------------------------------------------------------
function draw(t) {
  const zone = ZONES[state.zone];
  const grd = ctx.createLinearGradient(0, 0, 0, H);
  grd.addColorStop(0, zone.sky[0]);
  grd.addColorStop(0.52, zone.sky[1]);
  grd.addColorStop(0.53, zone.sea[0]);
  grd.addColorStop(1, zone.sea[1]);
  ctx.fillStyle = grd;
  ctx.fillRect(0, 0, W, H);

  // sky body (sun / storm)
  ctx.fillStyle = state.weather === 'Storm' ? '#d8e6ff' : '#f4c85a';
  ctx.fillRect(690, 70 + Math.sin(t) * 5, 48, 48);
  if (state.weather === 'Lucky') {
    ctx.fillStyle = 'rgba(255, 241, 166, 0.25)';
    for (let i = 0; i < 16; i++) ctx.fillRect((i * 67 + t * 30) % W, 120 + (i % 5) * 52, 6, 6);
  }
  if (state.weather === 'Storm') {
    ctx.fillStyle = 'rgba(216,230,255,0.5)';
    for (let i = 0; i < 40; i++) ctx.fillRect((i * 53 + t * 260) % W, (i * 71 + t * 420) % 320, 2, 12);
  }
  if (state.zone >= 2) {
    ctx.fillStyle = '#edf4ff';
    for (let i = 0; i < 22; i++) ctx.fillRect((i * 39) % W, 30 + (i * 29) % 210, 3, 3);
  }

  // ambient swimming fish below the waterline
  for (const s of state.swimmers) drawFishSprite(ctx, s.x, s.y, s.fish, 2.1, t, s.dir > 0);

  // waterline highlights
  ctx.fillStyle = 'rgba(255,255,255,0.16)';
  for (let x = 0; x < W; x += 42) {
    const y = 360 + Math.sin(t * 2 + x * 0.03) * 6;
    ctx.fillRect(x, y, 36, 7);
  }
  if (state.zone === 1) {
    ctx.fillStyle = '#185b45';
    for (let x = 30; x < W; x += 70) ctx.fillRect(x, 456 + Math.sin(t + x) * 10, 12, 88);
  }

  const bob = drawBoat(ctx, t, state.crew);

  // line + bobber
  if (state.mode !== 'ready') {
    const by = 365 + Math.sin(t * 7) * 8;
    ctx.strokeStyle = state.mode === 'bite' ? '#f4c85a' : '#edf4ff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(595, 286 + bob);
    ctx.lineTo(570, by);
    ctx.stroke();
    ctx.fillStyle = state.mode === 'bite' ? '#f4c85a' : '#edf4ff';
    ctx.fillRect(566, by, 8, 8);
    if (state.mode === 'waiting' && state.waitT > state.biteDelay - 0.6) {
      drawLureShadow(ctx, 570, by + 26, t, zone.accent);
    }
  }

  // splashes
  for (const r of state.ripples) {
    ctx.globalAlpha = Math.max(0, r.life);
    ctx.strokeStyle = r.color;
    ctx.lineWidth = 2;
    ctx.strokeRect(r.x - r.r, r.y - r.r / 2, r.r * 2, r.r);
  }
  ctx.globalAlpha = 1;
  for (const p of state.sparkles) {
    ctx.globalAlpha = Math.max(0, p.life);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x, p.y, 5, 5);
  }
  ctx.globalAlpha = 1;

  // freshly caught fish on display, with a pop
  if (state.caught) {
    const pop = 1 + Math.max(0, state.caughtFlash) * 0.5;
    drawFishSprite(ctx, 450, 176, state.caught, 3 * pop, t, true);
  }

  // bite bar minigame
  if (state.mode === 'bite') {
    const bx = 300, bw = 300, by = 96;
    ctx.fillStyle = 'rgba(5, 9, 15, 0.74)';
    ctx.fillRect(bx - 6, by - 8, bw + 12, 40);
    ctx.fillStyle = '#263243';
    ctx.fillRect(bx, by, bw, 14);
    const half = bandHalf();
    ctx.fillStyle = '#3a7d52';
    ctx.fillRect(bx + (0.5 - half) * bw, by, half * 2 * bw, 14);
    ctx.fillStyle = '#72df89';
    ctx.fillRect(bx + (0.5 - half * 0.42) * bw, by, half * 0.84 * bw, 14);
    ctx.fillStyle = '#f4c85a';
    ctx.fillRect(bx + state.markerPos * bw - 3, by - 6, 6, 26);
  }
}

// ---- hud ---------------------------------------------------------------
function updateHud() {
  document.getElementById('coins').textContent = Math.floor(state.coins);
  document.getElementById('rod').textContent = state.rod;
  document.getElementById('bait').textContent = state.bait;
  document.getElementById('boat').textContent = state.boat;
  document.getElementById('zone').textContent = tZone(ZONES[state.zone].id);
  document.getElementById('order').textContent = state.order
    ? `${state.order.have}/${state.order.need} ${tFish(state.order.id)}` : '-';
  document.getElementById('cast').textContent =
    state.mode === 'ready' ? t('castLine') : state.mode === 'bite' ? t('reelIn') : t('pullBack');
  const setBtn = (id, label, c) => {
    const el = document.getElementById(id);
    el.textContent = `${label} ${c}`;
    el.disabled = state.coins < c;
  };
  setBtn('rod-up', t('upRod'), rodCost());
  setBtn('bait-up', t('upBait'), baitCost());
  setBtn('boat-up', t('upBoat'), boatCost());
  setBtn('crew-up', t('upCrew'), crewCost());
  const next = (state.zone + 1) % ZONES.length;
  document.getElementById('zone-next').textContent = `${t('sail')} ${tZone(ZONES[next].id)}`;
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
}

function updateBook() {
  const seen = FISH.filter(f => state.collection[f.id]);
  document.getElementById('book').innerHTML = seen.map(f =>
    `<span style="border-color:${RARITY[f.rarity].color};color:${f.color}">${tFish(f.id)} x${state.collection[f.id]}</span>`
  ).join('');
}

// ---- upgrades ----------------------------------------------------------
function buy(key, costFn) {
  const c = costFn();
  if (state.coins < c) return;
  state.coins -= c;
  state[key]++;
  log(tUp(key));
  save();
}

function sail() {
  const next = (state.zone + 1) % ZONES.length;
  if (state.boat < ZONES[next].unlock) {
    log(t('needBoat', tZone(ZONES[next].id), ZONES[next].unlock));
    return;
  }
  state.zone = next;
  rollWeather();
  newOrder();
  spawnSwimmers();
  log(t('sailed', tZone(ZONES[state.zone].id), tWeather(state.weather)));
  save();
}

// ---- save / load / offline --------------------------------------------
function save() {
  state.lastSave = Date.now();
  try {
    localStorage.setItem(SAVE_KEY, JSON.stringify({
      coins: state.coins, rod: state.rod, bait: state.bait, boat: state.boat,
      crew: state.crew, zone: state.zone, weather: state.weather,
      collection: state.collection, order: state.order, lastSave: state.lastSave,
    }));
  } catch (e) { /* storage unavailable */ }
}

function load() {
  let d;
  try { d = JSON.parse(localStorage.getItem(SAVE_KEY)); } catch (e) { d = null; }
  if (!d) { newOrder(); rollWeather(); return; }
  Object.assign(state, {
    coins: d.coins || 0, rod: d.rod || 1, bait: d.bait || 1, boat: d.boat || 1,
    crew: d.crew || 1, zone: d.zone || 0, weather: d.weather || 'Calm',
    collection: d.collection || {}, order: d.order || null,
  });
  if (!state.order) newOrder();
  // offline crew income estimate
  const elapsed = Math.min(OFFLINE_CAP_SECONDS, Math.max(0, (Date.now() - (d.lastSave || Date.now())) / 1000));
  if (elapsed > 30 && (state.crew > 1 || state.boat > 1)) {
    const crewInterval = Math.max(2.4, 9 - state.boat * 0.6 - state.crew * 0.55);
    const hauls = elapsed / crewInterval;
    const avg = catchValue(FISH[0], 1) * (0.22 + state.crew * 0.08) * (1 + state.zone * 1.5);
    const earned = Math.floor(hauls * avg);
    if (earned > 0) {
      state.coins += earned;
      setTimeout(() => log(t('welcomeBack', earned)), 300);
    }
  }
}

// ---- input -------------------------------------------------------------
document.getElementById('cast').onclick = reel;
document.getElementById('rod-up').onclick = () => buy('rod', rodCost);
document.getElementById('bait-up').onclick = () => buy('bait', baitCost);
document.getElementById('boat-up').onclick = () => buy('boat', boatCost);
document.getElementById('crew-up').onclick = () => buy('crew', crewCost);
document.getElementById('zone-next').onclick = sail;
document.addEventListener('keydown', e => {
  if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); reel(); }
});
canvas.addEventListener('pointerdown', e => { e.preventDefault(); reel(); });
setupLanguageToggle(() => { updateHud(); updateBook(); });

// ---- loop --------------------------------------------------------------
let lastT = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  update(dt);
  draw(now / 1000);
  updateHud();
  requestAnimationFrame(loop);
}

setInterval(save, 5000);
addEventListener('beforeunload', save);

load();
spawnSwimmers();
updateBook();
log(t('howStart'));
requestAnimationFrame(loop);
