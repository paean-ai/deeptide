// Pixel Klotski - screen flow, tap-select + drag-to-slide + keyboard, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-klotski:save';

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

// Convert a canvas (px) point to a (col, row) board cell. Returns null if
// the tap is outside the playfield.
function tapToCell(p) {
  const g = gridGeometry();
  if (p.x < g.ox || p.y < g.oy || p.x > g.ox + g.bw || p.y > g.oy + g.bh) return null;
  return {
    c: Math.floor((p.x - g.ox) / g.cell),
    r: Math.floor((p.y - g.oy) / g.cell),
  };
}

let dragStart = null;
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
    // Tap selects whichever block sits at the tapped cell.
    const cell = tapToCell(p);
    if (cell) {
      const idx = pieceAt(state.pieces, cell.c, cell.r);
      if (idx >= 0) tapPiece(state, idx);
    }
    dragStart = p;
  }
});

canvas.addEventListener('pointermove', (e) => {
  if (!dragStart || screen !== 'play' || state.selected < 0) return;
  const p = relPoint(e);
  const dx = p.x - dragStart.x, dy = p.y - dragStart.y;
  const TH = 24;
  if (Math.max(Math.abs(dx), Math.abs(dy)) < TH) return;
  let mx = 0, my = 0;
  if (Math.abs(dx) > Math.abs(dy)) mx = dx > 0 ? 1 : -1;
  else my = dy > 0 ? 1 : -1;
  if (trySlide(state, mx, my)) { afterMove(); paint(); }
  dragStart = p;
});
canvas.addEventListener('pointerup', () => { dragStart = null; });
canvas.addEventListener('pointercancel', () => { dragStart = null; });

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  let mx = 0, my = 0;
  if (e.key === 'ArrowLeft' || e.key === 'a' || e.key === 'A')      mx = -1;
  else if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') mx = 1;
  else if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W')    my = -1;
  else if (e.key === 'ArrowDown' || e.key === 's' || e.key === 'S')  my = 1;
  else if (e.key === 'z' || e.key === 'Z') { undo(state); paint(); return; }
  else if (e.key === 'r' || e.key === 'R') { restart(state); paint(); return; }
  if (mx || my) { if (trySlide(state, mx, my)) { afterMove(); paint(); } }
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
  ctx.fillStyle = '#e85a3a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 96);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 122);
  // Inline demo: L2 (Squeeze) — shows a small classic Klotski look.
  const demo = buildGame(1);
  drawBoard(ctx, demo);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
}

function paintLevels() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#e85a3a';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#28203a' : '#15102a';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#e85a3a' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a0a8b8' : '#5a6188';
    ctx.fillText(t(lang, 'par') + ' ' + LEVELS[i].par + ' · ' + LEVELS[i].pieces.length + ' blocks', r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) {
      const best = save.best[String(i)] || 9999;
      drawStars(ctx, r.x + r.w - 64, r.y + 28, stars(best, LEVELS[i].par), 12);
    } else if (!can) {
      ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 34);
    }
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  drawBoard(ctx, state);
  drawButton(t(lang, 'undo'),    undoBtnRect(),    true);
  drawButton(t(lang, 'restart'), restartBtnRect(), true);
  drawButton(t(lang, 'backToMenu'), backRect(),    true);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 416);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#e85a3a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'moves') + ': ' + state.moves + '   ' + t(lang, 'par') + ': ' + state.lv.par,
    CANVAS_W / 2, 208);
  drawStars(ctx, CANVAS_W / 2 - 30, 240, stars(state.moves, state.lv.par), 18);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.solved && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 410, 140, 38); }
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
  ctx.fillStyle = dim ? '#28315c' : '#e85a3a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#ff8a6a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#1c1424';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
