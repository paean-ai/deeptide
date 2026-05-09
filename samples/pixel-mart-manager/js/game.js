const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const W = 960;
const H = 540;

const state = {
  coins: 0,
  rep: 1,
  shelfLevel: 1,
  helperLevel: 0,
  expansion: 1,
  stock: 0,
  maxStock: 6,
  shelf: 0,
  customers: [],
  floaters: [],
  keys: {},
  touch: {},
  customerTimer: 0,
  helperTimer: 0,
};

const player = { x: 180, y: 320, speed: 3.2, facing: 1 };
const zones = {
  field: { x: 80, y: 125, w: 160, h: 130, label: 'BANANA GROVE' },
  shelf: { x: 430, y: 175, w: 125, h: 105, label: 'SHELF' },
  register: { x: 705, y: 310, w: 110, h: 75, label: 'CHECKOUT' },
};

function rectHit(a, b, pad = 0) {
  return a.x > b.x - pad && a.x < b.x + b.w + pad && a.y > b.y - pad && a.y < b.y + b.h + pad;
}

function addFloater(text, x, y, color = '#f2c14e') {
  state.floaters.push({ text, x, y, color, life: 70 });
}

function action() {
  if (rectHit(player, zones.field, 22) && state.stock < state.maxStock) {
    const gain = Math.min(state.maxStock - state.stock, 1 + Math.floor(state.expansion / 2));
    state.stock += gain;
    addFloater(`+${gain} stock`, player.x, player.y - 24, '#60d882');
  } else if (rectHit(player, zones.shelf, 28) && state.stock > 0) {
    const cap = 5 + state.shelfLevel * 3;
    const moved = Math.min(state.stock, cap - state.shelf);
    if (moved > 0) {
      state.stock -= moved;
      state.shelf += moved;
      addFloater(`shelf +${moved}`, zones.shelf.x + 60, zones.shelf.y - 10, '#64c7ff');
    }
  } else if (rectHit(player, zones.register, 32)) {
    checkout();
  }
}

function checkout() {
  const ready = state.customers.filter(c => c.phase === 'paying');
  if (!ready.length) return;
  const c = ready[0];
  const value = 5 + state.shelfLevel * 2 + state.rep;
  state.coins += value;
  state.rep += 0.04;
  c.phase = 'done';
  addFloater(`+${value} coins`, zones.register.x + 44, zones.register.y - 12);
}

function spawnCustomer() {
  state.customers.push({ x: 980, y: 352, phase: 'enter', patience: 650, bob: Math.random() * 10 });
}

function update() {
  const left = state.keys.ArrowLeft || state.keys.a || state.touch.left;
  const right = state.keys.ArrowRight || state.keys.d || state.touch.right;
  const up = state.keys.ArrowUp || state.keys.w || state.touch.up;
  const down = state.keys.ArrowDown || state.keys.s || state.touch.down;
  if (left) { player.x -= player.speed; player.facing = -1; }
  if (right) { player.x += player.speed; player.facing = 1; }
  if (up) player.y -= player.speed;
  if (down) player.y += player.speed;
  player.x = Math.max(35, Math.min(W - 35, player.x));
  player.y = Math.max(98, Math.min(H - 66, player.y));

  state.customerTimer--;
  if (state.customerTimer <= 0) {
    spawnCustomer();
    state.customerTimer = Math.max(95, 250 - state.rep * 18);
  }

  for (const c of state.customers) {
    c.bob += 0.1;
    if (c.phase === 'enter') {
      c.x -= 1.45;
      if (c.x < zones.shelf.x + 110) c.phase = 'shop';
    } else if (c.phase === 'shop') {
      c.patience--;
      if (state.shelf > 0) {
        state.shelf--;
        c.phase = 'paying';
        c.x = zones.register.x + 78 + Math.random() * 32;
      } else if (c.patience <= 0) {
        c.phase = 'done';
        addFloater('missed sale', c.x, c.y - 28, '#e75b58');
      }
    } else if (c.phase === 'done') {
      c.x += 2.2;
    }
  }
  state.customers = state.customers.filter(c => c.x < W + 90);

  if (state.helperLevel > 0) {
    state.helperTimer--;
    if (state.helperTimer <= 0) {
      const cap = 5 + state.shelfLevel * 3;
      if (state.shelf < cap) state.shelf = Math.min(cap, state.shelf + state.helperLevel);
      state.helperTimer = Math.max(45, 150 - state.helperLevel * 16);
    }
  }

  for (const f of state.floaters) {
    f.y -= 0.45;
    f.life--;
  }
  state.floaters = state.floaters.filter(f => f.life > 0);
  updateHud();
}

function drawRect(x, y, w, h, color, top = '#ffffff22') {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = top;
  ctx.fillRect(x, y, w, 5);
}

