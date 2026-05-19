// Pixel Mahjong - solvable layout generation, free-tile matching, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-mahjong-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') {
      return { unlocked: d.unlocked, stars: d.stars || {}, bestTime: d.bestTime || {} };
    }
  } catch (e) { /* ignore */ }
  return { unlocked: 1, stars: {}, bestTime: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

let state = null;

// ---- board construction --------------------------------------------------
function key(L, r, c) { return L + '_' + r + '_' + c; }

// Is a tile free? — nothing on top, and a clear left OR right edge.
function tileFree(t, present) {
  if (t.aboveId >= 0 && present.has(t.aboveId)) return false;
  const leftBlocked = t.leftId >= 0 && present.has(t.leftId);
  const rightBlocked = t.rightId >= 0 && present.has(t.rightId);
  return !leftBlocked || !rightBlocked;
}

// Assign tile faces so the (sub)board is guaranteed solvable: repeatedly pick
// two currently-free positions and give them a matching pair.
function generateTypes(pool) {
  for (let attempt = 0; attempt < 90; attempt++) {
    const present = new Set(pool.map(t => t.id));
    const order = [];
    let ok = true;
    while (present.size) {
      const free = [];
      for (const t of pool) if (present.has(t.id) && tileFree(t, present)) free.push(t.id);
      if (free.length < 2) { ok = false; break; }
      const i = (Math.random() * free.length) | 0;
      let j = (Math.random() * free.length) | 0;
      while (j === i) j = (Math.random() * free.length) | 0;
      order.push([free[i], free[j]]);
      present.delete(free[i]);
      present.delete(free[j]);
    }
    if (ok) {
      const types = {};
      order.forEach((pair, k) => {
        const tp = k % SYMBOL_COUNT;
        types[pair[0]] = tp;
        types[pair[1]] = tp;
      });
      // `order` is itself a valid solve sequence for the dealt board.
      return { types, order };
    }
  }
  return null;
}

function buildBoard(index) {
  const layout = LAYOUTS[index];
  const cells = layoutCells(layout);
  const posMap = {};
  cells.forEach((c, i) => { posMap[key(c.layer, c.row, c.col)] = i; });
  const tiles = cells.map((c, i) => ({
    id: i, layer: c.layer, row: c.row, col: c.col,
    aboveId: posMap[key(c.layer + 1, c.row, c.col)] ?? -1,
    leftId: posMap[key(c.layer, c.row, c.col - 1)] ?? -1,
    rightId: posMap[key(c.layer, c.row, c.col + 1)] ?? -1,
    type: 0, removed: false,
  }));
  const gen = generateTypes(tiles);
  tiles.forEach((t, i) => { t.type = gen ? gen.types[i] : (i >> 1) % SYMBOL_COUNT; });

  state = {
    index, layout, tiles,
    present: new Set(tiles.map(t => t.id)),
    selected: -1, time: 0, running: false, won: false,
    shuffles: 0, hints: 0, hint: null, stuck: false,
    particles: [], drawOrder: tiles.map(t => t.id).sort((a, b) => {
      const A = tiles[a], B = tiles[b];
      return A.layer - B.layer || A.row - B.row || A.col - B.col;
    }),
  };
  computeLayout();
  updateHud();
}

function computeLayout() {
  const lo = state.layout;
  state.tileW = Math.min(40, Math.floor(340 / (lo.w + 0.6)));
  state.tileH = Math.min(46, Math.floor(372 / (lo.h + 0.8)));
  state.depth = 5;
  let minX = 1e9, minY = 1e9, maxX = -1e9, maxY = -1e9;
  for (const t of state.tiles) {
    const x = t.col * state.tileW - t.layer * state.depth;
    const y = t.row * state.tileH - t.layer * state.depth;
    minX = Math.min(minX, x); minY = Math.min(minY, y);
    maxX = Math.max(maxX, x + state.tileW); maxY = Math.max(maxY, y + state.tileH);
  }
  state.boardX = Math.round((VW - (maxX - minX)) / 2 - minX);
  state.boardY = Math.round(54 + (376 - (maxY - minY)) / 2 - minY);
}
function tileX(t) { return state.boardX + t.col * state.tileW - t.layer * state.depth; }
function tileY(t) { return state.boardY + t.row * state.tileH - t.layer * state.depth; }

// ---- matching ------------------------------------------------------------
function freePresentTiles() {
  return state.tiles.filter(t => state.present.has(t.id) && tileFree(t, state.present));
}
function findFreePair() {
  const free = freePresentTiles();
  for (let i = 0; i < free.length; i++) {
    for (let j = i + 1; j < free.length; j++) {
      if (free[i].type === free[j].type) return [free[i].id, free[j].id];
    }
  }
  return null;
}

function removePair(a, b) {
  state.tiles[a].removed = true;
  state.tiles[b].removed = true;
  state.present.delete(a);
  state.present.delete(b);
  for (const id of [a, b]) {
    const t = state.tiles[id];
    burst(tileX(t) + state.tileW / 2, tileY(t) + state.tileH / 2, SYMBOL_COLORS[t.type]);
  }
  state.selected = -1;
  updateHud();
  if (state.present.size === 0) { winLevel(); return; }
  state.stuck = !findFreePair();
}

function burst(x, y, color) {
  for (let i = 0; i < 9; i++) {
    const a = Math.random() * Math.PI * 2, s = 40 + Math.random() * 120;
    state.particles.push({
      x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s - 30,
      life: 0.4 + Math.random() * 0.4, color,
    });
  }
}

function tapTile(id) {
  const t = state.tiles[id];
  if (!tileFree(t, state.present)) return;
  if (!state.running) state.running = true;
  if (state.selected === -1) {
    state.selected = id;
  } else if (state.selected === id) {
    state.selected = -1;
  } else {
    const other = state.tiles[state.selected];
    if (other.type === t.type) removePair(other.id, id);
    else state.selected = id;
  }
}

function doShuffle() {
  if (!state || state.won) return;
  const remaining = state.tiles.filter(t => !t.removed);
  if (remaining.length === 0) return;
  const gen = generateTypes(remaining);
  if (!gen) return;
  for (const t of remaining) t.type = gen.types[t.id];
  state.shuffles++;
  state.selected = -1;
  state.hint = null;
  state.stuck = !findFreePair();
}

function doHint() {
  if (!state || state.won) return;
  const pair = findFreePair();
  if (pair) {
    state.hint = { ids: pair, t: 1.7 };
    state.hints++;
  }
}

// ---- win -----------------------------------------------------------------
function winLevel() {
  state.won = true;
  state.running = false;
  const tm = Math.floor(state.time);
  const penalty = state.shuffles + state.hints;
  const stars = penalty === 0 ? 3 : penalty <= 2 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (!progress.bestTime[i] || tm < progress.bestTime[i]) progress.bestTime[i] = tm;
  if (i + 1 < LAYOUT_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  for (let k = 0; k < 40; k++) {
    state.particles.push({
      x: Math.random() * VW, y: VH * 0.4,
      vx: (Math.random() - 0.5) * 150, vy: -60 - Math.random() * 150,
      life: 0.7 + Math.random() * 0.6,
      color: SYMBOL_COLORS[(Math.random() * SYMBOL_COUNT) | 0],
    });
  }
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', fmtTime(tm), state.shuffles);
    document.getElementById('btn-next').style.display =
      state.index + 1 < LAYOUT_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 800);
}

// ---- update / render -----------------------------------------------------
function update(dt) {
  if (!state) return;
  if (state.running && !state.won) state.time += dt;
  if (state.hint) { state.hint.t -= dt; if (state.hint.t <= 0) state.hint = null; }
  for (const p of state.particles) {
    p.life -= dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 320 * dt;
  }
  state.particles = state.particles.filter(p => p.life > 0);
}

function render() {
  drawBackground(ctx);
  if (!state) return;
  for (const id of state.drawOrder) {
    const t = state.tiles[id];
    if (t.removed) continue;
    const free = tileFree(t, state.present);
    let st = 'blocked';
    if (state.selected === id) st = 'selected';
    else if (state.hint && state.hint.ids.includes(id)) st = 'hint';
    else if (free) st = 'free';
    drawTile(ctx, tileX(t), tileY(t), state.tileW, state.tileH, t.type, st);
  }
  for (const p of state.particles) {
    ctx.globalAlpha = Math.min(1, p.life * 2.4);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x | 0, p.y | 0, 3, 3);
  }
  ctx.globalAlpha = 1;
  if (state.stuck && !state.won) {
    ctx.fillStyle = '#ff6b6b';
    ctx.font = '900 16px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t('noMoves'), VW / 2, 44);
    ctx.textAlign = 'left';
  }
}

