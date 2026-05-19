// Pixel Circuit - rotation play, animation, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-circuit-save';
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

let state = null;
const ROT_DUR = 0.13;

function buildLevel(index) {
  const pz = buildPuzzle(LEVELS[index]);
  state = {
    index, pz, geom: boardGeom(pz.n),
    moves: 0, won: false, anim: null,
    ev: evaluate(pz),
  };
  updateHud();
}

function tapTile(i) {
  if (!state || state.won || state.anim) return;
  state.anim = { idx: i, t: 0 };
  state.moves++;
  updateHud();
}

function finishAnim() {
  if (!state.anim) return;
  rotateCell(state.pz, state.anim.idx);
  state.anim = null;
  state.ev = evaluate(state.pz);
  if (state.ev.won) winLevel();
}

function winLevel() {
  state.won = true;
  const par = state.pz.par;
  const stars = state.moves <= par ? 3 : state.moves <= Math.round(par * 1.5) ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', state.moves, par);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    document.getElementById('overlay-win').classList.remove('hidden');
  }, 450);
}

// ---- render --------------------------------------------------------------
let last = 0;
function loop(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  if (state && state.anim) {
    state.anim.t += dt / ROT_DUR;
    if (state.anim.t >= 1) finishAnim();
  }
  drawBackground(ctx);
  if (state) drawBoard(ctx, state.pz, state.geom, state.ev, state.anim);
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-moves').textContent = t('moves') + ' ' + state.moves;
  document.getElementById('hud-par').textContent = t('par') + ' ' + state.pz.par;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won || state.anim) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const { cell, gx, gy } = state.geom;
  const c = Math.floor((px - gx) / cell), r = Math.floor((py - gy) / cell);
  if (r < 0 || c < 0 || r >= state.pz.n || c >= state.pz.n) return;
  tapTile(r * state.pz.n + c);
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
    const st = progress.stars[i] || 0;
    const btn = document.createElement('button');
    btn.className = 'level-cell' + (locked ? ' locked' : '');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) + '</span><em>' +
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
