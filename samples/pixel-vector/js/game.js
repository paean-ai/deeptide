// Pixel Vector - real-time loop, on-screen controls, screens, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-vector-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, best: d.best || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, best: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

let state = null;
let resultShown = false;
const pressed = { left: false, right: false, thrust: false, fire: false };
const pointerOwners = {};   // pointerId -> button key (or null for field)

function startLevel(index) {
  state = buildGame(index);
  resultShown = false;
  for (const k in pressed) pressed[k] = false;
  updateHud();
}

// ---- loop ----------------------------------------------------------------
let last = 0;
function loop(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  const playing = state && !state.over &&
    !document.getElementById('screen-game').classList.contains('hidden');
  if (playing) {
    tick(state, dt, pressed);
    updateHud();
    if (state.over) showResult();
  }
  drawBackground(ctx);
  if (state) {
    drawWorld(ctx, state, pressed);
    drawControls(ctx, pressed);
  }
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent =
    t('level') + ' ' + (state.levelIndex + 1);
  document.getElementById('hud-score').textContent = t('score') + ' ' + state.score;
  document.getElementById('hud-lives').textContent = '⏳'.repeat(Math.max(0, state.lives));
}

function showResult() {
  if (resultShown) return;
  resultShown = true;
  const i = state.levelIndex;
  if (state.won) {
    if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  }
  if (!progress.best[i] || state.score > progress.best[i]) progress.best[i] = state.score;
  saveProgress();
  setTimeout(() => {
    document.getElementById('res-title').textContent = state.won ? t('win') : t('lose');
    document.getElementById('res-line').textContent =
      state.won ? t('winLine', state.score) : t('loseLine');
    const nx = document.getElementById('btn-res-next');
    nx.textContent = state.won ? t('next') : t('retry');
    nx.style.display = (state.won && state.levelIndex + 1 >= LEVEL_COUNT) ? 'none' : '';
    document.getElementById('overlay-res').classList.remove('hidden');
  }, 600);
}

// ---- input ---------------------------------------------------------------
function pickBtn(px, py) {
  for (const k in BTN) {
    const b = BTN[k];
    if (px >= b.x && px < b.x + b.w && py >= b.y && py < b.y + b.h) return k;
  }
  return null;
}
function localXY(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
canvas.addEventListener('pointerdown', e => {
  if (!state) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const p = localXY(e);
  const k = pickBtn(p.x, p.y);
  if (k) {
    pressed[k] = true;
    pointerOwners[e.pointerId] = k;
    if (canvas.setPointerCapture) try { canvas.setPointerCapture(e.pointerId); } catch (x) { /* */ }
  }
});
function releasePointer(e) {
  const k = pointerOwners[e.pointerId];
  if (k) { pressed[k] = false; delete pointerOwners[e.pointerId]; }
}
canvas.addEventListener('pointerup', releasePointer);
canvas.addEventListener('pointercancel', releasePointer);
canvas.addEventListener('pointerleave', releasePointer);

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById('overlay-res').classList.add('hidden');
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
      (locked ? '🔒' : (progress.best[i] ? progress.best[i] : '·')) + '</span><em>' +
      p.name[lang === 'en' ? 0 : 1] + '</em>';
    if (!locked) btn.onclick = () => enterLevel(i);
    grid.appendChild(btn);
  });
}
function enterLevel(index) {
  startLevel(index);
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
  document.getElementById('btn-res-menu').textContent = t('menu');
}

document.getElementById('btn-play').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-restart').onclick = () => startLevel(state.levelIndex);
document.getElementById('btn-res-next').onclick = () => {
  if (state.won) enterLevel(Math.min(state.levelIndex + 1, LEVEL_COUNT - 1));
  else startLevel(state.levelIndex);
  document.getElementById('overlay-res').classList.add('hidden');
};
document.getElementById('btn-res-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  refreshText();
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

refreshText();
showScreen('screen-title');
requestAnimationFrame(loop);
