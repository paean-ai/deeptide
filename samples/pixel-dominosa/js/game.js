// Pixel Dominosa - puzzle play, domino placement, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-dominosa-save';
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

let state = null;   // { index, pz, geom, dominoes, edges, ev, won }

function buildLevel(index) {
  const pz = buildPuzzle(LEVELS[index]);
  const geom = boardGeom(pz);
  // candidate edges: every orthogonally-adjacent cell pair
  const edges = [];
  for (let i = 0; i < pz.pips.length; i++) {
    const r = (i / pz.cols) | 0, c = i % pz.cols;
    if (c + 1 < pz.cols) edges.push([i, i + 1]);
    if (r + 1 < pz.rows) edges.push([i, i + pz.cols]);
  }
  state = { index, pz, geom, edges, dominoes: [], won: false };
  reEval();
  updateHud();
}

function reEval() {
  state.ev = evaluate(state.pz, state.dominoes);
}

function placeDomino(a, b) {
  if (state.won) return;
  // already exactly this domino? remove it
  const exact = state.dominoes.findIndex(d =>
    (d[0] === a && d[1] === b) || (d[0] === b && d[1] === a));
  if (exact >= 0) { state.dominoes.splice(exact, 1); reEval(); updateHud(); return; }
  // drop any domino touching either cell, then add the new one
  state.dominoes = state.dominoes.filter(d =>
    d[0] !== a && d[1] !== a && d[0] !== b && d[1] !== b);
  state.dominoes.push([a, b]);
  reEval();
  updateHud();
  if (state.ev.solved) winLevel();
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
  }, 400);
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (state) drawBoard(ctx, state.pz, state.geom, state.ev, state.dominoes);
}
function loop() {
  render();
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  const left = state.ev.covered.filter(v => !v).length;
  document.getElementById('hud-left').textContent = t('left') + ' ' + left;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const { cell } = state.geom;
  let best = -1, bestD = cell * 0.6;
  state.edges.forEach((ed, i) => {
    const a = cellXY(state.geom, ed[0], state.pz.cols);
    const b = cellXY(state.geom, ed[1], state.pz.cols);
    const mx = (a.x + b.x) / 2 + cell / 2, my = (a.y + b.y) / 2 + cell / 2;
    const d = Math.hypot(px - mx, py - my);
    if (d < bestD) { bestD = d; best = i; }
  });
  if (best >= 0) placeDomino(state.edges[best][0], state.edges[best][1]);
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