// ---- HUD -----------------------------------------------------------------
function fmtTime(s) {
  const m = Math.floor(s / 60), sec = Math.floor(s % 60);
  return m + ':' + String(sec).padStart(2, '0');
}
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-tiles').textContent = t('tiles') + ' ' + state.present.size;
  document.getElementById('hud-time').textContent = fmtTime(state.time);
}

// ---- input ---------------------------------------------------------------
function onTap(px, py) {
  if (!state || state.won) return;
  // topmost present tile under the point (reverse draw order)
  for (let i = state.drawOrder.length - 1; i >= 0; i--) {
    const t = state.tiles[state.drawOrder[i]];
    if (t.removed) continue;
    const x = tileX(t), y = tileY(t);
    if (px >= x && px < x + state.tileW && py >= y && py < y + state.tileH) {
      tapTile(t.id);
      return;
    }
  }
}
canvas.addEventListener('pointerdown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  onTap((e.clientX - rect.left) * VW / rect.width, (e.clientY - rect.top) * VH / rect.height);
});

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  hideAllOverlays();
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() {
  document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
}
function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  LAYOUTS.forEach((lo, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    const tiles = layoutCells(lo).length;
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) +
      '</span><em>' + tiles + ' ' + lo.name[currentLang === 'zh' ? 1 : 0] + '</em>';
    if (!locked) btn.onclick = () => startLevel(i);
    grid.appendChild(btn);
  });
}
function startLevel(index) {
  buildBoard(index);
  showScreen('screen-game');
}

document.getElementById('btn-play').onclick = () => {
  let next = 0;
  for (let i = 0; i < LAYOUT_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startLevel(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-shuffle').onclick = doShuffle;
document.getElementById('btn-hint').onclick = doHint;
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LAYOUT_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
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
    drawBackground(ctx);
  }
  requestAnimationFrame(loop);
}
showScreen('screen-title');
requestAnimationFrame(loop);
