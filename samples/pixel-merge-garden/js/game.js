// Pixel Merge Garden - merge-idle garden with orders, mutations, greenhouse
// tiers, wild crops, offline income and a juicy particle layer.

const SAVE_KEY = 'pixel-merge-garden-save';

const state = {
  coins: 24,
  board: Array(SIZE).fill(null),
  bank: 0,
  best: 1,
  greenhouse: 0,        // index into GREENHOUSE
  rain: 0,              // seconds of rain boost remaining
  streak: 0,            // current merge combo
  order: null,
  selected: null,
  lastSave: Date.now(),
};

let fx = [];            // particles + floating text on the overlay canvas

// ---- economy -----------------------------------------------------------
function ghTier() { return GREENHOUSE[state.greenhouse]; }

function seedCost() {
  const filled = state.board.filter(Boolean).length;
  return Math.floor(9 + filled * 2.6 + state.best * 4 + state.greenhouse * 14);
}
function waterCost() { return Math.floor((34 + state.best * 6) * (1 + state.greenhouse * 0.5)); }

function cropValue(c) {
  if (!c) return 0;
  const m = MUTATIONS[c.mutation].mult * (c.wild ? 3 : 1);
  return Math.pow(2, c.level - 1) * 0.16 * m * ghTier().income * (state.rain > 0 ? 2 : 1);
}
function incomeRate() { return state.board.reduce((s, c) => s + cropValue(c), 0); }

function crop(level = 1, mutation = 'plain', wild = false) { return { level, mutation, wild }; }

function rollMutation(a, b) {
  let base = Math.max(MUTATION_RANK.indexOf(a.mutation), MUTATION_RANK.indexOf(b.mutation));
  const chance = 0.05 + state.greenhouse * 0.02 + Math.min(0.25, state.streak * 0.012);
  if (Math.random() < chance) base++;
  if (Math.random() < chance * 0.18) base++; // rare double-bump
  return MUTATION_RANK[Math.min(base, MUTATION_RANK.length - 1)];
}

function newOrder() {
  const reach = Math.max(1, state.best - 1);
  const level = Math.max(2, 2 + Math.floor(Math.random() * reach));
  state.order = {
    level,
    need: 2 + Math.floor(level / 3),
    have: 0,
    reward: Math.floor(40 * Math.pow(1.78, level - 1)),
  };
}

// ---- merge logic -------------------------------------------------------
function canMerge(a, b) { return !!a && !!b && (a.wild || b.wild || a.level === b.level); }

function doMerge(srcIdx, dstIdx) {
  const a = state.board[srcIdx], b = state.board[dstIdx];
  if (!canMerge(a, b) || srcIdx === dstIdx) { state.streak = 0; return false; }

  let level, wild = false;
  if (a.wild && b.wild) { level = Math.max(a.level, b.level) + 1; wild = true; }
  else if (a.wild)      { level = b.level + 1; }
  else if (b.wild)      { level = a.level + 1; }
  else                  { level = a.level + 1; }

  const merged = crop(level, rollMutation(a, b), wild);
  state.board[dstIdx] = merged;
  state.board[srcIdx] = null;
  state.best = Math.max(state.best, level);
  state.streak++;

  const bonus = Math.floor(level * 4 * MUTATIONS[merged.mutation].mult * (1 + state.streak * 0.15));
  state.coins += bonus;

  // order fulfilment
  if (state.order && level === state.order.level) {
    state.order.have++;
    if (state.order.have >= state.order.need) {
      state.coins += state.order.reward;
      floatText(`${t('order_')} +${fmt(state.order.reward)}`, dstIdx, '#f4c85a', 1.4);
      burst(dstIdx, '#f4c85a', 26);
      newOrder();
    }
  }
  floatText(`${t('merge')} +${fmt(bonus)}`, dstIdx, MUTATIONS[merged.mutation].color);
  if (state.streak >= 3) floatText(`${t('combo')} x${state.streak}`, dstIdx, '#ff9ce0', 0.85);
  burst(dstIdx, merged.wild ? '#ffffff' : cropArt(level).leaf, 14 + Math.min(20, state.streak * 2));
  save();
  return true;
}

