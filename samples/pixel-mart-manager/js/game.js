// Pixel Mart Manager - delta-timed shop management with helpers, marketing,
// customer patience, localStorage save and offline helper income.

const SAVE_KEY = 'pixel-mart-manager-save';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = W;
canvas.height = H;
artInit(ctx);

const state = {
  coins: 0, rep: 1,
  shelfLevel: 1, helperLevel: 0, expansion: 1, marketingLevel: 0,
  stock: 0, maxStock: 6, shelf: 0,
  customers: [], floaters: [],
  keys: {}, touch: {},
  customerTimer: 2, helperTimer: 2, time: 0,
  lastSave: Date.now(),
};

const player = { x: 180, y: 320, speed: 192, facing: 1, moving: false };

// ---- costs -------------------------------------------------------------
function shelfCost() { return COSTS.shelf(state.shelfLevel); }
function helperCost() { return COSTS.helper(state.helperLevel); }
function expandCost() { return COSTS.expand(state.expansion); }
function marketingCost() { return COSTS.marketing(state.marketingLevel); }
function checkoutValue() { return 5 + state.shelfLevel * 2 + Math.floor(state.rep) + state.marketingLevel * 2; }
function spawnInterval() { return Math.max(1.5, 4.2 - state.rep * 0.28 - state.marketingLevel * 0.32); }

// ---- helpers -----------------------------------------------------------
function rectHit(a, b, pad = 0) {
  return a.x > b.x - pad && a.x < b.x + b.w + pad && a.y > b.y - pad && a.y < b.y + b.h + pad;
}
function addFloater(text, x, y, color = PAL.gold) {
  state.floaters.push({ text, x, y, color, life: 1.2 });
}

function action() {
  if (rectHit(player, ZONES.field, 22) && state.stock < state.maxStock) {
    const gain = Math.min(state.maxStock - state.stock, 1 + Math.floor(state.expansion / 2));
    state.stock += gain;
    addFloater(t('stockGain', gain), player.x, player.y - 24, PAL.greenLight);
  } else if (rectHit(player, ZONES.shelf, 28) && state.stock > 0) {
    const cap = shelfCap(state.shelfLevel);
    const moved = Math.min(state.stock, cap - state.shelf);
    if (moved > 0) {
      state.stock -= moved;
      state.shelf += moved;
      addFloater(t('shelfGain', moved), ZONES.shelf.x + 60, ZONES.shelf.y - 10, PAL.blueLight);
    }
  } else if (rectHit(player, ZONES.register, 32)) {
    checkout();
  }
}

function checkout() {
  const c = state.customers.find(c => c.phase === 'paying');
  if (!c) return;
  const value = checkoutValue();
  state.coins += value;
  state.rep += 0.04;
  c.phase = 'done';
  addFloater(t('coinsGain', value), ZONES.register.x + 44, ZONES.register.y - 12);
  save();
}

function spawnCustomer() {
  const sprites = ['customerA', 'customerB', 'customerC'];
  state.customers.push({
    x: 980, y: 352, phase: 'enter',
    patience: 11, maxPatience: 11, bob: Math.random() * 10,
    sprite: sprites[Math.floor(Math.random() * sprites.length)],
  });
}

