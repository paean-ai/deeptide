// Pixel Shikaku - drag-to-draw-rectangle play, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-shikaku-save';
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

let state = null;
let drag = null;        // { startR, startC, curR, curC }

function buildLevel(index) {
  const pz = buildPuzzle(LEVELS[index]);
  state = {
    index, pz, geom: boardGeom(pz),
    rects: pz.clues.map(() => null),
    won: false,
  };
  drag = null;
  updateHud();
}

// turn drag start + current cell into a normalised rect {r,c,rh,rw}
function dragRect(d) {
  if (!d) return null;
  const r = Math.min(d.startR, d.curR), c = Math.min(d.startC, d.curC);
  const rh = Math.abs(d.startR - d.curR) + 1, rw = Math.abs(d.startC - d.curC) + 1;
  return { r, c, rh, rw };
}

function previewValid() {
  if (!drag) return false;
  const rec = dragRect(drag);
  return validateRect(state.pz, state.rects, rec).ok;
}

function placeOrRemove(downR, downC, upR, upC) {
  if (state.won) return;
  // tap (same cell) → if it lands on a placed rect, remove that rect
  if (downR === upR && downC === upC) {
    for (let i = 0; i < state.rects.length; i++) {
      const rec = state.rects[i];
      if (rec && upR >= rec.r && upR < rec.r + rec.rh && upC >= rec.c && upC < rec.c + rec.rw) {
        state.rects[i] = null;
        updateHud();
        return;
      }
    }
    // single-cell rect tap: still validate (a 1x1 area, e.g. clue n=1)
  }
  const rec = dragRect({ startR: downR, startC: downC, curR: upR, curC: upC });
  const v = validateRect(state.pz, state.rects, rec);
  if (!v.ok) return;
  state.rects[v.clueIdx] = rec;
  updateHud();
  const ev = evaluate(state.pz, state.rects);
  if (ev.solved) winLevel();
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
  if (state) {
    drawBoard(ctx, state.pz, state.geom, state.rects, dragRect(drag), previewValid());
  }
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  const placed = state.rects.filter(Boolean).length;
  document.getElementById('hud-placed').textContent =
    t('placed') + ' ' + placed + '/' + state.pz.clues.length;
}

// ---- input ---------------------------------------------------------------
function cellAt(px, py) {
  const { cell, gx, gy } = state.geom;
  const c = Math.floor((px - gx) / cell), r = Math.floor((py - gy) / cell);
  if (r < 0 || c < 0 || r >= state.pz.h || c >= state.pz.w) return null;
  return { r, c };
}
function localXY(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const p = localXY(e);
  const cell = cellAt(p.x, p.y);
  if (!cell) return;
  drag = { startR: cell.r, startC: cell.c, curR: cell.r, curC: cell.c };
  if (canvas.setPointerCapture) try { canvas.setPointerCapture(e.pointerId); } catch (x) { /* */ }
});
canvas.addEventListener('pointermove', e => {
  if (!drag || !state) return;
  const p = localXY(e);
  const cell = cellAt(p.x, p.y);
  if (cell) { drag.curR = cell.r; drag.curC = cell.c; }
});
function finishDrag() {
  if (!drag) return;
  const d = drag; drag = null;
  placeOrRemove(d.startR, d.startC, d.curR, d.curC);
}
canvas.addEventListener('pointerup', finishDrag);
canvas.addEventListener('pointercancel', () => { drag = null; });

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