// ---- actions -----------------------------------------------------------
function buySeed() {
  const cost = seedCost();
  const empty = state.board.map((v, i) => (v ? -1 : i)).filter(i => i >= 0);
  if (state.coins < cost || !empty.length) return;
  state.coins -= cost;
  const luck = 0.04 + ghTier().seedLuck;
  let c;
  if (Math.random() < 0.035 + state.greenhouse * 0.01) {
    c = crop(1, 'plain', true); floatTextCentered(t('wildSeed'), '#ffffff');
  } else if (Math.random() < luck) {
    c = crop(1, 'silver'); floatTextCentered(t('silverSeed'), '#c9d7e8');
  } else {
    c = crop(1, 'plain'); floatTextCentered(t('seed'), '#7fe089');
  }
  const idx = empty[Math.floor(Math.random() * empty.length)];
  state.board[idx] = c;
  burst(idx, c.wild ? '#ffffff' : '#7fe089', 10);
  state.streak = 0;
  renderBoard(); renderStats(); save();
}

function collect() {
  const amt = Math.floor(state.bank);
  if (amt < 1) return;
  state.coins += amt;
  state.bank = 0;
  floatTextCentered(`${t('collectF')} +${fmt(amt)}`, '#f4c85a');
  renderStats(); save();
}

function waterAll() {
  const cost = waterCost();
  if (state.coins < cost || state.rain > 0) return;
  state.coins -= cost;
  state.rain = 30;
  floatTextCentered(t('rainBoost'), '#64c7ff');
  for (let i = 0; i < SIZE; i++) if (state.board[i]) burst(i, '#64c7ff', 6);
  renderStats(); save();
}

function upgradeGreenhouse() {
  if (state.greenhouse >= GREENHOUSE.length - 1) return;
  const cost = GREENHOUSE[state.greenhouse + 1].cost;
  if (state.coins < cost) return;
  state.coins -= cost;
  state.greenhouse++;
  floatTextCentered(`${t('greenhouseUp')} ${state.greenhouse + 1}`, '#a8f0b0');
  renderStats(); save();
}

function resetGame() {
  if (!confirm(t('confirmReset'))) return;
  localStorage.removeItem(SAVE_KEY);
  Object.assign(state, {
    coins: 24, board: Array(SIZE).fill(null), bank: 0, best: 1,
    greenhouse: 0, rain: 0, streak: 0, selected: null, lastSave: Date.now(),
  });
  state.board[12] = crop(1); state.board[7] = crop(1); state.board[13] = crop(1);
  newOrder();
  renderBoard(); renderStats(); save();
}

// ---- save / load / offline --------------------------------------------
function save() {
  state.lastSave = Date.now();
  try {
    localStorage.setItem(SAVE_KEY, JSON.stringify({
      coins: state.coins, board: state.board, bank: state.bank, best: state.best,
      greenhouse: state.greenhouse, order: state.order, lastSave: state.lastSave,
    }));
  } catch (e) { /* storage unavailable - play without persistence */ }
}

function load() {
  let data = null;
  try { data = JSON.parse(localStorage.getItem(SAVE_KEY)); } catch (e) { data = null; }
  if (!data || !Array.isArray(data.board)) {
    state.board[12] = crop(1); state.board[7] = crop(1); state.board[13] = crop(1);
    newOrder();
    return;
  }
  state.coins = data.coins ?? 24;
  state.board = data.board.slice(0, SIZE);
  while (state.board.length < SIZE) state.board.push(null);
  state.bank = data.bank ?? 0;
  state.best = data.best ?? 1;
  state.greenhouse = Math.min(data.greenhouse ?? 0, GREENHOUSE.length - 1);
  state.order = data.order || null;
  if (!state.order) newOrder();
  // Offline income - the garden keeps banking coins while you're away.
  const elapsed = Math.max(0, (Date.now() - (data.lastSave || Date.now())) / 1000);
  const earned = incomeRate() * Math.min(elapsed, OFFLINE_CAP_SECONDS);
  if (earned >= 1) {
    state.bank += earned;
    setTimeout(() => floatTextCentered(`${t('welcomeBack')}: +${fmt(earned)}`, '#f4c85a', 1.6), 400);
  }
}

// ---- rendering ---------------------------------------------------------
const boardEl = document.getElementById('board');