// ---- update ------------------------------------------------------------
function update(dt) {
  state.time += dt;
  const k = state.keys, tc = state.touch;
  const left = k.ArrowLeft || k.a || k.A || tc.left;
  const right = k.ArrowRight || k.d || k.D || tc.right;
  const up = k.ArrowUp || k.w || k.W || tc.up;
  const down = k.ArrowDown || k.s || k.S || tc.down;
  const v = player.speed * dt;
  if (left) { player.x -= v; player.facing = -1; }
  if (right) { player.x += v; player.facing = 1; }
  if (up) player.y -= v;
  if (down) player.y += v;
  player.moving = !!(left || right || up || down);
  player.x = Math.max(35, Math.min(W - 35, player.x));
  player.y = Math.max(98, Math.min(H - 66, player.y));

  state.customerTimer -= dt;
  if (state.customerTimer <= 0) {
    spawnCustomer();
    state.customerTimer = spawnInterval();
  }

  for (const c of state.customers) {
    c.bob += 6 * dt;
    if (c.phase === 'enter') {
      c.x -= 90 * dt;
      if (c.x < ZONES.shelf.x + 110) c.phase = 'shop';
    } else if (c.phase === 'shop') {
      c.patience -= dt;
      if (state.shelf > 0) {
        state.shelf--;
        c.phase = 'paying';
        c.x = ZONES.register.x + 78 + Math.random() * 32;
      } else if (c.patience <= 0) {
        c.phase = 'done';
        state.rep = Math.max(1, state.rep - 0.05);
        addFloater(t('missed'), c.x, c.y - 28, PAL.red);
      }
    } else if (c.phase === 'done') {
      c.x += 132 * dt;
    }
  }
  state.customers = state.customers.filter(c => c.x < W + 90);

  if (state.helperLevel > 0) {
    state.helperTimer -= dt;
    if (state.helperTimer <= 0) {
      const cap = shelfCap(state.shelfLevel);
      if (state.shelf < cap) state.shelf = Math.min(cap, state.shelf + state.helperLevel);
      state.helperTimer = Math.max(0.75, 2.5 - state.helperLevel * 0.27);
    }
  }

  for (const f of state.floaters) { f.y -= 27 * dt; f.life -= dt; }
  state.floaters = state.floaters.filter(f => f.life > 0);
  updateHud();
}

