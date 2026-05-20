// Pixel Hanoi - screen flow, tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-hanoi:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;

const save = loadSave();
function loadSave() {
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return JSON.parse(raw); } catch (_) {}
  return { cleared: [], best: {} };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function unlocked(i) { return i === 0 || save.cleared.includes(i - 1); }
function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startLevel(i) {
  state = buildGame(i);
  screen = 'play';
  paint();
}

function afterMove() {
  if (state.solved) {
    const key = String(state.levelIndex);
    if (!save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
    if (!save.best[key] || save.best[key] > state.moves) save.best[key] = state.moves;
    persist();
    screen = 'result';
  }
}

// ---- input ------------------------------------------------------------
function relPoint(e) {
  const r = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) * (CANVAS_W / r.width),
    y: (e.clientY - r.top)  * (CANVAS_H / r.height),
  };
}
function hit(p, r) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { setScreen('levels'); return; }
    if (hit(p, langRect()))       { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'levels') {
    if (hit(p, backRect())) { setScreen('title'); return; }
    const idx = hitLevelCell(p);
    if (idx >= 0 && unlocked(idx)) startLevel(idx);
    return;
  }
  if (screen === 'result') {
    if (hit(p, retryRect())) { startLevel(state.levelIndex); return; }
    if (state.solved && state.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
      const nxt = state.levelIndex + 1;
      if (unlocked(nxt)) startLevel(nxt); else setScreen('levels');
      return;
    }
    if (hit(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect()))    { setScreen('levels'); return; }
    if (hit(p, undoBtnRect())) { undo(state); paint(); return; }
    if (hit(p, restartBtnRect())) { restart(state); paint(); return; }
    // Peg hit-test: widen each peg's hit zone to a comfortable touch column.
    for (let i = 0; i < 3; i++) {
      const col = { x: PEG_X[i] - 50, y: 90, w: 100, h: 300 };
      if (hit(p, col)) {
        if (tapPeg(state, i)) { afterMove(); paint(); }
        return;
      }
    }
  }
});

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === '1') tapPeg(state, 0);
  else if (e.key === '2') tapPeg(state, 1);
  else if (e.key === '3') tapPeg(state, 2);
  else if (e.key === 'z' || e.key === 'Z') undo(state);
  else if (e.key === 'r' || e.key === 'R') restart(state);
  afterMove();
  paint();
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#f4d27b';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 96);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 122);
  // Inline demo: 3-disk tower.
  drawTable(ctx);
  const demo = buildGame(0);
  drawDisks(ctx, demo);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '10px monospace';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 416);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 432);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 448);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 470);
}

function paintLevels() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#f4d27b';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#241936' : '#15102a';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#f4d27b' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 16);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a0a8b8' : '#5a6188';
    ctx.fillText(LEVELS[i].disks + ' ' + t(lang, 'disks') + ' · ' + t(lang, 'par') + ' ' + par(LEVELS[i].disks), r.x + 8, r.y + 32);
    ctx.textAlign = 'right';
    if (cleared) {
      const best = save.best[String(i)] || 999;
      drawStars(ctx, r.x + r.w - 64, r.y + 28, stars(best, LEVELS[i].disks), 12);
    } else if (!can) {
      ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 32);
    }
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawTable(ctx);
  drawDisks(ctx, state);
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  drawButton(t(lang, 'undo'),    undoBtnRect(),    true);
  drawButton(t(lang, 'restart'), restartBtnRect(), true);
  drawButton(t(lang, 'backToMenu'), backRect(),    true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f4d27b';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'moves') + ': ' + state.moves + '   ' + t(lang, 'par') + ': ' + par(state.disks),
    CANVAS_W / 2, 208);
  drawStars(ctx, CANVAS_W / 2 - 30, 240, stars(state.moves, state.disks), 18);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.solved && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 360, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(8,   CANVAS_H - 28, 60, 22); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 350, 100, 32); }
function undoBtnRect()    { return rect(112, CANVAS_H - 28, 60, 22); }
function restartBtnRect() { return rect(190, CANVAS_H - 28, 70, 22); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#28315c' : '#f4d27b';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#fff0c8';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#1a1224';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
