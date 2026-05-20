// Pixel Woody - screen flow, tap-and-tap input, save, RAF.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-woody:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0, rafId = null;
let hover = null;            // {cx, cy} where the selected piece would land
let hoverValid = false;

const save = loadSave();
function loadSave() {
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return JSON.parse(raw); } catch (_) {}
  return { best: 0 };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startGame() {
  state = newGame(Date.now() % 2147483647);
  hover = null;
  screen = 'play';
  if (rafId) cancelAnimationFrame(rafId);
  lastT = 0;
  rafId = requestAnimationFrame(loop);
}

function loop(now) {
  rafId = requestAnimationFrame(loop);
  if (screen !== 'play') { lastT = now; return; }
  if (!lastT) lastT = now;
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.05) dt = 0.05;
  tick(state, dt);
  if (state.over) {
    if (state.score > save.best) save.best = state.score;
    persist();
    screen = 'result';
  }
  paint();
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

// Convert a canvas point to a (col, row) on the board, or null if off.
function pointToCell(p) {
  if (p.x < BOARD_OX || p.y < BOARD_OY) return null;
  const cx = ((p.x - BOARD_OX) / CELL) | 0;
  const cy = ((p.y - BOARD_OY) / CELL) | 0;
  if (cx < 0 || cy < 0 || cx >= GRID || cy >= GRID) return null;
  return { cx, cy };
}

canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { startGame(); return; }
    if (hit(p, langRect()))       { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'result') {
    if (hit(p, retryRect())) { startGame(); return; }
    if (hit(p, menuRect())) { setScreen('title'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect())) { setScreen('title'); return; }
    if (hit(p, newBtnRect())) { startGame(); return; }
    // Tray hit-test.
    for (let i = 0; i < TRAY_SIZE; i++) {
      if (hit(p, trayRect(i))) { tapTray(state, i); paint(); return; }
    }
    // Board hit-test — drop the selected piece here.
    const cell = pointToCell(p);
    if (cell && state.selected >= 0) {
      tryPlace(state, state.selected, cell.cx, cell.cy);
      hover = null;
      paint();
    }
  }
});

canvas.addEventListener('pointermove', (e) => {
  if (screen !== 'play' || state.selected < 0) return;
  const p = relPoint(e);
  const cell = pointToCell(p);
  if (!cell) { hover = null; return; }
  const shape = state.tray[state.selected];
  hover = {
    cells: shape.cells.map(([cx, cy]) => [cell.cx + cx, cell.cy + cy]),
    valid: canPlace(state.grid, shape, cell.cx, cell.cy),
  };
});
canvas.addEventListener('pointerleave', () => { hover = null; });

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === '1') tapTray(state, 0);
  else if (e.key === '2') tapTray(state, 1);
  else if (e.key === '3') tapTray(state, 2);
  paint();
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#f4d27b';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#a08c70';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  // Inline demo: 3 sample shapes from the bank.
  const demo = newGame(7);
  for (let i = 0; i < 3 && i < demo.tray.length; i++) {
    const slot = trayRect(i);
    const piece = demo.tray[i];
    let maxX = 0, maxY = 0;
    for (const [cx, cy] of piece.cells) { if (cx > maxX) maxX = cx; if (cy > maxY) maxY = cy; }
    const pw = (maxX + 1) * 24, ph = (maxY + 1) * 24;
    const ox = slot.x + ((slot.w - pw) / 2) | 0;
    const oy = 180 + ((96 - ph) / 2) | 0;
    for (const [cx, cy] of piece.cells) {
      drawBlockCell(ctx, ox + cx * 24, oy + cy * 24, 24, 24, piece.color);
    }
  }
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#a08c70';
  ctx.font = '10px monospace';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 360);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 376);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 392);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawBoard(ctx, state, hover);
  drawTray(ctx, state);
  drawFlash(ctx, state);
  drawHud(ctx, lang, state, save.best || 0);
  drawButton(t(lang, 'backToMenu'), backRect(),   true);
  drawButton(t(lang, 'newGame'),    newBtnRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f4d27b';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'over'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#fbf3e2';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'score') + ': ' + state.score, CANVAS_W / 2, 210);
  ctx.fillStyle = '#a08c70';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'high') + ': ' + (save.best || 0), CANVAS_W / 2, 236);
  drawButton(t(lang, 'newGame'), retryRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 410, 140, 38); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(244, CANVAS_H - 36, 100, 28); }
function newBtnRect()     { return rect(20,  CANVAS_H - 36, 100, 28); }
function retryRect()      { return rect(80,  280, 100, 36); }
function menuRect()       { return rect(190, 280, 100, 36); }
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#3a2b1c' : '#f4d27b';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#5c4226' : '#fff0c8';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#fbf3e2' : '#1a120a';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
