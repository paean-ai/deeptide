const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const W = 900;
const H = 620;

const ZONES = [
  { name: 'Cove', unlock: 1, sky: ['#17304f', '#295b79'], sea: ['#14506e', '#082b47'], accent: '#65d9ff' },
  { name: 'Kelp Bay', unlock: 2, sky: ['#203d47', '#407566'], sea: ['#0f604f', '#07392e'], accent: '#68da86' },
  { name: 'Moon Reef', unlock: 4, sky: ['#241f4d', '#544587'], sea: ['#29315e', '#111936'], accent: '#b7a7ff' },
  { name: 'Sunken Crown', unlock: 7, sky: ['#40294d', '#7d4f62'], sea: ['#263253', '#121827'], accent: '#f4c85a' },
];

const RARITY = {
  common: { mult: 1, color: '#9ee8ff' },
  uncommon: { mult: 2.1, color: '#72df89' },
  rare: { mult: 5, color: '#f4c85a' },
  epic: { mult: 12, color: '#b7a7ff' },
  mythic: { mult: 30, color: '#ffffff' },
};

const FISH = [
  { name: 'Glass Minnow', zone: 0, rarity: 'common', value: 5, weight: 48, color: '#65d9ff' },
  { name: 'Copper Carp', zone: 0, rarity: 'uncommon', value: 15, weight: 24, color: '#d68a4a' },
  { name: 'Lantern Koi', zone: 0, rarity: 'rare', value: 45, weight: 8, color: '#f4c85a' },
  { name: 'Kelp Pike', zone: 1, rarity: 'common', value: 18, weight: 42, color: '#68da86' },
  { name: 'Emerald Eel', zone: 1, rarity: 'rare', value: 82, weight: 10, color: '#2ee6a6' },
  { name: 'Moonfin', zone: 2, rarity: 'uncommon', value: 70, weight: 28, color: '#b7a7ff' },
  { name: 'Star Ray', zone: 2, rarity: 'epic', value: 260, weight: 6, color: '#edf4ff' },
  { name: 'Crown Levi', zone: 3, rarity: 'mythic', value: 1200, weight: 2, color: '#fff7c4' },
];

const state = {
  coins: 0,
  rod: 1,
  bait: 1,
  boat: 1,
  crew: 1,
  zone: 0,
  weather: 'Calm',
  mode: 'ready',
  timer: 0,
  biteAt: 0,
  biteWindow: 0,
  ripples: [],
  sparkles: [],
  log: ['Cast, hit the bite window, fill orders, and upgrade.'],
  autoTimer: 0,
  caught: null,
  collection: {},
  order: null,
};

function rodCost() { return 24 * state.rod * state.rod; }
function baitCost() { return 34 * state.bait * state.bait; }
function boatCost() { return 90 * state.boat * state.boat; }
function crewCost() { return 120 * state.crew * state.crew; }

function log(text) {
  state.log.unshift(text);
  state.log = state.log.slice(0, 4);
  document.getElementById('log').innerHTML = state.log.map(x => `<div>${x}</div>`).join('');
}

function newOrder() {
  const available = FISH.filter(f => f.zone <= state.zone && f.rarity !== 'mythic');
  const f = available[Math.floor(Math.random() * available.length)];
  state.order = { type: f.name, need: 2 + Math.floor(state.boat / 2), have: 0, reward: Math.floor(f.value * (4 + state.zone)) };
}

function weatherBonus() {
  if (state.weather === 'Lucky Tide') return 1.35;
  if (state.weather === 'Storm') return 1.15;
  return 1;
}

function rollWeather() {
  const r = Math.random();
  state.weather = r > 0.88 ? 'Lucky Tide' : r > 0.72 ? 'Storm' : 'Calm';
}