function renderBoard() {
  boardEl.innerHTML = '';
  state.board.forEach((c, i) => {
    const cell = document.createElement('button');
    cell.type = 'button';
    cell.dataset.idx = i;
    cell.className = 'cell'
      + (state.selected === i ? ' selected' : '')
      + (c && c.mutation !== 'plain' ? ` ${c.mutation}` : '')
      + (c && c.wild ? ' wild' : '');
    if (c) {
      const cv = document.createElement('canvas');
      cv.className = 'crop';
      cv.style.animationDelay = ((i * 137) % 1000) + 'ms';
      paintCrop(cv, c);
      cell.appendChild(cv);
      const lbl = document.createElement('span');
      lbl.className = 'level';
      lbl.textContent = `L${c.level}${MUTATIONS[c.mutation].label}`;
      cell.appendChild(lbl);
      if (state.order && c.level === state.order.level && !c.wild) {
        const pin = document.createElement('span');
        pin.className = 'pin'; pin.textContent = '!';
        cell.appendChild(pin);
      }
      const name = c.wild ? t('wildSeed') : cropName(c.level);
      cell.setAttribute('aria-label', `Plot ${i + 1}: level ${c.level} ${name}. Drag onto a matching crop to merge.`);
      cell.title = c.wild ? t('wildHint') : `L${c.level} ${name}`;
    } else {
      cell.setAttribute('aria-label', `Plot ${i + 1}: empty`);
    }
    boardEl.appendChild(cell);
  });
}

function renderStats() {
  set('coins', fmt(state.coins));
  set('bank', fmt(state.bank));
  set('best', `${state.best} ${cropName(state.best)}`);
  set('income', `${incomeRate().toFixed(1)}${t('perSec')}${state.rain > 0 ? ' ' + t('rain') : ''}`);
  set('order', state.order ? `L${state.order.level} ${state.order.have}/${state.order.need} +${fmt(state.order.reward)}` : '-');

  const buy = document.getElementById('buy');
  buy.textContent = `${t('buySeed')} ${fmt(seedCost())}`;
  buy.disabled = state.coins < seedCost() || !state.board.includes(null);

  const col = document.getElementById('collect');
  col.textContent = `${t('collect')} ${fmt(state.bank)}`;
  col.disabled = state.bank < 1;

  const wat = document.getElementById('water');
  wat.textContent = `${t('water')} ${fmt(waterCost())}`;
  wat.disabled = state.coins < waterCost() || state.rain > 0;

  const up = document.getElementById('upgrade');
  const maxed = state.greenhouse >= GREENHOUSE.length - 1;
  up.textContent = maxed
    ? `${t('upgrade')} ${t('maxGreenhouse')}`
    : `${t('upgrade')} ${fmt(GREENHOUSE[state.greenhouse + 1].cost)}`;
  up.disabled = maxed || state.coins < GREENHOUSE[state.greenhouse + 1].cost;

  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
}

function set(id, v) { document.getElementById(id).textContent = v; }
function fmt(n) {
  n = Math.floor(n);
  if (n < 1000) return '' + n;
  const u = ['', 'K', 'M', 'B', 'T'];
  let i = 0;
  while (n >= 1000 && i < u.length - 1) { n /= 1000; i++; }
  return n.toFixed(n < 10 ? 1 : 0) + u[i];
}

// ---- particle / floating-text overlay ---------------------------------
const fxCanvas = document.createElement('canvas');
fxCanvas.id = 'fx';
document.body.appendChild(fxCanvas);
const fxCtx = fxCanvas.getContext('2d');

function resizeFx() {
  fxCanvas.width = innerWidth;
  fxCanvas.height = innerHeight;
}
addEventListener('resize', resizeFx);
resizeFx();