function drawPerson(x, y, color, flip = 1) {
  ctx.save();
  ctx.translate(Math.round(x), Math.round(y));
  ctx.scale(flip, 1);
  ctx.fillStyle = '#141923';
  ctx.fillRect(-11, -30, 22, 33);
  ctx.fillStyle = color;
  ctx.fillRect(-9, -20, 18, 18);
  ctx.fillStyle = '#f0bf8f';
  ctx.fillRect(-7, -34, 14, 12);
  ctx.fillStyle = '#10151d';
  ctx.fillRect(-5, -30, 3, 2);
  ctx.fillRect(3, -30, 3, 2);
  ctx.fillStyle = '#263242';
  ctx.fillRect(-8, 3, 6, 14);
  ctx.fillRect(2, 3, 6, 14);
  ctx.restore();
}

function draw() {
  ctx.fillStyle = '#17202d';
  ctx.fillRect(0, 0, W, H);
  ctx.fillStyle = '#203046';
  for (let x = 0; x < W; x += 48) {
    for (let y = 92; y < H; y += 48) ctx.fillRect(x, y, 24, 24);
  }
  drawRect(42, 96, 876, 382, '#26344a');
  drawRect(zones.field.x, zones.field.y, zones.field.w, zones.field.h, '#284b2e', '#4cca6c');
  drawRect(zones.shelf.x, zones.shelf.y, zones.shelf.w, zones.shelf.h, '#583d25', '#f2c14e');
  drawRect(zones.register.x, zones.register.y, zones.register.w, zones.register.h, '#2b4562', '#64c7ff');

  ctx.font = '12px monospace';
  ctx.fillStyle = '#edf4ff';
  for (const z of Object.values(zones)) ctx.fillText(z.label, z.x, z.y - 8);

  for (let i = 0; i < 12; i++) {
    const x = zones.field.x + 18 + (i % 4) * 34;
    const y = zones.field.y + 22 + Math.floor(i / 4) * 32;
    ctx.fillStyle = '#6ee58a';
    ctx.fillRect(x, y, 18, 18);
    ctx.fillStyle = '#f2c14e';
    ctx.fillRect(x + 5, y + 5, 8, 12);
  }
  ctx.fillStyle = '#f2c14e';
  for (let i = 0; i < state.shelf; i++) {
    ctx.fillRect(zones.shelf.x + 12 + (i % 5) * 20, zones.shelf.y + 18 + Math.floor(i / 5) * 24, 12, 16);
  }

  for (const c of state.customers) {
    drawPerson(c.x, c.y + Math.sin(c.bob) * 2, c.phase === 'paying' ? '#64c7ff' : '#aa7dff', -1);
  }
  if (state.helperLevel > 0) drawPerson(350, 255, '#60d882', 1);
  drawPerson(player.x, player.y, '#f2c14e', player.facing);

  for (const f of state.floaters) {
    ctx.globalAlpha = Math.max(0, f.life / 70);
    ctx.fillStyle = f.color;
    ctx.font = '14px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(f.text, f.x, f.y);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
}

function updateHud() {
  document.getElementById('coins').textContent = Math.floor(state.coins);
  document.getElementById('stock').textContent = `${state.stock}/${state.maxStock}`;
  document.getElementById('shelf').textContent = state.shelf;
  document.getElementById('rep').textContent = state.rep.toFixed(1);
  document.getElementById('upgrade-shelf').textContent = `Upgrade Shelf (${shelfCost()})`;
  document.getElementById('hire-helper').textContent = `Hire Helper (${helperCost()})`;
  document.getElementById('expand-mart').textContent = `Expand Mart (${expandCost()})`;
  document.getElementById('upgrade-shelf').disabled = state.coins < shelfCost();
  document.getElementById('hire-helper').disabled = state.coins < helperCost();
  document.getElementById('expand-mart').disabled = state.coins < expandCost();
}

function shelfCost() { return 40 + state.shelfLevel * 35; }
function helperCost() { return 90 + state.helperLevel * 120; }
function expandCost() { return 160 + state.expansion * 180; }

document.addEventListener('keydown', e => {
  state.keys[e.key] = true;
  if (e.key === ' ' || e.key === 'Enter') action();
});
document.addEventListener('keyup', e => { state.keys[e.key] = false; });
document.getElementById('action').onclick = action;
document.querySelectorAll('[data-dir]').forEach(btn => {
  const dir = btn.dataset.dir;
  const on = e => { e.preventDefault(); state.touch[dir] = true; };
  const off = e => { e.preventDefault(); state.touch[dir] = false; };
  btn.addEventListener('pointerdown', on);
  btn.addEventListener('pointerup', off);
  btn.addEventListener('pointercancel', off);
});
document.getElementById('upgrade-shelf').onclick = () => {
  if (state.coins < shelfCost()) return;
  state.coins -= shelfCost();
  state.shelfLevel++;
  addFloater('shelf upgraded', 520, 145, '#64c7ff');
};
document.getElementById('hire-helper').onclick = () => {
  if (state.coins < helperCost()) return;
  state.coins -= helperCost();
  state.helperLevel++;
  addFloater('helper hired', 350, 220, '#60d882');
};
document.getElementById('expand-mart').onclick = () => {
  if (state.coins < expandCost()) return;
  state.coins -= expandCost();
  state.expansion++;
  state.maxStock += 2;
  state.rep += 0.4;
  addFloater('mart expanded', 480, 105, '#f2c14e');
};

function loop() {
  update();
  draw();
  requestAnimationFrame(loop);
}

updateHud();
loop();