function weightedFish() {
  const luck = 1 + state.bait * 0.16 + state.zone * 0.12 + (state.weather === 'Lucky Tide' ? 0.35 : 0);
  const pool = FISH.filter(f => f.zone <= state.zone + (state.weather === 'Storm' ? 1 : 0) && f.zone <= ZONES.length - 1)
    .map(f => {
      const rarityRank = Object.keys(RARITY).indexOf(f.rarity);
      return { ...f, rollWeight: Math.max(1, f.weight / (1 + rarityRank * luck * 0.55)) };
    });
  const total = pool.reduce((s, f) => s + f.rollWeight, 0);
  let roll = Math.random() * total;
  for (const f of pool) {
    roll -= f.rollWeight;
    if (roll <= 0) return f;
  }
  return pool[0];
}

function catchValue(f, perfect = false) {
  return Math.floor(f.value * RARITY[f.rarity].mult * (1 + state.rod * 0.1) * (1 + state.boat * 0.08) * weatherBonus() * (perfect ? 1.55 : 1));
}

function cast() {
  if (state.mode === 'ready') {
    state.mode = 'waiting';
    state.timer = 0;
    state.biteAt = Math.max(45, 170 - state.rod * 15 - Math.random() * 55);
    state.caught = null;
    log(`${ZONES[state.zone].name}: line cast in ${state.weather.toLowerCase()}.`);
  } else if (state.mode === 'bite') {
    const f = weightedFish();
    const perfect = state.biteWindow > 34 && state.biteWindow < 76;
    const value = catchValue(f, perfect);
    state.coins += value;
    state.mode = 'ready';
    state.caught = f;
    state.collection[f.name] = (state.collection[f.name] || 0) + 1;
    if (state.order.type === f.name) state.order.have++;
    if (state.order.have >= state.order.need) {
      state.coins += state.order.reward;
      log(`Order complete: ${state.order.type}. +${state.order.reward}`);
      newOrder();
    }
    log(`${perfect ? 'Perfect catch' : 'Caught'} ${f.name} (${f.rarity}). +${value}`);
    splash(560, 360, f.color, f.rarity === 'mythic' ? 40 : 22);
    updateBook();
  } else if (state.mode === 'waiting') {
    state.mode = 'ready';
    log('Pulled too early. The line went quiet.');
  }
}

function splash(x, y, color, count) {
  for (let i = 0; i < count; i++) {
    state.ripples.push({ x: x + Math.random() * 80 - 40, y: y + Math.random() * 24 - 12, r: 2, life: 38, color });
    state.sparkles.push({ x, y, vx: (Math.random() - 0.5) * 4.2, vy: -Math.random() * 4 - 1, life: 32, color });
  }
}

function crewCatch() {
  const f = weightedFish();
  const value = Math.floor(catchValue(f, false) * (0.22 + state.crew * 0.08));
  state.coins += value;
  state.collection[f.name] = (state.collection[f.name] || 0) + 1;
  if (state.order.type === f.name) state.order.have++;
  splash(260 + Math.random() * 160, 390, f.color, 8);
  log(`Crew hauled ${f.name}. +${value}`);
  if (state.order.have >= state.order.need) {
    state.coins += state.order.reward;
    log(`Order complete: ${state.order.type}. +${state.order.reward}`);
    newOrder();
  }
  updateBook();
}

function update() {
  if (state.mode === 'waiting') {
    state.timer++;
    if (state.timer >= state.biteAt) {
      state.mode = 'bite';
      state.biteWindow = 100;
      log('BITE! Reel while the marker is centered.');
    }
  } else if (state.mode === 'bite') {
    state.biteWindow -= 1.15 + Math.max(0, state.zone - 1) * 0.12;
    if (state.biteWindow <= 0) {
      state.mode = 'ready';
      log('Too late. The fish shook free.');
    }
  }

  state.autoTimer++;
  if (state.autoTimer > Math.max(110, 430 - state.boat * 28 - state.crew * 22)) {
    state.autoTimer = 0;
    if (state.crew > 1 || state.boat > 1) crewCatch();
    if (Math.random() < 0.18) rollWeather();
  }

  for (const r of state.ripples) {
    r.r += 0.45;
    r.life--;
  }
  state.ripples = state.ripples.filter(r => r.life > 0);
  for (const p of state.sparkles) {
    p.x += p.vx;
    p.y += p.vy;
    p.vy += 0.12;
    p.life--;
  }
  state.sparkles = state.sparkles.filter(p => p.life > 0);
  updateHud();
}

