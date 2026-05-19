// Pixel Sudoku - board state, input, win detection, progress save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-sudoku-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, stars: d.stars || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, stars: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

const CELL = 34, GX = 27, GY = 54;
const NPY = 374, NPH = 38;
let state = null;
let notesMode = false;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const lv = LEVELS[index];
  const { puzzle, solution } = generatePuzzle(lv.seed, lv.holes);
  state = {
    index, lv, solution,
    value: puzzle.slice(),
    given: puzzle.map(v => v !== 0),
    notes: Array.from({ length: 81 }, () => new Set()),
    selected: -1, mistakes: 0, hints: 0, time: 0, running: false, won: false,
  };
  notesMode = false;
  updateNotesBtn();
  updateHud();
}

function peers(i) {
  const r = (i / 9) | 0, c = i % 9, out = [];
  for (let k = 0; k < 9; k++) {
    if (k !== c) out.push(r * 9 + k);
    if (k !== r) out.push(k * 9 + c);
  }
  const br = Math.floor(r / 3) * 3, bc = Math.floor(c / 3) * 3;
  for (let dr = 0; dr < 3; dr++) {
    for (let dc = 0; dc < 3; dc++) {
      const p = (br + dr) * 9 + (bc + dc);
      if (p !== i) out.push(p);
    }
  }
  return out;
}

// ---- actions -------------------------------------------------------------
function placeDigit(d) {
  const i = state.selected;
  if (i < 0 || state.given[i] || state.won) return;
  if (!state.running) state.running = true;
  if (notesMode) {
    if (state.value[i] !== 0) return;
    if (state.notes[i].has(d)) state.notes[i].delete(d);
    else state.notes[i].add(d);
    return;
  }
  state.value[i] = d;
  state.notes[i].clear();
  if (d !== state.solution[i]) state.mistakes++;
  else for (const p of peers(i)) state.notes[p].delete(d);
  updateHud();
  checkWin();
}

function eraseCell() {
  const i = state.selected;
  if (i < 0 || state.given[i] || state.won) return;
  if (state.value[i] !== 0) state.value[i] = 0;
  else state.notes[i].clear();
}

function useHint() {
  if (!state || state.won) return;
  let target = -1;
  if (state.selected >= 0 && !state.given[state.selected] &&
      state.value[state.selected] !== state.solution[state.selected]) {
    target = state.selected;
  } else {
    for (let i = 0; i < 81; i++) {
      if (state.value[i] !== state.solution[i]) { target = i; break; }
    }
  }
  if (target < 0) return;
  state.value[target] = state.solution[target];
  state.notes[target].clear();
  state.hints++;
  state.selected = target;
  if (!state.running) state.running = true;
  updateHud();
  checkWin();
}

function checkWin() {
  for (let i = 0; i < 81; i++) if (state.value[i] !== state.solution[i]) return;
  state.won = true;
  state.running = false;
  const penalty = state.mistakes + state.hints;
  const stars = penalty === 0 ? 3 : penalty <= 3 ? 2 : 1;
  const idx = state.index;
  if ((progress.stars[idx] || 0) < stars) progress.stars[idx] = stars;
  if (idx + 1 < LEVEL_COUNT && progress.unlocked < idx + 2) progress.unlocked = idx + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', fmtTime(state.time), state.mistakes);
    document.getElementById('btn-next').style.display = idx + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 550);
}

// ---- HUD -----------------------------------------------------------------
function fmtTime(s) {
  const m = Math.floor(s / 60), sec = Math.floor(s % 60);
  return m + ':' + String(sec).padStart(2, '0');
}
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent =
    t('tiers')[state.lv.tier] + ' ' + (state.index + 1);
  document.getElementById('hud-miss').textContent = '✕ ' + state.mistakes;
  document.getElementById('hud-time').textContent = fmtTime(state.time);
}
function updateNotesBtn() {
  document.getElementById('btn-notes').classList.toggle('on', notesMode);
}

