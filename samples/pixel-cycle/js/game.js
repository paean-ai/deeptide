// Pixel Cycle - real-time loop, on-screen controls, screens, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-cycle-save';
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
let resultShown = false;
const pressed = { left: false, right: false };
const pointerOwners = {};

function startLevel(index) {
  state = buildGame(index);
  resultShown = false;
  pressed.left = pressed.right = false;
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
    tick(state, dt);
    updateHud();
    if (state.over) showResult();
  }
  drawBackground(ctx);
  if (state) {
    drawField(ctx, state);
    drawControls(ctx, pressed);
  }
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent =
    t('level') + ' ' + (state.levelIndex + 1);
  document.getElementById('hud-name').textContent =
    LEVELS[state.levelIndex].name[lang === 'en' ? 0 : 1];
  document.getElementById('hud-score').textContent =
    state.playerWins + ' - ' + state.cpuWins;
}

function showResult() {
  if (resultShown) return;
  resultShown = true;
  const i = state.levelIndex;
  if (state.won) {
    progress.done[i] = true;
    if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
    saveProgress();
  }
  setTimeout(() => {
    document.getElementById('res-title').textContent = state.won ? t('win') : t('lose');
    document.getElementById('res-line').textContent =
      t('winLine', state.playerWins, state.cpuWins);
    const nx = document.getElementById('btn-res-next');
    nx.textContent = state.won ? t('next') : t('retry');
    nx.style.display = (state.won && state.levelIndex + 1 >= LEVEL_COUNT) ? 'none' : '';
    document.getElementById('overlay-res').classList.remove('hidden');
  }, 600);
}

// ---- input: tap LEFT / RIGHT to queue a turn ----------------------------
function pickBtn(px, py) {
  if (px >= PAD_L.x && px < PAD_L.x + PAD_L.w && py >= PAD_L.y && py < PAD_L.y + PAD_L.h) return PAD_L;
  if (px >= PAD_R.x && px < PAD_R.x + PAD_R.w && py >= PAD_R.y && py < PAD_R.y + PAD_R.h) return PAD_R;
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
  const b = pickBtn(p.x, p.y);
  if (b) {
    pressed[b.key] = true;
    pointerOwners[e.pointerId] = b.key;
    if (canvas.setPointerCapture) try { canvas.setPointerCapture(e.pointerId); } catch (x) { /* */ }
    turnPlayer(state, b.key);   // queue the turn for the next tick
  }
});
function release(e) {
  const k = pointerOwners[e.pointerId];
  if (k) { pressed[k] = false; delete pointerOwners[e.pointerId]; }
}
canvas.addEventListener('pointerup', release);
canvas.addEventListener('pointercancel', release);

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
      (locked ? '🔒' : progress.done[i] ? '✓' : '·') + '</span><em>' +
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