function drawFish(x, y, f, scale = 1) {
  ctx.fillStyle = f.color;
  ctx.fillRect(x, y, 42 * scale, 18 * scale);
  ctx.fillRect(x + 8 * scale, y - 6 * scale, 22 * scale, 6 * scale);
  ctx.fillRect(x - 10 * scale, y + 5 * scale, 10 * scale, 8 * scale);
  ctx.fillStyle = '#071018';
  ctx.fillRect(x + 31 * scale, y + 5 * scale, 4 * scale, 4 * scale);
  ctx.fillStyle = RARITY[f.rarity].color;
  ctx.fillRect(x + 10 * scale, y + 18 * scale, 18 * scale, 4 * scale);
}

function draw() {
  const t = performance.now() / 1000;
  const zone = ZONES[state.zone];
  const sky = ctx.createLinearGradient(0, 0, 0, H);
  sky.addColorStop(0, zone.sky[0]);
  sky.addColorStop(0.52, zone.sky[1]);
  sky.addColorStop(0.53, zone.sea[0]);
  sky.addColorStop(1, zone.sea[1]);
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, W, H);

  ctx.fillStyle = state.weather === 'Storm' ? '#d8e6ff' : '#f4c85a';
  ctx.fillRect(690, 70 + Math.sin(t) * 5, 48, 48);
  if (state.weather === 'Lucky Tide') {
    ctx.fillStyle = 'rgba(255, 241, 166, 0.25)';
    for (let i = 0; i < 16; i++) ctx.fillRect((i * 67 + t * 30) % W, 120 + (i % 5) * 52, 6, 6);
  }

  ctx.fillStyle = 'rgba(255,255,255,0.16)';
  for (let x = 0; x < W; x += 42) {
    const y = 360 + Math.sin(t * 2 + x * 0.03) * 6;
    ctx.fillRect(x, y, 36, 7);
  }
  if (state.zone === 1) {
    ctx.fillStyle = '#185b45';
    for (let x = 30; x < W; x += 70) ctx.fillRect(x, 456 + Math.sin(t + x) * 10, 12, 88);
  }
  if (state.zone >= 2) {
    ctx.fillStyle = '#edf4ff';
    for (let i = 0; i < 22; i++) ctx.fillRect((i * 39) % W, 30 + (i * 29) % 210, 3, 3);
  }

  ctx.fillStyle = '#5a3f2c';
  ctx.fillRect(326, 276, 196, 40);
  ctx.fillStyle = '#7b5738';
  ctx.fillRect(356, 244, 122, 36);
  ctx.fillStyle = '#f0bf8f';
  ctx.fillRect(404, 212, 28, 26);
  ctx.fillStyle = '#263243';
  ctx.fillRect(394, 238, 48, 42);
  ctx.fillStyle = '#65d9ff';
  for (let i = 0; i < Math.min(4, state.crew); i++) ctx.fillRect(350 + i * 30, 232, 14, 18);
  ctx.strokeStyle = '#d7b46a';
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.moveTo(430, 230);
  ctx.lineTo(595, 286);
  ctx.stroke();

  if (state.mode !== 'ready') {
    const bob = 365 + Math.sin(t * 7) * 8;
    ctx.strokeStyle = state.mode === 'bite' ? '#f4c85a' : '#edf4ff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(595, 286);
    ctx.lineTo(570, bob);
    ctx.stroke();
    ctx.fillStyle = state.mode === 'bite' ? '#f4c85a' : '#edf4ff';
    ctx.fillRect(566, bob, 8, 8);
  }

  for (const r of state.ripples) {
    ctx.globalAlpha = Math.max(0, r.life / 38);
    ctx.strokeStyle = r.color;
    ctx.lineWidth = 2;
    ctx.strokeRect(r.x - r.r, r.y - r.r / 2, r.r * 2, r.r);
    ctx.globalAlpha = 1;
  }
  for (const p of state.sparkles) {
    ctx.globalAlpha = Math.max(0, p.life / 32);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x, p.y, 5, 5);
    ctx.globalAlpha = 1;
  }
  if (state.caught) drawFish(390, 166, state.caught, 1.25);

  if (state.mode === 'bite') {
    ctx.fillStyle = 'rgba(5, 9, 15, 0.68)';
    ctx.fillRect(300, 94, 300, 30);
    ctx.fillStyle = '#263243';
    ctx.fillRect(314, 104, 272, 10);
    ctx.fillStyle = '#72df89';
    ctx.fillRect(410, 101, 80, 16);
    ctx.fillStyle = '#f4c85a';
    ctx.fillRect(314 + state.biteWindow * 2.72, 96, 8, 26);
  }
}