// ---- scene drawing -----------------------------------------------------
function drawStorefront() {
  rect(0, 0, W, H, '#0a0d14');
  for (let i = 0; i < 70; i++) {
    const x = (i * 137) % W, y = 12 + (i * 43) % 78;
    rect(x, y, i % 3 === 0 ? 2 : 1, 1, i % 5 === 0 ? PAL.gold : '#5d6d83');
  }
  rect(40, 40, 880, 72, PAL.wallDark);
  rect(54, 50, 852, 52, PAL.wall);
  for (let x = 62; x < 900; x += 52) {
    rect(x, 54, 36, 44, '#233249');
    rect(x, 54, 36, 5, '#425875');
  }
  drawPanel(354, 34, 252, 58, '#14251c', PAL.greenDark, '#071009');
  pixelText(t('title'), 480, 69, PAL.greenLight, 'center', 26);
  rect(392 + Math.sin(state.time * 3.6) * 2, 77, 176, 3, '#d5ffb8');
  for (let x = 46; x < 914; x += 48) {
    rect(x, 104, 24, 16, PAL.red);
    rect(x + 24, 104, 24, 16, PAL.cream);
    rect(x, 120, 48, 5, PAL.ink);
  }
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

function drawAisles() {
  for (let i = 0; i < 3; i++) {
    const x = 300 + i * 98;
    drawShadow(x + 28, 380, 48, 9, 0.2);
    drawPanel(x, 318, 58, 58, i === 1 ? '#314b5c' : '#3f3429', i === 1 ? PAL.blue : '#bd8045', PAL.ink);
    for (let j = 0; j < 6; j++) {
      const px = x + 10 + (j % 3) * 14, py = 333 + Math.floor(j / 3) * 18;
      rect(px, py, 10, 12, [PAL.red, PAL.gold, PAL.green, PAL.blueLight][(i + j) % 4]);
      rect(px + 2, py + 2, 6, 3, PAL.white);
    }
  }
}

function drawBananaGrove() {
  const z = ZONES.field;
  drawPanel(z.x, z.y, z.w, z.h, '#223f2a', PAL.greenDark, '#0c1b10');
  for (let i = 0; i < 20; i++) {
    const x = z.x + 14 + (i % 5) * 29, y = z.y + 24 + Math.floor(i / 5) * 24;
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
  const z = ZONES.shelf;
  drawShadow(z.x + z.w / 2, z.y + z.h + 7, 74, 13, 0.28);
  drawPanel(z.x, z.y, z.w, z.h, PAL.wood, '#b06d36', PAL.woodDark);
  for (let r = 0; r < 3; r++) {
    rect(z.x + 9, z.y + 22 + r * 27, z.w - 18, 7, PAL.woodDark);
    rect(z.x + 12, z.y + 20 + r * 27, z.w - 24, 3, '#c8864b');
  }
  const cap = Math.min(state.shelf, 15);
  const colors = [PAL.gold, PAL.green, PAL.blueLight, PAL.red, PAL.violet];
  for (let i = 0; i < cap; i++) {
    const x = z.x + 18 + (i % 5) * 20, y = z.y + 19 + Math.floor(i / 5) * 27;
    rect(x - 1, y + 2, 14, 17, PAL.ink);
    rect(x, y, 12, 17, colors[i % colors.length]);
    rect(x + 2, y + 3, 8, 3, PAL.white);
    rect(x + 2, y + 10, 8, 5, '#00000024');
  }
  if (state.shelf > cap) pixelText(`+${state.shelf - cap}`, z.x + z.w - 18, z.y + z.h - 10, PAL.gold, 'center', 10);
}

function drawRegister() {
  const z = ZONES.register;
  drawShadow(z.x + z.w / 2, z.y + z.h + 6, 66, 12, 0.26);
  drawPanel(z.x, z.y, z.w, z.h, '#24476b', PAL.blue, '#102034');
  drawPixelSprite(SPRITES.register, z.x + 54, z.y + 62, 5, 1);
  rect(z.x + 14, z.y + 12, 24, 18, '#101722');
  rect(z.x + 18, z.y + 16, 16, 8, PAL.greenLight);
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

  pixelText(t(ZONES.field.labelKey), ZONES.field.x, ZONES.field.y - 9, PAL.greenLight);
  pixelText(t(ZONES.shelf.labelKey), ZONES.shelf.x, ZONES.shelf.y - 9, PAL.gold);
  pixelText(t(ZONES.register.labelKey), ZONES.register.x, ZONES.register.y - 9, PAL.blueLight);

  for (const c of state.customers) {
    drawPerson(c.x, c.y, SPRITES[c.sprite], -1, Math.sin(c.bob) * 2);
    if (c.phase === 'shop') {
      const f = c.patience / c.maxPatience;
      drawBar(c.x - 14, c.y - 42, 28, f, f > 0.4 ? PAL.greenLight : PAL.red);
    }
  }
  if (state.helperLevel > 0) {
    drawPerson(350, 255, SPRITES.helper, 1, Math.sin(state.time * 4.8) * 1.5);
    pixelText(`LV ${state.helperLevel}`, 350, 202, PAL.greenLight, 'center', 10);
  }
  drawPerson(player.x, player.y, SPRITES.player, player.facing,
    player.moving ? Math.sin(state.time * 17) * 1.5 : 0);

  for (const f of state.floaters) {
    setGlobalAlpha(Math.max(0, f.life / 1.2));
    pixelText(f.text, f.x, f.y, f.color, 'center', 14);
    setGlobalAlpha(1);
  }
}

// ---- hud ---------------------------------------------------------------
function updateHud() {
  document.getElementById('coins').textContent = Math.floor(state.coins);
  document.getElementById('stock').textContent = `${state.stock}/${state.maxStock}`;
  document.getElementById('shelf').textContent = state.shelf;
  document.getElementById('rep').textContent = state.rep.toFixed(1);
  const setBtn = (id, label, c) => {
    const el = document.getElementById(id);
    el.textContent = `${label} (${c})`;
    el.disabled = state.coins < c;
  };
  setBtn('upgrade-shelf', t('upgradeShelf'), shelfCost());
  setBtn('hire-helper', t('hireHelper'), helperCost());
  setBtn('expand-mart', t('expandMart'), expandCost());
  setBtn('upgrade-marketing', t('marketing'), marketingCost());
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
}

// ---- save / load / offline --------------------------------------------
function save() {
  state.lastSave = Date.now();
  try {
    localStorage.setItem(SAVE_KEY, JSON.stringify({
      coins: state.coins, rep: state.rep, shelfLevel: state.shelfLevel,
      helperLevel: state.helperLevel, expansion: state.expansion,
      marketingLevel: state.marketingLevel, maxStock: state.maxStock,
      stock: state.stock, shelf: state.shelf, lastSave: state.lastSave,
    }));
  } catch (e) { /* storage unavailable */ }
}

function load() {
  let d;
  try { d = JSON.parse(localStorage.getItem(SAVE_KEY)); } catch (e) { d = null; }
  if (!d) return;
  Object.assign(state, {
    coins: d.coins || 0, rep: d.rep || 1, shelfLevel: d.shelfLevel || 1,
    helperLevel: d.helperLevel || 0, expansion: d.expansion || 1,
    marketingLevel: d.marketingLevel || 0, maxStock: d.maxStock || 6,
    stock: d.stock || 0, shelf: d.shelf || 0,
  });
  // offline helper income: helpers keep restocking and selling while away
  const elapsed = Math.min(OFFLINE_CAP_SECONDS, Math.max(0, (Date.now() - (d.lastSave || Date.now())) / 1000));
  if (elapsed > 30 && state.helperLevel > 0) {
    const earned = Math.floor(elapsed * state.helperLevel * (1 + state.shelfLevel * 0.25) * 0.5);
    if (earned > 0) {
      state.coins += earned;
      setTimeout(() => addFloater(t('welcomeBack', earned), W / 2, 150, PAL.gold), 400);
    }
  }
}

// ---- input -------------------------------------------------------------
document.addEventListener('keydown', e => {
  state.keys[e.key] = true;
  if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); action(); }
});
document.addEventListener('keyup', e => { state.keys[e.key] = false; });
document.getElementById('action').onclick = action;
document.querySelectorAll('[data-dir]').forEach(btn => {
  const dir = btn.dataset.dir;
  const on = e => { e.preventDefault(); state.touch[dir] = true; };
  const off = e => { e.preventDefault(); state.touch[dir] = false; };
  btn.addEventListener('pointerdown', on);
  btn.addEventListener('pointerup', off);
  btn.addEventListener('pointerleave', off);
  btn.addEventListener('pointercancel', off);
});