// ---- render --------------------------------------------------------------
function digitCount(d) {
  let n = 0;
  for (let i = 0; i < 81; i++) if (state.value[i] === d && state.value[i] === state.solution[i]) n++;
  return n;
}
function render() {
  drawBackground(ctx);
  if (!state) return;
  const sel = state.selected;
  const selVal = sel >= 0 ? state.value[sel] : 0;
  const peerSet = sel >= 0 ? new Set(peers(sel)) : new Set();

  for (let i = 0; i < 81; i++) {
    const r = (i / 9) | 0, c = i % 9;
    const x = GX + c * CELL, y = GY + r * CELL;
    let bg = '#131a2e';
    if (i === sel) bg = '#33457a';
    else if (selVal && state.value[i] === selVal) bg = '#26345c';
    else if (peerSet.has(i)) bg = '#1b2440';
    ctx.fillStyle = bg;
    ctx.fillRect(x, y, CELL, CELL);
    ctx.strokeStyle = '#28324c';
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, y + 0.5, CELL - 1, CELL - 1);

    const v = state.value[i];
    if (v !== 0) {
      const color = state.given[i] ? '#e9edf6'
        : (v === state.solution[i] ? '#6ea8ff' : '#ff6b6b');
      drawDigit(ctx, v, x + CELL / 2, y + CELL / 2, 4, color);
    } else if (state.notes[i].size) {
      for (const d of state.notes[i]) {
        const sr = ((d - 1) / 3) | 0, sc = (d - 1) % 3;
        drawDigit(ctx, d, x + (sc + 0.5) * CELL / 3, y + (sr + 0.5) * CELL / 3, 1, '#7c88a8');
      }
    }
  }
  // box separators
  ctx.fillStyle = '#4a5680';
  for (let b = 0; b <= 9; b += 3) {
    ctx.fillRect(GX + b * CELL - 1, GY - 1, 2, 9 * CELL + 2);
    ctx.fillRect(GX - 1, GY + b * CELL - 1, 9 * CELL + 2, 2);
  }

  // number pad
  for (let k = 0; k < 9; k++) {
    const d = k + 1;
    const x = GX + k * CELL;
    const remaining = 9 - digitCount(d);
    ctx.fillStyle = remaining <= 0 ? '#161d30' : '#222d4c';
    ctx.fillRect(x, NPY, CELL - 1, NPH);
    drawDigit(ctx, d, x + CELL / 2, NPY + NPH / 2 - 2, 3,
      remaining <= 0 ? '#3c4660' : '#cfe0ff');
    ctx.fillStyle = remaining <= 0 ? '#3c4660' : '#6e7da0';
    ctx.font = '8px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(String(Math.max(0, remaining)), x + CELL / 2, NPY + NPH - 3);
  }
  ctx.textAlign = 'left';
}

// ---- update --------------------------------------------------------------
function update(dt) {
  if (state && state.running && !state.won) { state.time += dt; updateHud(); }
}

// ---- input ---------------------------------------------------------------
function onTap(px, py) {
  if (!state || state.won) return;
  if (px >= GX && px < GX + 9 * CELL && py >= GY && py < GY + 9 * CELL) {
    const c = Math.floor((px - GX) / CELL), r = Math.floor((py - GY) / CELL);
    state.selected = r * 9 + c;
    return;
  }
  if (px >= GX && px < GX + 9 * CELL && py >= NPY && py < NPY + NPH) {
    placeDigit(Math.floor((px - GX) / CELL) + 1);
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
  LEVELS.forEach((lv, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    btn.innerHTML = '<b>' + t('tiers')[lv.tier] + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) + '</span>';
    if (!locked) btn.onclick = () => startLevel(i);
    grid.appendChild(btn);
  });
}
function startLevel(index) {
  buildLevel(index);
  showScreen('screen-game');
}

document.getElementById('btn-play').onclick = () => {
  let next = 0;
  for (let i = 0; i < LEVEL_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startLevel(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-notes').onclick = () => { notesMode = !notesMode; updateNotesBtn(); };
document.getElementById('btn-erase').onclick = eraseCell;
document.getElementById('btn-hint').onclick = useHint;
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
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
