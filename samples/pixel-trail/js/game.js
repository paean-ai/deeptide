// Pixel Trail - puzzle play, path drawing, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-trail-save';
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

let state = null;   // { index, pz, geom, pathSoFar, clueOfNum, won }

function buildLevel(index) {
  const pz = buildPuzzle(LEVELS[index]);
  const clueOfNum = {};
  for (const cell in pz.revealed) clueOfNum[pz.revealed[cell]] = +cell;
  state = {
    index, pz, geom: boardGeom(pz.n),
    pathSoFar: [clueOfNum[1]],
    clueOfNum, won: false,
  };
  updateHud();
}

function isAdjacent(a, b, n) {
  return neighbors(n, a).includes(b);
}

function onTapCell(cell) {
  if (state.won) return;
  const path = state.pathSoFar;
  const idxOnPath = path.indexOf(cell);
  if (idxOnPath >= 0) {
    // undo back to and including this cell? No - undo to keep this cell as the new tip
    if (idxOnPath === path.length - 1) return; // already the tip
    state.pathSoFar = path.slice(0, idxOnPath + 1);
    updateHud();
    return;
  }
  // try to extend
  const tip = path[path.length - 1];
  if (!isAdjacent(cell, tip, state.pz.n)) return;
  const nextNum = path.length + 1;
  const must = state.clueOfNum[nextNum];
  if (must !== undefined && cell !== must) return;
  if (must === undefined && state.pz.revealed[cell] !== undefined) return;
  state.pathSoFar.push(cell);
  updateHud();
  if (state.pathSoFar.length === state.pz.N) winLevel();
}

function winLevel() {
  state.won = true;
  const i = state.index;
  progress.done[i] = true;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = t('win');
    document.getElementById('win-line').textContent = t('winLine');
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    document.getElementById('overlay-win').classList.remove('hidden');
  }, 350);
}

// ---- render --------------------------------------------------------------
function loop() {
  drawBackground(ctx);
  if (state) drawBoard(ctx, state.pz, state.geom, state.pathSoFar);
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-step').textContent =
    t('step') + ' ' + Math.min(state.pathSoFar.length + 1, state.pz.N) + '/' + state.pz.N;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const { cell, gx, gy } = state.geom;
  const c = Math.floor((px - gx) / cell), r = Math.floor((py - gy) / cell);
  if (r < 0 || c < 0 || r >= state.pz.n || c >= state.pz.n) return;
  onTapCell(r * state.pz.n + c);
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
  LEVELS.forEach((p, i) => {
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
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
document.getElementById('btn-win-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  refreshText();
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

refreshText();
showScreen('screen-title');
requestAnimationFrame(loop);
