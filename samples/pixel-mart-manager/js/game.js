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
  frame: 0,
};

const player = { x: 180, y: 320, speed: 3.2, facing: 1 };
const zones = {
  field: { x: 80, y: 125, w: 160, h: 130, label: 'BANANA GROVE' },
  shelf: { x: 430, y: 175, w: 125, h: 105, label: 'SHELF' },
  register: { x: 705, y: 310, w: 110, h: 75, label: 'CHECKOUT' },
};

const PAL = {
  ink: '#080a0f',
  edge: '#141922',
  shadow: '#050608',
  wall: '#1f2b3a',
  wallDark: '#111924',
  wallLight: '#34465c',
  floor: '#263447',
  floorAlt: '#2d3c51',
  floorDark: '#1a2533',
  cream: '#f2e6bd',
  gold: '#f2c14e',
  goldDark: '#a66a25',
  green: '#43d17a',
  greenDark: '#1f7f46',
  greenLight: '#8df09f',
  blue: '#2f80ed',
  blueLight: '#a9e8ff',
  red: '#e05243',
  redDark: '#8d2630',
  violet: '#b66cff',
  violetDark: '#5d328f',
  wood: '#8b5a32',
  woodDark: '#4e2e1d',
  skin: '#e9b57f',
  skinDark: '#a96f4a',
  white: '#f3f7ff',
  metal: '#8b93a1',
};

const SPRITE_MAP = {
  k: PAL.ink,
  e: PAL.edge,
  w: PAL.white,
  y: PAL.gold,
  Y: PAL.goldDark,
  g: PAL.green,
  G: PAL.greenDark,
  l: PAL.greenLight,
  b: PAL.blue,
  B: PAL.blueLight,
  r: PAL.red,
  R: PAL.redDark,
  v: PAL.violet,
  V: PAL.violetDark,
  n: PAL.wood,
  N: PAL.woodDark,
  s: PAL.skin,
  S: PAL.skinDark,
  m: PAL.metal,
  c: PAL.cream,
};

const SPRITES = {
  player: [
    '...kkkk...',
    '..kssssk..',
    '..kswwsk..',
    '.kyyyyyk.',
    'kyggggyk',
    'kyggggyk',
    '.kyNNyk.',
    '..kn.nk..',
    '.kn...nk.',
  ],
  helper: [
    '...kkkk...',
    '..kssssk..',
    '..kswwsk..',
    '.kgggggk.',
    'kgbbbbgk',
    'kggbbggk',
    '.kgNNgk.',
    '..kn.nk..',
    '.kn...nk.',
  ],
  customerA: [
    '...kkkk...',
    '..kssssk..',
    '..kswwsk..',
    '.kvvvvvk.',
    'kvyyyyvk',
    'kvyyyyvk',
    '.kvNNvk.',
    '..kn.nk..',
    '.kn...nk.',
  ],
  customerB: [
    '...kkkk...',
    '..kssssk..',
    '..kswwsk..',
    '.kbbbbbk.',
    'kbccccbk',
    'kbccccbk',
    '.kbNNbk.',
    '..kn.nk..',
    '.kn...nk.',
  ],
  banana: [
    '..yy.',
    '.yYy.',
    '.yy..',
    'yY...',
  ],
  crate: [
    'NNNNNN',
    'NnYYnN',
    'NYyyYN',
    'NnYYnN',
    'NNNNNN',
  ],
  register: [
    'mmmmmmmm',
    'mBBBBBBm',
    'mBkkkkBm',
    'mmmmmmmm',
    'NNNNNNNN',
    'NyyyyyyN',
  ],
};

function hash2(x, y, seed = 0) {
  const n = Math.sin(x * 12.9898 + y * 78.233 + seed * 41.17) * 43758.5453;
  return n - Math.floor(n);
}

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
  state.frame++;
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

function rect(x, y, w, h, color) {
  ctx.fillStyle = color;
  ctx.fillRect(Math.round(x), Math.round(y), Math.round(w), Math.round(h));
}

function pixelText(text, x, y, color = PAL.white, align = 'left', size = 12) {
  ctx.font = `bold ${size}px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`;
  ctx.textAlign = align;
  ctx.fillStyle = PAL.shadow;
  ctx.fillText(text, x + 1, y + 1);
  ctx.fillStyle = color;
  ctx.fillText(text, x, y);
  ctx.textAlign = 'left';
}

