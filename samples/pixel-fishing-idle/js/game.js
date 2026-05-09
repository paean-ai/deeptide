const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const W = 900;
const H = 620;

const fish = [
  { name: 'Minnow', value: 4, color: '#65d9ff', weight: 48 },
  { name: 'Carp', value: 12, color: '#68da86', weight: 30 },
  { name: 'Koi', value: 32, color: '#f4c85a', weight: 15 },
  { name: 'Moonfin', value: 90, color: '#aa7dff', weight: 6 },
  { name: 'Ancient Ray', value: 240, color: '#ffffff', weight: 1 },
];

const state = {
  coins: 0,
  rod: 1,
  bait: 1,
  boat: 1,
  mode: 'ready',
  timer: 0,
  biteAt: 0,
  ripples: [],
  log: ['Cast, wait for a bite, then reel in.'],
  autoTimer: 0,
  caught: null,
};

function rodCost() { return 25 * state.rod * state.rod; }
function baitCost() { return 35 * state.bait * state.bait; }
function boatCost() { return 80 * state.boat * state.boat; }

function log(text) {
  state.log.unshift(text);
  state.log = state.log.slice(0, 3);
  document.getElementById('log').innerHTML = state.log.map(x => `<div>${x}</div>`).join('');
}

function weightedFish() {
  const luck = state.bait * 2;
  const pool = fish.map((f, i) => ({ ...f, weight: Math.max(1, f.weight + (i - 1) * luck) }));
  const total = pool.reduce((s, f) => s + f.weight, 0);
  let roll = Math.random() * total;
  for (const f of pool) {
    roll -= f.weight;
    if (roll <= 0) return f;
  }
  return pool[0];
}

function cast() {
  if (state.mode === 'ready') {
    state.mode = 'waiting';
    state.timer = 0;
    state.biteAt = Math.max(55, 180 - state.rod * 16 - Math.random() * 60);
    state.caught = null;
    log('Line cast...');
  } else if (state.mode === 'bite') {
    const f = weightedFish();
    const value = Math.floor(f.value * (1 + state.rod * 0.12) * (1 + state.boat * 0.08));
    state.coins += value;
    state.mode = 'ready';
    state.caught = f;
    log(`Caught ${f.name}! +${value} coins`);
    for (let i = 0; i < 14; i++) state.ripples.push({ x: 510 + Math.random() * 80, y: 335 + Math.random() * 45, r: 2, life: 40, color: f.color });
  } else if (state.mode === 'waiting') {
    state.mode = 'ready';
    log('Pulled too early. The fish escaped.');
  }
}

function update() {
  if (state.mode === 'waiting') {
    state.timer++;
    if (state.timer >= state.biteAt) {
      state.mode = 'bite';
      state.timer = 95;
      log('BITE! Reel in now!');
    }
  } else if (state.mode === 'bite') {
    state.timer--;
    if (state.timer <= 0) {
      state.mode = 'ready';
      log('Too late. The fish got away.');
    }
  }

  state.autoTimer++;
  if (state.boat > 1 && state.autoTimer > Math.max(150, 420 - state.boat * 35)) {
    state.autoTimer = 0;
    const f = fish[Math.min(fish.length - 1, Math.floor(Math.random() * state.boat))];
    const value = Math.floor(f.value * 0.45 * state.boat);
    state.coins += value;
    log(`Crew caught ${f.name}. +${value}`);
  }

  for (const r of state.ripples) {
    r.r += 0.45;
    r.life--;
  }
  state.ripples = state.ripples.filter(r => r.life > 0);
  updateHud();
}

function draw() {
  const t = performance.now() / 1000;
  const sky = ctx.createLinearGradient(0, 0, 0, H);
  sky.addColorStop(0, '#172946');
  sky.addColorStop(0.52, '#25476b');
  sky.addColorStop(0.53, '#123f62');
  sky.addColorStop(1, '#08243a');
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, W, H);

  ctx.fillStyle = '#f4c85a';
  ctx.fillRect(690, 70, 48, 48);
  ctx.fillStyle = '#315076';
  for (let x = 0; x < W; x += 42) {
    const y = 360 + Math.sin(t * 2 + x * 0.03) * 6;
    ctx.fillRect(x, y, 36, 7);
  }

  ctx.fillStyle = '#5a3f2c';
  ctx.fillRect(330, 270, 180, 38);
  ctx.fillStyle = '#7b5738';
  ctx.fillRect(356, 245, 116, 32);
  ctx.fillStyle = '#f0bf8f';
  ctx.fillRect(404, 214, 28, 26);
  ctx.fillStyle = '#263243';
  ctx.fillRect(394, 238, 48, 40);
  ctx.strokeStyle = '#d7b46a';
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.moveTo(430, 230);
  ctx.lineTo(585, 286);
  ctx.stroke();

  if (state.mode !== 'ready') {
    ctx.strokeStyle = state.mode === 'bite' ? '#f4c85a' : '#edf4ff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(585, 286);
    ctx.lineTo(570, 365 + Math.sin(t * 7) * 8);
    ctx.stroke();
    ctx.fillStyle = state.mode === 'bite' ? '#f4c85a' : '#edf4ff';
    ctx.fillRect(566, 365 + Math.sin(t * 7) * 8, 8, 8);
  }

  for (const r of state.ripples) {
    ctx.globalAlpha = Math.max(0, r.life / 40);
    ctx.strokeStyle = r.color;
    ctx.lineWidth = 2;
    ctx.strokeRect(r.x - r.r, r.y - r.r / 2, r.r * 2, r.r);
    ctx.globalAlpha = 1;
  }

  if (state.caught) {
    ctx.fillStyle = state.caught.color;
    ctx.fillRect(392, 168, 68, 28);
    ctx.fillStyle = '#101722';
    ctx.fillRect(442, 176, 6, 6);
  }
}

function updateHud() {
  document.getElementById('coins').textContent = Math.floor(state.coins);
  document.getElementById('rod').textContent = state.rod;
  document.getElementById('bait').textContent = state.bait;
  document.getElementById('boat').textContent = state.boat;
  document.getElementById('cast').textContent = state.mode === 'ready' ? 'Cast Line' : state.mode === 'bite' ? 'Reel In!' : 'Pull Back';
  document.getElementById('rod-up').textContent = `Rod (${rodCost()})`;
  document.getElementById('bait-up').textContent = `Bait (${baitCost()})`;
  document.getElementById('boat-up').textContent = `Boat (${boatCost()})`;
  document.getElementById('rod-up').disabled = state.coins < rodCost();
  document.getElementById('bait-up').disabled = state.coins < baitCost();
  document.getElementById('boat-up').disabled = state.coins < boatCost();
}

document.getElementById('cast').onclick = cast;
document.getElementById('rod-up').onclick = () => {
  if (state.coins < rodCost()) return;
  state.coins -= rodCost();
  state.rod++;
  log('Rod upgraded.');
};
document.getElementById('bait-up').onclick = () => {
  if (state.coins < baitCost()) return;
  state.coins -= baitCost();
  state.bait++;
  log('Better bait unlocked.');
};
document.getElementById('boat-up').onclick = () => {
  if (state.coins < boatCost()) return;
  state.coins -= boatCost();
  state.boat++;
  log('Boat crew improved.');
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

log('Cast, wait for a bite, then reel in.');
loop();