function updateHud() {
  document.getElementById('coins').textContent = Math.floor(state.coins);
  document.getElementById('rod').textContent = state.rod;
  document.getElementById('bait').textContent = state.bait;
  document.getElementById('boat').textContent = state.boat;
  document.getElementById('zone').textContent = ZONES[state.zone].name;
  document.getElementById('order').textContent = `${state.order.have}/${state.order.need} ${state.order.type}`;
  document.getElementById('cast').textContent = state.mode === 'ready' ? 'Cast Line' : state.mode === 'bite' ? 'Reel In!' : 'Pull Back';
  document.getElementById('rod-up').textContent = `Rod ${rodCost()}`;
  document.getElementById('bait-up').textContent = `Bait ${baitCost()}`;
  document.getElementById('boat-up').textContent = `Boat ${boatCost()}`;
  document.getElementById('crew-up').textContent = `Crew ${crewCost()}`;
  document.getElementById('zone-next').textContent = `Sail ${state.zone + 1 < ZONES.length ? ZONES[state.zone + 1].name : ZONES[0].name}`;
  document.getElementById('rod-up').disabled = state.coins < rodCost();
  document.getElementById('bait-up').disabled = state.coins < baitCost();
  document.getElementById('boat-up').disabled = state.coins < boatCost();
  document.getElementById('crew-up').disabled = state.coins < crewCost();
}

function updateBook() {
  const seen = FISH.filter(f => state.collection[f.name]);
  document.getElementById('book').innerHTML = seen.slice(-5).map(f => (
    `<span style="border-color:${RARITY[f.rarity].color};color:${f.color}">${f.name} x${state.collection[f.name]}</span>`
  )).join('');
}

function buy(costFn, key, text) {
  const cost = costFn();
  if (state.coins < cost) return;
  state.coins -= cost;
  state[key]++;
  log(text);
}

document.getElementById('cast').onclick = cast;
document.getElementById('rod-up').onclick = () => buy(rodCost, 'rod', 'Rod upgraded: faster bites and higher value.');
document.getElementById('bait-up').onclick = () => buy(baitCost, 'bait', 'Bait upgraded: rarer fish now surface more often.');
document.getElementById('boat-up').onclick = () => buy(boatCost, 'boat', 'Boat upgraded: new waters and stronger idle income.');
document.getElementById('crew-up').onclick = () => buy(crewCost, 'crew', 'Crew upgraded: idle hauls improved.');
document.getElementById('zone-next').onclick = () => {
  const next = (state.zone + 1) % ZONES.length;
  if (state.boat < ZONES[next].unlock) {
    log(`${ZONES[next].name} requires boat ${ZONES[next].unlock}.`);
    return;
  }
  state.zone = next;
  rollWeather();
  newOrder();
  log(`Sailed to ${ZONES[state.zone].name}. Weather: ${state.weather}.`);
};
document.addEventListener('keydown', e => {
  if (e.key === ' ' || e.key === 'Enter') {
    e.preventDefault();
    cast();
  }
});

function loop() {
  update();
  draw();
  requestAnimationFrame(loop);
}

newOrder();
rollWeather();
log('Cast, wait for the bite marker, then reel in.');
updateBook();
loop();
