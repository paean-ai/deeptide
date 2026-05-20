// Pixel Knight - screen flow, tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-knight:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let showHint = true;

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
  if (state.over) {
    const key = String(state.levelIndex);
    const moves = state.moves;
    if (state.won && !save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
    // Best = highest cell-count covered (lower is worse here).
    if (!save.best[key] || save.best[key] < moves) save.best[key] = moves;
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

function pointToCell(p) {
  const g = gridGeometry(state.n);
  if (p.x < g.ox || p.y < g.oy || p.x > g.ox + g.bw || p.y > g.oy + g.bw) return null;
  const c = Math.floor((p.x - g.ox) / g.cell);
  const r = Math.floor((p.y - g.oy) / g.cell);
  if (c < 0 || r < 0 || c >= state.n || r >= state.n) return null;
  return { c, r };
}

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
    if (state.won && state.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
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
    if (hit(p, hintBtnRect())) { showHint = !showHint; paint(); return; }
    const cell = pointToCell(p);
    if (cell) {
      if (tryMove(state, cell.c, cell.r)) { afterMove(); paint(); }
    }
  }
});

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === 'z' || e.key === 'Z') { undo(state); paint(); }
  else if (e.key === 'r' || e.key === 'R') { restart(state); paint(); }
  else if (e.key === 'h' || e.key === 'H') { showHint = !showHint; paint(); }
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
  ctx.fillStyle = '#5fc0ff';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  // Inline demo: render L1 board state.
  const demo = buildGame(0);
  drawBoard(ctx, demo);
  const tgts = legalMoves(demo.n, demo.visited, demo.cx, demo.cy);
  const hints = warnsdorffHints(demo.n, demo.visited, demo.cx, demo.cy);
  drawTargets(ctx, demo, tgts, hints);
  drawKnight(ctx, demo);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '10px monospace';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 412);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 428);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 444);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 470);
}

function paintLevels() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#5fc0ff';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#28203a' : '#150f24';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#5fc0ff' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a0a8b8' : '#5a6188';
    ctx.fillText(LEVELS[i].n + '×' + LEVELS[i].n + ' from (' + LEVELS[i].sx + ',' + LEVELS[i].sy + ')', r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) {
      drawStars(ctx, r.x + r.w - 64, r.y + 28, 3, 12);
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
  const tgts = state.over ? [] : legalMoves(state.n, state.visited, state.cx, state.cy);
  const hints = (state.over || !showHint) ? [] : warnsdorffHints(state.n, state.visited, state.cx, state.cy);
  drawTargets(ctx, state, tgts, hints);
  drawKnight(ctx, state);
  drawButton(t(lang, 'undo'),    undoBtnRect(),    true);
  drawButton(t(lang, 'restart'), restartBtnRect(), true);
  drawButton((showHint ? '· ' : '') + t(lang, 'hint'), hintBtnRect(), !showHint);
  drawButton(t(lang, 'backToMenu'), backRect(),    true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = state.won ? '#5fc06e' : '#ff7a7a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(state.won ? t(lang, 'win') : t(lang, 'lose'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'visited') + ': ' + state.moves + ' / ' + (state.n * state.n), CANVAS_W / 2, 210);
  drawStars(ctx, CANVAS_W / 2 - 30, 240, stars(state.moves, state.n * state.n), 18);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.won && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 388, 140, 38); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(8,   CANVAS_H - 28, 60, 22); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 350, 100, 32); }
function undoBtnRect()    { return rect(80,  CANVAS_H - 28, 60, 22); }
function restartBtnRect() { return rect(150, CANVAS_H - 28, 70, 22); }
function hintBtnRect()    { return rect(228, CANVAS_H - 28, 60, 22); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#28315c' : '#5fc0ff';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#a8e0ff';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#0a0a18';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
