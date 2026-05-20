// Pixel Black Box - screen flow, tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-blackbox:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let confirmReveal = false;

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
  confirmReveal = false;
  screen = 'play';
  paint();
}

function doReveal() {
  reveal(state);
  const key = String(state.levelIndex);
  if (state.solved && !save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
  if (!save.best[key] || save.best[key] < state.score) save.best[key] = state.score;
  persist();
  confirmReveal = false;
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
  if (screen === 'play') {
    if (hit(p, backRect())) { setScreen('levels'); return; }
    if (confirmReveal) {
      if (hit(p, yesRect())) { doReveal(); return; }
      if (hit(p, noRect()))  { confirmReveal = false; paint(); return; }
      return;
    }
    if (!state.revealed && hit(p, revealRect())) { confirmReveal = true; paint(); return; }
    if (state.revealed) {
      if (hit(p, retryRect())) { startLevel(state.levelIndex); return; }
      if (state.solved && state.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
        const nxt = state.levelIndex + 1;
        if (unlocked(nxt)) startLevel(nxt); else setScreen('levels');
        return;
      }
      if (hit(p, menuRect())) { setScreen('levels'); return; }
      return;
    }
    // Edge probe hit-test
    for (let i = 0; i < 4 * state.n; i++) {
      if (hit(p, edgeRect(state.n, i))) { fireProbe(state, i); paint(); return; }
    }
    // Cell hit-test (interior)
    for (let y = 0; y < state.n; y++) for (let x = 0; x < state.n; x++) {
      if (hit(p, gridCellRect(state.n, x, y))) { toggleMark(state, x, y); paint(); return; }
    }
  }
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
}

function paintTitle() {
  ctx.fillStyle = '#0d0918';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#bda6ff';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#a8a0c4';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  // Inline demo: 3x3 grid with one atom, one probe button result.
  const demoX = 90, demoY = 168, dc = 28;
  for (let y = 0; y < 3; y++) for (let x = 0; x < 3; x++) {
    ctx.fillStyle = (x + y) % 2 === 0 ? '#1f1838' : '#2d2452';
    ctx.fillRect(demoX + x * dc, demoY + y * dc, dc - 1, dc - 1);
  }
  // Probe button to the left of row 1
  ctx.fillStyle = '#3a2c5a';
  ctx.fillRect(demoX - dc, demoY + dc, dc - 1, dc - 1);
  ctx.fillStyle = '#5fc0ff';
  ctx.font = 'bold 14px monospace';
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  ctx.fillText('R', demoX - dc / 2, demoY + dc + dc / 2);
  // Atom in centre
  const cx = demoX + dc + dc / 2, cy = demoY + dc + dc / 2;
  ctx.fillStyle = '#bda6ff';
  fillDisk(ctx, cx, cy, 8);
  ctx.fillStyle = '#e3d3ff';
  fillDisk(ctx, cx - 2, cy - 2, 4);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#a8a0c4';
  ctx.font = '10px monospace';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 332);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 348);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 364);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintLevels() {
  ctx.fillStyle = '#0d0918';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#bda6ff';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#1a1230' : '#100a20';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#bda6ff' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a8a0c4' : '#5a6188';
    ctx.fillText(LEVELS[i].n + '×' + LEVELS[i].n + ' · ' + LEVELS[i].atoms + ' atoms', r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) {
      ctx.fillStyle = '#f4d27b';
      ctx.fillText(t(lang, 'high') + ' ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 34);
    } else if (!can) {
      ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 34);
    }
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  drawBoard(ctx, state);
  if (!state.revealed) drawButton(t(lang, 'reveal'), revealRect());
  drawButton(t(lang, 'backToMenu'), backRect(), true);
  if (state.revealed) {
    // Result panel below the grid.
    ctx.fillStyle = 'rgba(0,0,0,0.62)';
    ctx.fillRect(0, 380, CANVAS_W, 100);
    ctx.fillStyle = state.solved ? '#5fc06e' : '#ffd34a';
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(state.solved ? t(lang, 'perfect') : (t(lang, 'correct') + ' ' + state.correctMarks + '/' + state.lv.atoms + '  ' + t(lang, 'wrong') + ' ' + state.wrongMarks), CANVAS_W / 2, 400);
    ctx.fillStyle = '#f8f5e8';
    ctx.font = '12px monospace';
    ctx.fillText(t(lang, 'score') + ': ' + state.score, CANVAS_W / 2, 420);
    drawButton(t(lang, 'retry'), retryRect(), true);
    if (state.solved && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
    drawButton(t(lang, 'backToMenu'), menuRect(), true);
  }
  if (confirmReveal) {
    ctx.fillStyle = 'rgba(0,0,0,0.72)';
    ctx.fillRect(0, 180, CANVAS_W, 120);
    ctx.fillStyle = '#f8f5e8';
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t(lang, 'revealQ'), CANVAS_W / 2, 220);
    drawButton(t(lang, 'yes'), yesRect());
    drawButton(t(lang, 'no'), noRect(), true);
  }
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 388, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(8,   CANVAS_H - 24, 60, 18); }
function revealRect()     { return rect(130, 388, 100, 32); }
function retryRect()      { return rect(20,  434, 90,  28); }
function nextRect()       { return rect(120, 434, 90,  28); }
function menuRect()       { return rect(220, 434, 110, 28); }
function yesRect()        { return rect(80,  248, 80,  32); }
function noRect()         { return rect(200, 248, 80,  32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#2a2244' : '#bda6ff';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c3460' : '#e3d3ff';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#0d0918';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
