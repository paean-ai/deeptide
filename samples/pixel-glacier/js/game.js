// Pixel Glacier - sliding play, animation, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-glacier-save';
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

const SLIDE_SPEED = 16;   // cells per second
let state = null;   // { index, pz, geom, pos, moves, won, anim }
let clock = 0;

function buildLevel(index) {
  const pz = buildPuzzle(LEVELS[index]);
  state = {
    index, pz, geom: boardGeom(pz.n),
    pos: pz.start, moves: 0, won: false, anim: null,
  };
  updateHud();
}

function trySlide(d) {
  if (!state || state.won || state.anim) return;
  const dest = slideFrom(state.pz.grid, state.pz.n, state.pos, d);
  if (dest === state.pos) return;   // blocked, no move
  state.moves++;
  const fr = state.pos, to = dest;
  state.anim = {
    fromR: (fr / state.pz.n) | 0, fromC: fr % state.pz.n,
    toR: (to / state.pz.n) | 0, toC: to % state.pz.n,
    r: (fr / state.pz.n) | 0, c: fr % state.pz.n, t: 0, dest,
  };
  updateHud();
}

function stepAnim(dt) {
  const a = state.anim;
  if (!a) return;
  const dist = Math.abs(a.toR - a.fromR) + Math.abs(a.toC - a.fromC);
  a.t += dt * SLIDE_SPEED / Math.max(1, dist);
  if (a.t >= 1) {
    a.r = a.toR; a.c = a.toC;
    state.pos = a.dest;
    state.anim = null;
    if (state.pos === state.pz.exit) winLevel();
  } else {
    a.r = a.fromR + (a.toR - a.fromR) * a.t;
    a.c = a.fromC + (a.toC - a.fromC) * a.t;
  }
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
  }, 350);
}

// ---- render --------------------------------------------------------------
let last = 0;
function loop(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  clock += dt;
  if (state && state.anim) stepAnim(dt);
  drawBackground(ctx);
  if (state) drawBoard(ctx, state.pz, state.geom, state.anim, clock);
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-moves').textContent = t('moves') + ' ' + state.moves;
  document.getElementById('hud-par').textContent = t('par') + ' ' + state.pz.par;
}

// ---- input: tap a direction relative to the player -----------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won || state.anim) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const { cell, gx, gy } = state.geom;
  const pr = (state.pos / state.pz.n) | 0, pc = state.pos % state.pz.n;
  const cx = gx + pc * cell + cell / 2, cy = gy + pr * cell + cell / 2;
  const dx = px - cx, dy = py - cy;
  if (Math.abs(dx) < 4 && Math.abs(dy) < 4) return;
  if (Math.abs(dx) > Math.abs(dy)) trySlide(dx > 0 ? 1 : 3);
  else trySlide(dy > 0 ? 2 : 0);
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