function buy(costFn, apply) {
  const c = costFn();
  if (state.coins < c) return;
  state.coins -= c;
  apply();
  save();
}
document.getElementById('upgrade-shelf').onclick = () => buy(shelfCost, () => {
  state.shelfLevel++;
  addFloater(t('shelfUp'), 520, 145, PAL.blueLight);
});
document.getElementById('hire-helper').onclick = () => buy(helperCost, () => {
  state.helperLevel++;
  addFloater(t('helperUp'), 350, 220, PAL.greenLight);
});
document.getElementById('expand-mart').onclick = () => buy(expandCost, () => {
  state.expansion++;
  state.maxStock += 2;
  state.rep += 0.4;
  addFloater(t('martUp'), 480, 105, PAL.gold);
});
document.getElementById('upgrade-marketing').onclick = () => buy(marketingCost, () => {
  state.marketingLevel++;
  state.rep += 0.2;
  addFloater(t('marketUp'), 700, 150, PAL.violet);
});
setupLanguageToggle(() => updateHud());

// ---- loop --------------------------------------------------------------
let lastT = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  update(dt);
  draw();
  requestAnimationFrame(loop);
}

setInterval(save, 5000);
addEventListener('beforeunload', save);

load();
updateHud();
requestAnimationFrame(loop);
