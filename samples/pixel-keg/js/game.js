// Pixel Keg - real-time loop, on-screen controls, screens, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-keg-save';
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
let geom = null;
let resultShown = false;
let clock = 0;
const STEP_INTERVAL = 0.16;
const pressed = { up: false, down: false, left: false, right: false };
const stepCooldown = { up: 0, down: 0, left: 0, right: 0 };
let bombPressed = false;
let flash = 0;

function startLevel(index) {
  state = buildGame(index);
  geom = boardGeom();
  resultShown = false;
  flash = 0;
  for (const k in pressed) { pressed[k] = false; stepCooldown[k] = 0; }
  updateHud();
}

// ---- loop ----------------------------------------------------------------
let last = 0;
function loop(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  clock += dt;
  if (flash > 0) flash = Math.max(0, flash - dt * 2);
  if (state && !state.over &&
      !document.getElementById('screen-game').classList.contains('hidden')) {
    for (const k in pressed) {
      stepCooldown[k] -= dt;
      if (pressed[k] && stepCooldown[k] <= 0) {
        if (movePlayer(state, PAD[k].dir)) flash = 0.05;
        stepCooldown[k] = STEP_INTERVAL;
      }
    }
    const livesBefore = state.lives;
    tick(state, dt);
    if (state.lives < livesBefore) flash = 0.4;
    updateHud();
    if (state.over) showResult();
  }
  drawBackground(ctx, flash);
  if (state) {
    drawBoard(ctx, state, geom, clock);
    drawControls(ctx, pressed, bombPressed);
  }
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent =
    t('level') + ' ' + (state.levelIndex + 1);
  document.getElementById('hud-lives').textContent = '♥'.repeat(Math.max(0, state.lives));
  document.getElementById('hud-name').textContent =
    LEVELS[state.levelIndex].name[lang === 'en' ? 0 : 1];
}

function showResult() {
  if (resultShown) return;
  resultShown = true;
  if (state.won) {
    const i = state.levelIndex;
    progress.done[i] = true;
    if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
    saveProgress();
  }
  setTimeout(() => {
    document.getElementById('res-title').textContent = state.won ? t('win') : t('lose');
    document.getElementById('res-line').textContent =
      state.won ? t('winLine', state.levelIndex) : t('loseLine');
    const nx = document.getElementById('btn-res-next');
    nx.textContent = state.won ? t('next') : t('retry');
    nx.style.display = (state.won && state.levelIndex + 1 >= LEVEL_COUNT) ? 'none' : '';
    document.getElementById('overlay-res').classList.remove('hidden');
  }, 500);
}

// ---- input ---------------------------------------------------------------
function inRect(px, py, b) {
  return px >= b.x && px < b.x + b.w && py >= b.y && py < b.y + b.h;
}
function localXY(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * VW / rect.width,
    y: (e.clientY - rect.top) * VH / rect.height,
  };
}
canvas.addEventListener('pointerdown', e => {
  if (!state || state.over) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const p = localXY(e);
  for (const k in PAD) {
    if (inRect(p.x, p.y, PAD[k])) {
      pressed[k] = true;
      if (movePlayer(state, PAD[k].dir)) flash = 0.05;
      stepCooldown[k] = STEP_INTERVAL;
      return;
    }
  }
  if (inRect(p.x, p.y, BOMB_BTN)) {
    bombPressed = true;
    placeBomb(state);
  }
});
canvas.addEventListener('pointerup', () => {
  for (const k in pressed) pressed[k] = false;
  bombPressed = false;
});
canvas.addEventListener('pointercancel', () => {
  for (const k in pressed) pressed[k] = false;
  bombPressed = false;
});

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
