// Pixel Water Sort - pour logic, undo, win detection, progress save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-water-sort-save';
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

const UNIT_H = 24;
let state = null;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const lv = LEVELS[index];
  const tubes = genPuzzle(lv.colors, lv.seed);
  const n = tubes.length;
  const perRow = n <= 5 ? n : Math.ceil(n / 2);
  const rows = Math.ceil(n / perRow);
  const gap = 12;
  const tubeW = Math.min(48, Math.floor((342 - (perRow - 1) * gap) / perRow));
  const tubeH = UNIT_H * TUBE_CAP;
  const rowH = tubeH + 40;
  const layout = [];
  for (let i = 0; i < n; i++) {
    const row = Math.floor(i / perRow);
    const inRow = Math.min(perRow, n - row * perRow);
    const idxInRow = i - row * perRow;
    const rowW = inRow * tubeW + (inRow - 1) * gap;
    const x = Math.round((VW - rowW) / 2) + idxInRow * (tubeW + gap);
    const y = Math.round(92 + (336 - rows * rowH) / 2) + row * rowH + 16;
    layout.push({ x, y });
  }
  state = {
    index, tubes, layout, tubeW, tubeH,
    selected: -1, moves: 0, undos: 0, history: [], won: false,
  };
  updateHud();
}

function snapshot() {
  state.history.push(JSON.stringify({ tubes: state.tubes, moves: state.moves }));
  if (state.history.length > 250) state.history.shift();
}

// ---- actions -------------------------------------------------------------
function tapTube(idx) {
  if (state.won) return;
  if (state.selected === -1) {
    if (state.tubes[idx].length) state.selected = idx;
    return;
  }
  if (state.selected === idx) { state.selected = -1; return; }
  if (canPour(state.tubes[state.selected], state.tubes[idx])) {
    snapshot();
    state.tubes = doPour(state.tubes, state.selected, idx);
    state.moves++;
    state.selected = -1;
    updateHud();
    if (isSolved(state.tubes)) winLevel();
  } else {
    state.selected = state.tubes[idx].length ? idx : -1;
  }
}

function undo() {
  if (!state || state.won || !state.history.length) return;
  const s = JSON.parse(state.history.pop());
  state.tubes = s.tubes;
  state.moves = s.moves;
  state.undos++;
  state.selected = -1;
  updateHud();
}

function winLevel() {
  state.won = true;
  const stars = state.undos === 0 ? 3 : state.undos <= 3 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', state.moves, state.undos);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 500);
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  const selTop = state.selected >= 0 ? topRun(state.tubes[state.selected]) : null;
  for (let i = 0; i < state.tubes.length; i++) {
    const p = state.layout[i];
    const pourHi = selTop != null && i !== state.selected &&
      canPour(state.tubes[state.selected], state.tubes[i]);
    drawTube(ctx, p.x, p.y, state.tubeW, UNIT_H, state.tubes[i],
      i === state.selected, pourHi);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-moves').textContent = 'MOVES ' + state.moves;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  for (let i = 0; i < state.tubes.length; i++) {
    const p = state.layout[i];
    if (px >= p.x - 4 && px <= p.x + state.tubeW + 4 &&
        py >= p.y - 18 && py <= p.y + state.tubeH + 10) {
      tapTube(i);
      return;
    }
  }
  state.selected = -1;
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
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
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
document.getElementById('btn-undo').onclick = undo;
document.getElementById('btn-restart').onclick = () => startLevel(state.index);
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

// ---- main loop -----------------------------------------------------------
function loop() {
  if (!document.getElementById('screen-game').classList.contains('hidden')) render();
  else drawBackground(ctx);
  requestAnimationFrame(loop);
}
showScreen('screen-title');
requestAnimationFrame(loop);