function drawPixelSprite(sprite, x, y, scale = 3, flip = 1, alpha = 1) {
  ctx.save();
  ctx.globalAlpha = alpha;
  const rows = sprite.length;
  const cols = sprite[0].length;
  const ox = Math.round(x - cols * scale / 2);
  const oy = Math.round(y - rows * scale);
  ctx.translate(ox + (flip < 0 ? cols * scale : 0), oy);
  ctx.scale(flip, 1);
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const ch = sprite[row][col];
      if (ch !== '.' && SPRITE_MAP[ch]) {
        ctx.fillStyle = SPRITE_MAP[ch];
        ctx.fillRect(col * scale, row * scale, scale, scale);
      }
    }
  }
  ctx.restore();
}

function drawShadow(x, y, w = 34, h = 10, alpha = 0.32) {
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.fillStyle = PAL.shadow;
  for (let px = -w; px <= w; px += 3) {
    for (let py = -h; py <= h; py += 3) {
      if ((px * px) / (w * w) + (py * py) / (h * h) <= 1) {
        ctx.fillRect(Math.round(x + px), Math.round(y + py), 3, 3);
      }
    }
  }
  ctx.restore();
}

function drawPanel(x, y, w, h, fill, hi, edge = PAL.ink) {
  rect(x + 4, y + 4, w, h, '#00000030');
  rect(x, y, w, h, edge);
  rect(x + 3, y + 3, w - 6, h - 6, fill);
  rect(x + 3, y + 3, w - 6, 5, hi);
  rect(x + 3, y + h - 8, w - 6, 5, '#00000022');
}

function drawTileFloor() {
  drawPanel(42, 96, 876, 382, PAL.floor, PAL.floorAlt, '#0b1119');
  for (let x = 54; x < 900; x += 32) {
    for (let y = 112; y < 462; y += 24) {
      const alt = (Math.floor(x / 32) + Math.floor(y / 24)) % 2 === 0;
      rect(x, y, 30, 22, alt ? PAL.floor : PAL.floorDark);
      if (hash2(x, y, 3) > 0.78) rect(x + 5, y + 4, 5, 3, '#3a4d63');
      if (hash2(x, y, 8) > 0.86) rect(x + 18, y + 14, 4, 3, '#172131');
    }
  }
  rect(42, 96, 876, 12, PAL.wallLight);
  rect(42, 466, 876, 12, '#0c121a');
  rect(42, 96, 12, 382, '#0c121a');
  rect(906, 96, 12, 382, PAL.wallDark);
}

function drawStorefront() {
  rect(0, 0, W, H, '#0a0d14');
  for (let i = 0; i < 70; i++) {
    const x = (i * 137) % W;
    const y = 12 + (i * 43) % 78;
    const c = i % 5 === 0 ? PAL.gold : '#5d6d83';
    rect(x, y, i % 3 === 0 ? 2 : 1, 1, c);
  }
  rect(40, 40, 880, 72, PAL.wallDark);
  rect(54, 50, 852, 52, PAL.wall);
  for (let x = 62; x < 900; x += 52) {
    rect(x, 54, 36, 44, '#233249');
    rect(x, 54, 36, 5, '#425875');
  }
  drawPanel(354, 34, 252, 58, '#14251c', PAL.greenDark, '#071009');
  pixelText('PIXEL MART', 480, 69, PAL.greenLight, 'center', 26);
  rect(392 + Math.sin(state.frame * 0.06) * 2, 77, 176, 3, '#d5ffb8');
  for (let x = 46; x < 914; x += 48) {
    rect(x, 104, 24, 16, PAL.red);
    rect(x + 24, 104, 24, 16, PAL.cream);
    rect(x, 120, 48, 5, PAL.ink);
  }
}

function drawBananaGrove() {
  const z = zones.field;
  drawPanel(z.x, z.y, z.w, z.h, '#223f2a', PAL.greenDark, '#0c1b10');
  for (let i = 0; i < 20; i++) {
    const x = z.x + 14 + (i % 5) * 29;
    const y = z.y + 24 + Math.floor(i / 5) * 24;
    rect(x, y + 10, 4, 16, PAL.greenDark);
    rect(x - 6, y + 4, 16, 8, PAL.green);
    rect(x + 4, y, 16, 8, PAL.greenLight);
    drawPixelSprite(SPRITES.banana, x + 10, y + 21, 3, 1);
  }
  for (let i = 0; i < Math.min(state.stock, 8); i++) {
    drawPixelSprite(SPRITES.crate, z.x + 26 + i * 16, z.y + 124, 2, 1);
  }
}