function cellCenter(idx) {
  const cell = boardEl.querySelector(`[data-idx="${idx}"]`);
  if (!cell) return { x: innerWidth / 2, y: innerHeight / 2 };
  const r = cell.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

function burst(idx, color, count) {
  const { x, y } = cellCenter(idx);
  for (let i = 0; i < count; i++) {
    const a = Math.random() * Math.PI * 2, sp = 1.5 + Math.random() * 3.5;
    fx.push({ kind: 'spark', x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 1,
      life: 1, color, size: 2 + Math.random() * 3 });
  }
}
function floatText(text, idx, color = '#f4c85a', scale = 1) {
  const { x, y } = cellCenter(idx);
  fx.push({ kind: 'text', x, y: y - 12, vy: -0.7, life: 1, text, color, scale });
}
function floatTextCentered(text, color = '#f4c85a', scale = 1.2) {
  fx.push({ kind: 'text', x: innerWidth / 2, y: innerHeight * 0.32, vy: -0.5, life: 1, text, color, scale });
}

function fxLoop() {
  fxCtx.clearRect(0, 0, fxCanvas.width, fxCanvas.height);
  for (const f of fx) {
    f.life -= f.kind === 'text' ? 0.012 : 0.022;
    if (f.kind === 'spark') {
      f.x += f.vx; f.y += f.vy; f.vy += 0.16; f.vx *= 0.98;
      fxCtx.globalAlpha = Math.max(0, f.life);
      fxCtx.fillStyle = f.color;
      fxCtx.fillRect(f.x | 0, f.y | 0, f.size, f.size);
    } else {
      f.y += f.vy;
      fxCtx.globalAlpha = Math.max(0, Math.min(1, f.life * 1.4));
      const px = Math.round(13 * f.scale);
      fxCtx.font = `900 ${px}px ui-monospace, Menlo, monospace`;
      fxCtx.textAlign = 'center';
      fxCtx.lineWidth = 4; fxCtx.strokeStyle = '#000';
      fxCtx.strokeText(f.text, f.x, f.y);
      fxCtx.fillStyle = f.color;
      fxCtx.fillText(f.text, f.x, f.y);
    }
  }
  fxCtx.globalAlpha = 1;
  fx = fx.filter(f => f.life > 0);
  requestAnimationFrame(fxLoop);
}
requestAnimationFrame(fxLoop);

// ---- input: drag-to-merge + tap-to-merge ------------------------------
let drag = null; // { from, ghost, moved }

function cellIndexAt(clientX, clientY) {
  const el = document.elementFromPoint(clientX, clientY);
  const cell = el && el.closest('.cell');
  return cell ? +cell.dataset.idx : -1;
}

function highlight(idx) {
  boardEl.querySelectorAll('.cell.target').forEach(c => c.classList.remove('target'));
  if (idx >= 0 && drag && idx !== drag.from && canMerge(state.board[drag.from], state.board[idx])) {
    boardEl.querySelector(`[data-idx="${idx}"]`).classList.add('target');
  }
}

boardEl.addEventListener('pointerdown', e => {
  const cell = e.target.closest('.cell');
  if (!cell) return;
  const idx = +cell.dataset.idx;
  if (!state.board[idx]) { state.selected = null; renderBoard(); renderStats(); return; }
  drag = { from: idx, moved: false, ghost: null };
  boardEl.setPointerCapture(e.pointerId);
});

boardEl.addEventListener('pointermove', e => {
  if (!drag) return;
  if (!drag.moved) {
    drag.moved = true;
    const ghost = document.createElement('canvas');
    ghost.className = 'crop drag-ghost';
    paintCrop(ghost, state.board[drag.from]);
    document.body.appendChild(ghost);
    drag.ghost = ghost;
    boardEl.querySelector(`[data-idx="${drag.from}"]`).classList.add('lifting');
  }
  drag.ghost.style.left = e.clientX + 'px';
  drag.ghost.style.top = e.clientY + 'px';
  highlight(cellIndexAt(e.clientX, e.clientY));
});

boardEl.addEventListener('pointerup', e => {
  if (!drag) return;
  const from = drag.from;
  const dropped = drag.moved;
  if (drag.ghost) drag.ghost.remove();
  drag = null;
  boardEl.querySelectorAll('.lifting,.target').forEach(c => c.classList.remove('lifting', 'target'));

  const target = cellIndexAt(e.clientX, e.clientY);

  if (dropped && target >= 0 && target !== from) {
    if (canMerge(state.board[from], state.board[target])) {
      doMerge(from, target);
      state.selected = null;
    }
    renderBoard(); renderStats();
    return;
  }
  // No drag: tap-to-select / tap-to-merge.
  if (state.selected === null) {
    state.selected = from;
  } else if (state.selected === from) {
    state.selected = null;
  } else if (canMerge(state.board[state.selected], state.board[from])) {
    doMerge(state.selected, from);
    state.selected = null;
  } else {
    state.selected = from;
  }
  renderBoard(); renderStats();
});

boardEl.addEventListener('pointercancel', () => {
  if (drag && drag.ghost) drag.ghost.remove();
  drag = null;
  boardEl.querySelectorAll('.lifting,.target').forEach(c => c.classList.remove('lifting', 'target'));
});

// ---- wiring ------------------------------------------------------------
document.getElementById('buy').onclick = buySeed;
document.getElementById('collect').onclick = collect;
document.getElementById('water').onclick = waterAll;
document.getElementById('upgrade').onclick = upgradeGreenhouse;
document.getElementById('reset').onclick = resetGame;
setupLanguageToggle(() => { renderBoard(); renderStats(); });

// economy tick - bank passive income, count down rain.
setInterval(() => {
  state.bank += incomeRate();
  if (state.rain > 0) {
    state.rain--;
    if (state.rain === 0) renderBoard();
  }
  renderStats();
}, 1000);

// periodic autosave keeps offline income accurate.
setInterval(save, 5000);
addEventListener('beforeunload', save);

load();
renderBoard();
renderStats();
