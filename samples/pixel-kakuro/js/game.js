// Pixel Kakuro - puzzle play, digit pad, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-kakuro-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, done: d.done || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, done: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

// digit pad geometry
const PAD = { cell: 32, y: 352 };
PAD.x = Math.round((VW - PAD.cell * 9) / 2);
const ERASE = { x: 110, y: 400, w: 140, h: 40 };

let state = null;   // { index, pz, geom, fills, selected, badCells, won }

function buildLevel(index) {
  const pz = buildPuzzle(PUZZLES[index]);
  state = {
    index, pz,
    geom: boardGeom(pz),
    fills: {}, selected: null, badCells: new Set(), won: false,
  };
  recomputeBad();
  updateHud();
}

// ---- run validation ------------------------------------------------------
function recomputeBad() {
  const bad = new Set();
  for (const run of state.pz.runs) {
    const seen = {};
    let sum = 0, filled = 0;
    for (const [r, c] of run.cells) {
      const v = state.fills[r + ',' + c];
      if (v) {
        filled++; sum += v;
        (seen[v] || (seen[v] = [])).push(r + ',' + c);
      }
    }
    for (const d in seen) if (seen[d].length > 1) seen[d].forEach(k => bad.add(k));
    if (filled === run.cells.length && sum !== run.sum) {
      run.cells.forEach(([r, c]) => bad.add(r + ',' + c));
    }
  }
  state.badCells = bad;
}

function checkWin() {
  for (let r = 0; r < state.pz.h; r++) {
    for (let c = 0; c < state.pz.w; c++) {
      if (state.pz.white[r][c] && !state.fills[r + ',' + c]) return;
    }
  }
  if (state.badCells.size > 0) return;
  state.won = true;
  const i = state.index;
  progress.done[i] = true;
  if (i + 1 < PUZZLE_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = t('win');
    document.getElementById('win-line').textContent = t('winLine');
    document.getElementById('btn-next').style.display = i + 1 < PUZZLE_COUNT ? '' : 'none';
    document.getElementById('overlay-win').classList.remove('hidden');
  }, 400);
}

function setDigit(d) {
  if (!state.selected || state.won) return;
  if (d === 0) delete state.fills[state.selected];
  else state.fills[state.selected] = d;
  recomputeBad();
  checkWin();
}

// ---- render --------------------------------------------------------------
function drawPad(ctx) {
  for (let i = 0; i < 9; i++) {
    const x = PAD.x + i * PAD.cell;
    ctx.fillStyle = '#222a42';
    ctx.fillRect(x + 1, PAD.y + 1, PAD.cell - 2, PAD.cell - 2);
    ctx.strokeStyle = '#3a4566';
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 1.5, PAD.y + 1.5, PAD.cell - 3, PAD.cell - 3);
    ctx.fillStyle = '#e9edf7';
    ctx.font = 'bold 17px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(i + 1), x + PAD.cell / 2, PAD.y + PAD.cell / 2 + 1);
  }
  ctx.fillStyle = '#3a2a32';
  ctx.fillRect(ERASE.x, ERASE.y, ERASE.w, ERASE.h);
  ctx.strokeStyle = '#6a4452';
  ctx.strokeRect(ERASE.x + 0.5, ERASE.y + 0.5, ERASE.w - 1, ERASE.h - 1);
  ctx.fillStyle = '#ff9caa';
  ctx.font = 'bold 13px monospace';
  ctx.fillText('⌫ ' + t('erase'), ERASE.x + ERASE.w / 2, ERASE.y + ERASE.h / 2 + 1);
}

function render() {
  drawBackground(ctx);
  if (!state) return;
  drawBoard(ctx, state.pz, state.geom, state);
  drawPad(ctx);
}
function loop() {
  render();
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent =
    t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-name').textContent =
    PUZZLES[state.index].name[lang === 'en' ? 0 : 1];
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const { cell, gx, gy } = state.geom;
  // board
  const c = Math.floor((px - gx) / cell), r = Math.floor((py - gy) / cell);
  if (r >= 0 && c >= 0 && r < state.pz.h && c < state.pz.w && state.pz.white[r][c]) {
    state.selected = r + ',' + c;
    return;
  }
  // digit pad
  if (py >= PAD.y && py < PAD.y + PAD.cell && px >= PAD.x && px < PAD.x + PAD.cell * 9) {
    setDigit(Math.floor((px - PAD.x) / PAD.cell) + 1);
    return;
  }
  // erase
  if (px >= ERASE.x && px < ERASE.x + ERASE.w && py >= ERASE.y && py < ERASE.y + ERASE.h) {
    setDigit(0);
  }
});

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById('overlay-win').classList.add('hidden');
  document.getElementById(id).classList.remove('hidden');
}
function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  PUZZLES.forEach((p, i) => {
    const locked = i + 1 > progress.unlocked;
    const btn = document.createElement('button');
    btn.className = 'level-cell' + (locked ? ' locked' : '');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : progress.done[i] ? '✓' : '·') + '</span><em>' +
      p.name[lang === 'en' ? 0 : 1] + '</em>';
    if (!locked) btn.onclick = () => startLevel(i);
    grid.appendChild(btn);
  });
}
function startLevel(index) {
  buildLevel(index);
  showScreen('screen-game');
}

function refreshText() {
  document.getElementById('title-h').textContent = t('title');
  document.getElementById('title-tag').textContent = t('tagline');
  document.getElementById('title-howto').textContent = t('howto');
  document.getElementById('btn-play').textContent = t('play');
  document.getElementById('levels-h').textContent = t('pick');
  document.getElementById('btn-levels-back').textContent = t('menu');
  document.getElementById('btn-restart').textContent = t('restart');
  document.getElementById('btn-game-menu').textContent = t('menu');
  document.getElementById('btn-next').textContent = t('next');
  document.getElementById('btn-win-menu').textContent = t('menu');
}

document.getElementById('btn-play').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-restart').onclick = () => startLevel(state.index);
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, PUZZLE_COUNT - 1));
document.getElementById('btn-win-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  refreshText();
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

refreshText();
showScreen('screen-title');
requestAnimationFrame(loop);