function drawShelf() {
  const z = zones.shelf;
  drawShadow(z.x + z.w / 2, z.y + z.h + 7, 74, 13, 0.28);
  drawPanel(z.x, z.y, z.w, z.h, PAL.wood, '#b06d36', PAL.woodDark);
  for (let r = 0; r < 3; r++) {
    rect(z.x + 9, z.y + 22 + r * 27, z.w - 18, 7, PAL.woodDark);
    rect(z.x + 12, z.y + 20 + r * 27, z.w - 24, 3, '#c8864b');
  }
  const cap = Math.min(state.shelf, 15);
  for (let i = 0; i < cap; i++) {
    const x = z.x + 18 + (i % 5) * 20;
    const y = z.y + 19 + Math.floor(i / 5) * 27;
    const colors = [PAL.gold, PAL.green, PAL.blueLight, PAL.red, PAL.violet];
    rect(x - 1, y + 2, 14, 17, PAL.ink);
    rect(x, y, 12, 17, colors[i % colors.length]);
    rect(x + 2, y + 3, 8, 3, PAL.white);
    rect(x + 2, y + 10, 8, 5, '#00000024');
  }
  if (state.shelf > cap) pixelText(`+${state.shelf - cap}`, z.x + z.w - 18, z.y + z.h - 10, PAL.gold, 'center', 10);
}

function drawRegister() {
  const z = zones.register;
  drawShadow(z.x + z.w / 2, z.y + z.h + 6, 66, 12, 0.26);
  drawPanel(z.x, z.y, z.w, z.h, '#24476b', PAL.blue, '#102034');
  drawPixelSprite(SPRITES.register, z.x + 54, z.y + 62, 5, 1);
  rect(z.x + 14, z.y + 12, 24, 18, '#101722');
  rect(z.x + 18, z.y + 16, 16, 8, PAL.greenLight);
  rect(z.x + 72, z.y + 18, 22, 8, PAL.gold);
  rect(z.x + 76, z.y + 30, 15, 4, PAL.ink);
}

function drawAisles() {
  for (let i = 0; i < 3; i++) {
    const x = 300 + i * 98;
    drawShadow(x + 28, 380, 48, 9, 0.2);
    drawPanel(x, 318, 58, 58, i === 1 ? '#314b5c' : '#3f3429', i === 1 ? PAL.blue : '#bd8045', PAL.ink);
    for (let j = 0; j < 6; j++) {
      const px = x + 10 + (j % 3) * 14;
      const py = 333 + Math.floor(j / 3) * 18;
      rect(px, py, 10, 12, [PAL.red, PAL.gold, PAL.green, PAL.blueLight][(i + j) % 4]);
      rect(px + 2, py + 2, 6, 3, PAL.white);
    }
  }
}

function drawPerson(x, y, sprite, flip = 1, bob = 0) {
  drawShadow(x, y + 17, 22, 6, 0.34);
  drawPixelSprite(sprite, x, y + bob, 4, flip);
}

function draw() {
  drawStorefront();
  drawTileFloor();
  drawAisles();
  drawBananaGrove();
  drawShelf();
  drawRegister();

  pixelText(zones.field.label, zones.field.x, zones.field.y - 9, PAL.greenLight);
  pixelText(zones.shelf.label, zones.shelf.x, zones.shelf.y - 9, PAL.gold);
  pixelText(zones.register.label, zones.register.x, zones.register.y - 9, PAL.blueLight);

  for (const c of state.customers) {
    const sprite = c.phase === 'paying' ? SPRITES.customerB : SPRITES.customerA;
    drawPerson(c.x, c.y, sprite, -1, Math.sin(c.bob) * 2);
  }
  if (state.helperLevel > 0) {
    const bob = Math.sin(state.frame * 0.08) * 1.5;
    drawPerson(350, 255, SPRITES.helper, 1, bob);
    pixelText(`LV ${state.helperLevel}`, 350, 202, PAL.greenLight, 'center', 10);
  }
  drawPerson(player.x, player.y, SPRITES.player, player.facing, (state.keys.ArrowLeft || state.keys.ArrowRight || state.keys.ArrowUp || state.keys.ArrowDown) ? Math.sin(state.frame * 0.28) * 1.5 : 0);

  for (const f of state.floaters) {
    ctx.globalAlpha = Math.max(0, f.life / 70);
    pixelText(f.text, f.x, f.y, f.color, 'center', 14);
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
