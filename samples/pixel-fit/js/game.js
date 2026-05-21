// Pixel Fit - screen flow, tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-fit:save';

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

function onWin() {
  if (!save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
  const key = String(state.levelIndex);
  if (!save.best[key] || save.best[key] > state.moves) save.best[key] = state.moves;
  persist();
  screen = 'result';
}

// ---- input -------------------------------------------------------------
function relPoint(e) {
  const r = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) * (CANVAS_W / r.width),
    y: (e.clientY - r.top)  * (CANVAS_H / r.height),
  };
}
function hit(p, r) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

// The piece index of the i-th tray (unplaced) slot.
function trayPiece(s, slotIndex) {
  let k = 0;
  for (let i = 0; i < s.pieces.length; i++) {
    if (s.pieces[i].placed) continue;
    if (k === slotIndex) return i;
    k++;
  }
  return -1;
}

function handlePlay(p) {
  if (hit(p, backRect())) { setScreen('levels'); return; }
  const g = frameGeo(state);
  // Frame tap.
  if (p.x >= g.ox && p.x < g.ox + g.totalW && p.y >= g.oy && p.y < g.oy + g.totalH) {
    const col = ((p.x - g.ox) / g.cell) | 0;
    const row = ((p.y - g.oy) / g.cell) | 0;
    const owner = cellOwner(state, col, row);
    if (owner !== -1) {
      pickUp(state, owner);
      state.selected = owner;
    } else if (state.selected >= 0 && !state.pieces[state.selected].placed) {
      if (place(state, state.selected, col, row)) {
        state.selected = -1;
        if (state.over) onWin();
      }
    }
    paint();
    return;
  }
  // Tray tap.
  let unplaced = 0;
  for (let i = 0; i < state.pieces.length; i++) if (!state.pieces[i].placed) unplaced++;
  for (let slot = 0; slot < unplaced; slot++) {
    if (hit(p, traySlot(state, slot))) {
      const pi = trayPiece(state, slot);
      if (state.selected === pi) rotatePiece(state, pi);
      else state.selected = pi;
      paint();
      return;
    }
  }
}

canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { setScreen('levels'); return; }
    if (hit(p, langRect())) { setLang(lang === 'en' ? 'zh' : 'en'); return; }
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
    if (state.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
      const nxt = state.levelIndex + 1;
      if (unlocked(nxt)) startLevel(nxt); else setScreen('levels');
      return;
    }
    if (hit(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  if (screen === 'play') handlePlay(p);
});

// ---- painting ----------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  drawBackdrop(ctx);
  drawTitleArt(ctx, CANVAS_W / 2, 244);
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 23px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 104);
  ctx.fillStyle = '#aab0c8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#aab0c8';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 392);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 408);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 424);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 462);
}

function paintLevels() {
  drawBackdrop(ctx);
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 40);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? PALETTE.frame : '#1a1d2c';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.good : (can ? PALETTE.accent : '#444a60');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#6b7188';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 20);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#aab0c8' : '#6b7188';
    ctx.fillText(LEVELS[i].w + '×' + LEVELS[i].h + ' · ' + LEVELS[i].k + ' pcs', r.x + 8, r.y + 36);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(t(lang, 'best') + ' ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawFrame(ctx, state);
  drawTray(ctx, state, lang);
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  ctx.fillStyle = PALETTE.hudDim;
  ctx.font = '9px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  const g = frameGeo(state);
  ctx.fillText(t(lang, state.selected >= 0 ? 'selHint' : 'rulesTxt1'),
               CANVAS_W / 2, g.oy + g.totalH + 8);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.66)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.textAlign = 'center';
  ctx.fillStyle = PALETTE.good;
  ctx.font = 'bold 24px monospace';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 176);
  ctx.fillStyle = '#f3f1e6';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'moves') + ': ' + state.moves, CANVAS_W / 2, 214);
  ctx.fillStyle = '#aab0c8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'best') + ': ' + (save.best[String(state.levelIndex)] || state.moves), CANVAS_W / 2, 238);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 308, 140, 40); }
function langRect()       { return rect(285, 6,   68,  20); }
function backRect()       { return rect(6,   5,   52,  20); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 350, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 62 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#2e3550' : '#f4c44a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3a4060' : '#ffe08a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f3f1e6' : '#1c2030';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
