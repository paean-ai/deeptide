// Pixel Codebreaker - guess building, feedback display, win/lose, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-codebreaker-save';
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

const ROWH = 24, PEG_R = 8, PEG_STEP = 22, BOARD_X = 36;
const SECRET_Y = 48, BOARD_Y = 74, SWATCH = 34;
let state = null;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const level = LEVELS[index];
  state = {
    index, level,
    code: makeCode(level),
    guesses: [],
    current: new Array(level.len).fill(-1),
    won: false, lost: false, winStars: 0,
    paletteY: BOARD_Y + level.attempts * ROWH + 14,
  };
  updateHud();
  updateControls();
}

function feedbackX() { return BOARD_X + state.level.len * PEG_STEP + 14; }

// ---- actions -------------------------------------------------------------
function fillColor(idx) {
  if (state.won || state.lost) return;
  for (let i = 0; i < state.level.len; i++) {
    if (state.current[i] === -1) { state.current[i] = idx; break; }
  }
  updateControls();
}
function clearSlot(i) {
  if (state.won || state.lost) return;
  if (i >= 0 && i < state.level.len) state.current[i] = -1;
  updateControls();
}
function clearGuess() {
  if (state.won || state.lost) return;
  state.current.fill(-1);
  updateControls();
}
function submitGuess() {
  if (state.won || state.lost) return;
  if (state.current.some(c => c === -1)) return;
  const fb = feedback(state.current, state.code);
  state.guesses.push({ pegs: state.current.slice(), black: fb.black, white: fb.white });
  if (fb.black === state.level.len) {
    winLevel();
  } else {
    state.current = new Array(state.level.len).fill(-1);
    if (state.guesses.length >= state.level.attempts) loseLevel();
  }
  updateHud();
  updateControls();
}

function winLevel() {
  state.won = true;
  const used = state.guesses.length, att = state.level.attempts;
  const stars = used <= Math.ceil(att * 0.5) ? 3 : used <= Math.ceil(att * 0.8) ? 2 : 1;
  state.winStars = stars;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', used);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 550);
}
function loseLevel() {
  state.lost = true;
  setTimeout(() => {
    document.getElementById('lose-line').textContent = t('loseLine');
    showOverlay('overlay-lose');
  }, 550);
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  const len = state.level.len;
  // secret code row — hidden until the game ends
  ctx.fillStyle = '#cfc6e4';
  ctx.font = '900 11px ui-monospace, monospace';
  ctx.textAlign = 'left';
  ctx.fillText('CODE', 8, SECRET_Y + 4);
  for (let i = 0; i < len; i++) {
    const cx = BOARD_X + i * PEG_STEP + PEG_R;
    if (state.won || state.lost) {
      drawPeg(ctx, cx, SECRET_Y, PEG_R, state.code[i]);
    } else {
      ctx.fillStyle = '#2a2046';
      ctx.beginPath();
      ctx.arc(cx, SECRET_Y, PEG_R, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#7a6ea0';
      ctx.fillText('?', cx - 3, SECRET_Y + 4);
    }
  }
  // guess rows
  for (let r = 0; r < state.level.attempts; r++) {
    const y = BOARD_Y + r * ROWH + ROWH / 2;
    const isCurrent = r === state.guesses.length && !state.won && !state.lost;
    if (isCurrent) {
      ctx.fillStyle = 'rgba(180,139,255,0.12)';
      ctx.fillRect(BOARD_X - 6, y - ROWH / 2 + 1, VW - BOARD_X, ROWH - 2);
    }
    const pegs = r < state.guesses.length ? state.guesses[r].pegs
      : (isCurrent ? state.current : null);
    for (let i = 0; i < len; i++) {
      const cx = BOARD_X + i * PEG_STEP + PEG_R;
      drawPeg(ctx, cx, y, PEG_R, pegs ? pegs[i] : -1);
    }
    if (r < state.guesses.length) {
      drawFeedback(ctx, feedbackX(), y - 5, len, state.guesses[r].black, state.guesses[r].white);
    }
  }
  // colour palette
  const k = state.level.colors;
  const px0 = (VW - k * SWATCH) / 2;
  for (let i = 0; i < k; i++) {
    drawPeg(ctx, px0 + i * SWATCH + SWATCH / 2, state.paletteY + 14, 13, i);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-tries').textContent =
    t('tries') + ' ' + state.guesses.length + '/' + state.level.attempts;
}
function updateControls() {
  if (!state) return;
  const full = !state.current.some(c => c === -1);
  document.getElementById('btn-submit').disabled = !full || state.won || state.lost;
  document.getElementById('btn-clear').disabled =
    state.current.every(c => c === -1) || state.won || state.lost;
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.won || state.lost) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  // palette
  const k = state.level.colors, px0 = (VW - k * SWATCH) / 2;
  if (py >= state.paletteY && py <= state.paletteY + 30) {
    const idx = Math.floor((px - px0) / SWATCH);
    if (idx >= 0 && idx < k) fillColor(idx);
    return;
  }
  // current guess row — tap a peg to clear it
  const curRow = state.guesses.length;
  const rowY = BOARD_Y + curRow * ROWH;
  if (py >= rowY && py < rowY + ROWH) {
    const si = Math.floor((px - BOARD_X) / PEG_STEP);
    if (si >= 0 && si < state.level.len) clearSlot(si);
  }
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
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) +
      '</span><em>' + lv.len + '×' + lv.colors + '</em>';
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
document.getElementById('btn-submit').onclick = submitGuess;
document.getElementById('btn-clear').onclick = clearGuess;
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-lose-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-lose-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

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
