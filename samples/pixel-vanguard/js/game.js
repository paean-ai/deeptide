// Pixel Vanguard - turn UI, screens, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-vanguard-save';
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

let state = null;     // combat state
let geom = null;
let selected = null;  // selected hero id
let overShown = false;

const ENDBTN = { x: 110, y: 408, w: 140, h: 40 };

function startLevel(index) {
  state = buildState(index);
  geom = boardGeom(state);
  selected = null;
  overShown = false;
  updateHud();
}

// ---- ui state ------------------------------------------------------------
function selectedHero() {
  return state ? state.heroes.find(h => h.id === selected) : null;
}
function uiOverlays() {
  const h = selectedHero();
  if (!h || state.turn !== 'player') return {};
  const reach = h.moved ? null : reachable(state, h);
  const targets = attackTargets(state, h).map(e => e.id);
  return { selected: h.id, reach, targets };
}

function onTileTap(r, c) {
  if (state.over || state.turn !== 'player') return;
  const hero = heroAt(state, r, c);
  const h = selectedHero();
  // tap a hero -> select / deselect
  if (hero) {
    if (hero.id === selected) selected = null;
    else if (!hero.acted) selected = hero.id;
    return;
  }
  if (!h) return;
  // tap an enemy adjacent to the selected hero -> attack
  const enemy = enemyAt(state, r, c);
  if (enemy && attackTargets(state, h).some(e => e.id === enemy.id)) {
    heroAttack(state, h, enemy);
    selected = null;
    updateHud();
    if (state.over) showResult();
    return;
  }
  // tap a reachable tile -> move
  if (!h.moved) {
    const reach = reachable(state, h);
    if ((r + ',' + c) in reach) {
      moveHero(state, h, r, c);
      // if no targets now, keep selected for clarity; stays usable
    }
  }
}

function doEndTurn() {
  if (!state || state.over || state.turn !== 'player') return;
  selected = null;
  endPlayerTurn(state);
  updateHud();
  if (state.over) showResult();
}

function showResult() {
  if (overShown) return;
  overShown = true;
  if (state.won) {
    const i = state.index;
    progress.done[i] = true;
    if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
    saveProgress();
  }
  setTimeout(() => {
    document.getElementById('res-title').textContent = state.won ? t('win') : t('lose');
    document.getElementById('res-line').textContent = state.won ? t('winLine') : t('loseLine');
    const nextBtn = document.getElementById('btn-res-next');
    nextBtn.textContent = state.won ? t('next') : t('retry');
    nextBtn.style.display = (state.won && state.index + 1 >= LEVEL_COUNT) ? 'none' : '';
    document.getElementById('overlay-res').classList.remove('hidden');
  }, 450);
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (state) {
    drawBoard(ctx, state, geom, uiOverlays());
    drawEndButton(ctx);
  }
}
function drawEndButton(ctx) {
  const enabled = !state.over && state.turn === 'player';
  ctx.fillStyle = enabled ? '#f2c14e' : '#3a4256';
  ctx.fillRect(ENDBTN.x, ENDBTN.y, ENDBTN.w, ENDBTN.h);
  ctx.fillStyle = enabled ? '#0b0e18' : '#7a8298';
  ctx.font = 'bold 14px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(t('endTurn'), ENDBTN.x + ENDBTN.w / 2, ENDBTN.y + ENDBTN.h / 2 + 1);
}
function loop() {
  render();
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-core').textContent = t('core') + ' ' + Math.max(0, state.core) + '/' + state.coreMax;
  document.getElementById('hud-foes').textContent = '☠ ' + state.enemies.length;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  if (px >= ENDBTN.x && px < ENDBTN.x + ENDBTN.w && py >= ENDBTN.y && py < ENDBTN.y + ENDBTN.h) {
    doEndTurn();
    return;
  }
  const c = Math.floor((px - geom.gx) / geom.cell);
  const r = Math.floor((py - geom.gy) / geom.cell);
  if (r >= 0 && c >= 0 && r < state.h && c < state.w) onTileTap(r, c);
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
document.getElementById('btn-restart').onclick = () => startLevel(state.index);
document.getElementById('btn-res-next').onclick = () => {
  if (state.won) enterLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
  else startLevel(state.index);
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
