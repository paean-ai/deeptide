// Pixel Burrow - real-time loop, input, screens, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-burrow-save';
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
let pops = [];        // floating score popups
let resultShown = false;

function startLevel(index) {
  state = buildGame(index);
  pops = [];
  resultShown = false;
  updateHud();
}

function popText(i, txt, color) {
  const r = burrowRect(i);
  pops.push({ x: r.x + r.s / 2, y: r.y + r.s * 0.42, txt, color, life: 0.8 });
}

// ---- loop ----------------------------------------------------------------
let last = 0;
function loop(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  const playing = state && !document.getElementById('screen-game').classList.contains('hidden');
  if (playing && state && !state.over) {
    tick(state, dt);
    updateHud();
    if (state.over) showResult();
  }
  for (const p of pops) { p.life -= dt; p.y -= dt * 30; }
  pops = pops.filter(p => p.life > 0);
  render();
  requestAnimationFrame(loop);
}

function render() {
  drawBackground(ctx, state ? state.flash : 0);
  if (!state) return;
  drawBurrows(ctx, state);
  for (const p of pops) {
    ctx.globalAlpha = Math.min(1, p.life * 2);
    ctx.fillStyle = p.color;
    ctx.font = 'bold 18px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(p.txt, p.x, p.y);
    ctx.globalAlpha = 1;
  }
  // combo badge
  if (state.combo > 1 && !state.over) {
    ctx.fillStyle = '#ffd23e';
    ctx.font = 'bold 15px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t('combo') + ' x' + state.combo, VW / 2, 444);
  }
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-score').textContent =
    state.score + '/' + state.cfg.target;
  document.getElementById('hud-time').textContent = '⏱ ' + Math.ceil(state.timeLeft);
  document.getElementById('hud-lives').textContent = '♥'.repeat(Math.max(0, state.lives));
}

function showResult() {
  if (resultShown) return;
  resultShown = true;
  const i = state.index;
  if (state.won) {
    if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  }
  if (!progress.best[i] || state.score > progress.best[i]) progress.best[i] = state.score;
  saveProgress();
  setTimeout(() => {
    const blownUp = state.lives <= 0;
    document.getElementById('res-title').textContent =
      state.won ? t('win') : blownUp ? t('bust') : t('lose');
    document.getElementById('res-line').textContent = state.won
      ? t('winLine', state.score)
      : t('loseLine', state.score, state.cfg.target);
    const nx = document.getElementById('btn-res-next');
    nx.textContent = state.won ? t('next') : t('retry');
    nx.style.display = (state.won && state.index + 1 >= LEVEL_COUNT) ? 'none' : '';
    document.getElementById('overlay-res').classList.remove('hidden');
  }, 600);
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.over) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  for (let i = 0; i < BURROWS; i++) {
    const r = burrowRect(i);
    if (px >= r.x && px < r.x + r.s && py >= r.y && py < r.y + r.s) {
      const c = state.burrows[i];
      if (!c) return;
      if (c.type === 'bomb') { bonk(state, i); popText(i, '✖', '#ff6e7a'); }
      else {
        const g = bonk(state, i);
        popText(i, '+' + g, c.type === 'golden' ? '#ffd23e' : '#fff2cf');
      }
      if (state.over) showResult();
      return;
    }
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
