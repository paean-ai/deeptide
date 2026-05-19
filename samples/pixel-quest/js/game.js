// Pixel Quest - battle UI, turn pacing, screens, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-quest-save';
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
let mode = 'busy';     // menu | enemy | ally | busy
let clock = 0;
let resultShown = false;

function startBattle(index) {
  state = buildBattle(index);
  mode = 'busy';
  resultShown = false;
  updateHud();
  tick();
}

// advance the turn pump: auto-run enemy turns, stop for hero input
function tick() {
  if (!state) return;
  if (state.over) { showResult(); return; }
  const u = currentUnit(state);
  if (!u) return;
  updateHud();
  if (u.side === 'enemy') {
    mode = 'busy';
    setTimeout(() => {
      if (!state || state.over) { if (state) showResult(); return; }
      enemyAct(state);
      tick();
    }, 640);
  } else {
    mode = 'menu';
  }
}

function heroDidAct() {
  mode = 'busy';
  updateHud();
  tick();
}

function showResult() {
  if (resultShown) return;
  resultShown = true;
  if (state.won) {
    const i = state.index;
    progress.done[i] = true;
    if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
    saveProgress();
  }
  setTimeout(() => {
    document.getElementById('res-title').textContent = state.won ? t('win') : t('lose');
    document.getElementById('res-line').textContent = state.won ? t('winLine') : t('loseLine');
    const nx = document.getElementById('btn-res-next');
    nx.textContent = state.won ? t('next') : t('retry');
    nx.style.display = (state.won && state.index + 1 >= LEVEL_COUNT) ? 'none' : '';
    document.getElementById('overlay-res').classList.remove('hidden');
  }, 700);
}

// ---- render --------------------------------------------------------------
function uiState() {
  const u = state ? currentUnit(state) : null;
  let pickable = null;
  if (mode === 'enemy') pickable = state.enemies.filter(e => e.hp > 0);
  else if (mode === 'ally') pickable = state.heroes.filter(h => h.hp > 0);
  return {
    current: u, mode, pulse: clock,
    banner: u ? t('turnOf', u.name[lang === 'en' ? 0 : 1]) : '',
    pickable,
  };
}
function loop() {
  clock += 0.04;
  drawBackground(ctx);
  if (state) drawBattle(ctx, state, uiState());
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent =
    t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-name').textContent = LEVELS[state.index].name[lang === 'en' ? 0 : 1];
}

// ---- input ---------------------------------------------------------------
function hitUnit(s, px, py, list) {
  for (const u of list) {
    if (u.hp <= 0) continue;
    const r = unitRect(s, u);
    if (px >= r.x - 4 && px <= r.x + r.w + 4 && py >= r.y - 4 && py <= r.y + r.h + 8) return u;
  }
  return null;
}

canvas.addEventListener('pointerdown', e => {
  if (!state || state.over || mode === 'busy') return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const u = currentUnit(state);
  if (!u || u.side !== 'hero') return;

  if (mode === 'menu') {
    for (const b of MENU_BTN) {
      if (px >= b.x && px <= b.x + b.w && py >= b.y && py <= b.y + b.h) {
        if (b.key === 'attack') { mode = 'enemy'; }
        else if (b.key === 'defend') { heroDefend(state); heroDidAct(); }
        else if (b.key === 'skill') {
          if (!canCast(u)) return;
          const kind = SKILLS[u.skill].kind;
          if (kind === 'heal') mode = 'ally';
          else { heroSkill(state, null); heroDidAct(); }
        }
        return;
      }
    }
    return;
  }
  if (mode === 'enemy') {
    const tgt = hitUnit(state, px, py, state.enemies);
    if (tgt) { heroAttack(state, tgt); heroDidAct(); }
    else mode = 'menu';
    return;
  }
  if (mode === 'ally') {
    const tgt = hitUnit(state, px, py, state.heroes);
    if (tgt) { heroSkill(state, tgt); heroDidAct(); }
    else mode = 'menu';
  }
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
  startBattle(index);
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
document.getElementById('btn-restart').onclick = () => startBattle(state.index);
document.getElementById('btn-res-next').onclick = () => {
  if (state.won) enterLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
  else startBattle(state.index);
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
