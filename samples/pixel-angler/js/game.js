// Pixel Angler - cast/hook/reel loop, reeling minigame, shop, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-angler-save';
function loadProg() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.coins === 'number') {
      return {
        coins: d.coins, rod: d.rod || 0, reel: d.reel || 0,
        watersUnlocked: d.watersUnlocked || 1, dex: d.dex || {},
      };
    }
  } catch (e) { /* ignore */ }
  return { coins: 0, rod: 0, reel: 0, watersUnlocked: 1, dex: {} };
}
function saveProg() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(prog)); } catch (e) { /* ignore */ }
}
let prog = loadProg();

const BOBBER_X = 250, BOBBER_Y = 250;
const TRACK_X = 130, TRACK_Y = 112, TRACK_W = 60, TRACK_H = 296;
let state = null;
let holding = false;

function curWaters() { return prog.watersUnlocked - 1; }

// ---- phase machine -------------------------------------------------------
function newSession() {
  state = { phase: 'idle', timer: 0, fish: null, reel: null, result: '', bob: 0 };
  updateHud();
}

function startCast() {
  if (!state || state.phase !== 'idle') return;
  state.phase = 'wait';
  state.timer = 1.6 + Math.random() * 2.8;
}

function startHook() {
  state.phase = 'hook';
  state.timer = 1.15;
  state.fish = pickFish(curWaters(), Math.random);
}

function startReel() {
  const f = state.fish;
  state.phase = 'reel';
  const zoneH = 64 + prog.rod * 15;
  state.reel = {
    zoneH, zy: TRACK_Y + TRACK_H / 2, zvel: 0,
    fishY: TRACK_Y + TRACK_H * 0.5,
    targetY: TRACK_Y + TRACK_H * 0.5, retarget: 0,
    progress: 32,
    fillRate: 30 + prog.reel * 6,
    drainRate: 22 + f.diff * 9,
    fishSpeed: 40 + f.diff * 55,
  };
}

function finishReel(won) {
  const f = state.fish;
  state.phase = 'result';
  state.timer = 1.9;
  if (won) {
    prog.coins += f.value;
    prog.dex[f.id] = (prog.dex[f.id] || 0) + 1;
    saveProg();
    state.result = t('caught', f.name[currentLang === 'zh' ? 1 : 0]);
  } else {
    state.result = t('escaped');
  }
  updateHud();
}

// ---- update --------------------------------------------------------------
function update(dt) {
  if (!state) return;
  state.bob += dt;
  if (state.phase === 'wait') {
    state.timer -= dt;
    if (state.timer <= 0) startHook();
  } else if (state.phase === 'hook') {
    state.timer -= dt;
    if (state.timer <= 0) {
      state.phase = 'result';
      state.timer = 1.8;
      state.result = t('missed');
    }
  } else if (state.phase === 'reel') {
    updateReel(dt);
  } else if (state.phase === 'result') {
    state.timer -= dt;
    if (state.timer <= 0) { state.phase = 'idle'; state.fish = null; }
  }
}

function updateReel(dt) {
  const r = state.reel;
  // catch bar — hold to rise, gravity pulls it down
  r.zvel += (holding ? -470 : 520) * dt;
  r.zvel *= 0.86;
  r.zy += r.zvel * dt;
  const lo = TRACK_Y + r.zoneH / 2, hi = TRACK_Y + TRACK_H - r.zoneH / 2;
  if (r.zy < lo) { r.zy = lo; r.zvel = 0; }
  if (r.zy > hi) { r.zy = hi; r.zvel = 0; }
  // fish — darts toward a roaming target
  r.retarget -= dt;
  if (r.retarget <= 0) {
    r.targetY = TRACK_Y + 14 + Math.random() * (TRACK_H - 28);
    r.retarget = 0.45 + Math.random() * 1.0;
  }
  const d = r.targetY - r.fishY;
  r.fishY += Math.sign(d) * Math.min(Math.abs(d), r.fishSpeed * dt);
  // progress
  const overlap = Math.abs(r.fishY - r.zy) < r.zoneH / 2;
  r.progress += (overlap ? r.fillRate : -r.drainRate) * dt;
  if (r.progress >= 100) { r.progress = 100; finishReel(true); }
  else if (r.progress <= 0) { r.progress = 0; finishReel(false); }
}

// ---- render --------------------------------------------------------------
function render() {
  if (!state) { drawScene(ctx, curWaters(), null, null); return; }
  const inWater = state.phase === 'wait' || state.phase === 'hook';
  drawScene(ctx, curWaters(), BOBBER_X, inWater ? BOBBER_Y : null);

  if (inWater) {
    const dip = state.phase === 'hook' ? 5 : Math.sin(state.bob * 3) * 2;
    drawBobber(ctx, BOBBER_X, BOBBER_Y, dip);
  }
  if (state.phase === 'reel') drawReel();

  ctx.textAlign = 'center';
  ctx.font = '900 15px ui-monospace, monospace';
  let msg = '';
  if (state.phase === 'idle') msg = t('castMsg');
  else if (state.phase === 'wait') msg = t('waiting');
  else if (state.phase === 'hook') msg = t('bite');
  else if (state.phase === 'reel') msg = t('reelMsg');
  else if (state.phase === 'result') msg = state.result;
  ctx.fillStyle = state.phase === 'hook' ? '#ffd23f' : '#e8f0f6';
  ctx.fillText(msg, VW / 2, VH - 64);
  ctx.textAlign = 'left';
}

function drawReel() {
  const r = state.reel;
  ctx.fillStyle = '#0c1e2e';
  ctx.fillRect(TRACK_X - 2, TRACK_Y - 2, TRACK_W + 4, TRACK_H + 4);
  ctx.fillStyle = '#15324a';
  ctx.fillRect(TRACK_X, TRACK_Y, TRACK_W, TRACK_H);
  // catch zone
  ctx.fillStyle = 'rgba(95,192,110,0.42)';
  ctx.fillRect(TRACK_X, r.zy - r.zoneH / 2, TRACK_W, r.zoneH);
  ctx.strokeStyle = '#5fc06e';
  ctx.lineWidth = 2;
  ctx.strokeRect(TRACK_X + 1, r.zy - r.zoneH / 2 + 1, TRACK_W - 2, r.zoneH - 2);
  // fish
  drawFish(ctx, TRACK_X + TRACK_W / 2, r.fishY, 13 * (0.7 + state.fish.size * 0.4),
    state.fish.color, 1);
  // progress gauge
  const px = TRACK_X + TRACK_W + 12, pw = 20;
  ctx.fillStyle = '#0c1e2e';
  ctx.fillRect(px - 2, TRACK_Y - 2, pw + 4, TRACK_H + 4);
  ctx.fillStyle = '#15324a';
  ctx.fillRect(px, TRACK_Y, pw, TRACK_H);
  const h = TRACK_H * r.progress / 100;
  ctx.fillStyle = r.progress > 60 ? '#5fc06e' : r.progress > 25 ? '#f2cf3f' : '#e8554f';
  ctx.fillRect(px, TRACK_Y + TRACK_H - h, pw, h);
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  document.getElementById('hud-coins').textContent = t('coins', prog.coins);
  document.getElementById('hud-waters').textContent =
    WATERS[curWaters()].name[currentLang === 'zh' ? 1 : 0];
}

// ---- shop ----------------------------------------------------------------
function buildShop() {
  document.getElementById('shop-coins').textContent = t('coins', prog.coins);
  const list = document.getElementById('shop-list');
  list.innerHTML = '';
  const items = [
    { key: 'rod', label: t('rod'), desc: t('rodDesc'), level: prog.rod,
      cost: rodCost(prog.rod), max: MAX_UPGRADE },
    { key: 'reel', label: t('reel'), desc: t('reelDesc'), level: prog.reel,
      cost: reelCost(prog.reel), max: MAX_UPGRADE },
  ];
  for (const it of items) {
    const maxed = it.level >= it.max;
    const row = document.createElement('div');
    row.className = 'shop-item';
    row.innerHTML = '<div class="info"><b>' + it.label + ' Lv ' + it.level +
      '</b><span>' + it.desc + '</span></div>';
    const btn = document.createElement('button');
    btn.textContent = maxed ? t('maxed') : t('coins', it.cost);
    btn.disabled = maxed || prog.coins < it.cost;
    btn.onclick = () => {
      if (prog.coins < it.cost) return;
      prog.coins -= it.cost;
      prog[it.key]++;
      saveProg();
      buildShop();
      updateHud();
    };
    row.appendChild(btn);
    list.appendChild(row);
  }
  if (prog.watersUnlocked < WATERS.length) {
    const nw = WATERS[prog.watersUnlocked];
    const row = document.createElement('div');
    row.className = 'shop-item';
    row.innerHTML = '<div class="info"><b>' + nw.name[currentLang === 'zh' ? 1 : 0] +
      '</b><span>' + t('unlock') + '</span></div>';
    const btn = document.createElement('button');
    btn.textContent = t('coins', nw.unlockCost);
    btn.disabled = prog.coins < nw.unlockCost;
    btn.onclick = () => {
      if (prog.coins < nw.unlockCost) return;
      prog.coins -= nw.unlockCost;
      prog.watersUnlocked++;
      saveProg();
      buildShop();
      updateHud();
    };
    row.appendChild(btn);
    list.appendChild(row);
  }
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', () => {
  if (!state || document.getElementById('screen-game').classList.contains('hidden')) return;
  if (state.phase === 'hook') startReel();
  else if (state.phase === 'reel') holding = true;
});
canvas.addEventListener('pointerup', () => { holding = false; });
canvas.addEventListener('pointercancel', () => { holding = false; });

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function refreshTitle() {
  document.getElementById('title-coins').textContent =
    prog.coins > 0 ? t('coinsLine', prog.coins) : '';
}

document.getElementById('btn-play').onclick = () => { newSession(); updateHud(); showScreen('screen-game'); };
document.getElementById('btn-cast').onclick = startCast;
document.getElementById('btn-shop').onclick = () => {
  if (state && state.phase === 'reel') return;
  buildShop();
  showScreen('screen-shop');
};
document.getElementById('btn-shop-back').onclick = () => { updateHud(); showScreen('screen-game'); };
document.getElementById('btn-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };

setupLanguageToggle(() => {
  updateHud();
  refreshTitle();
  if (!document.getElementById('screen-shop').classList.contains('hidden')) buildShop();
});

// ---- main loop -----------------------------------------------------------
let last = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  if (!document.getElementById('screen-game').classList.contains('hidden')) {
    update(dt);
    render();
  } else {
    drawScene(ctx, curWaters(), null, null);
  }
  requestAnimationFrame(loop);
}
refreshTitle();
showScreen('screen-title');
requestAnimationFrame(loop);
