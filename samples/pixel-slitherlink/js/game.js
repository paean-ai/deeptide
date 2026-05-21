// Pixel Slitherlink - screen flow, edge-tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-slitherlink:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let puzzle = null;
let edgeState = null;
let solved = false;
let startTime = 0, elapsed = 0, timerHandle = null;

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
  puzzle = buildPuzzle(i);
  if (!puzzle) return;
  edgeState = new Array(puzzle.graph.edges.length).fill(E_BLANK);
  solved = false;
  startTime = Date.now(); elapsed = 0;
  screen = 'play';
  if (timerHandle) clearInterval(timerHandle);
  timerHandle = setInterval(() => {
    if (screen !== 'play') { clearInterval(timerHandle); timerHandle = null; return; }
    elapsed = Math.floor((Date.now() - startTime) / 1000);
    paint();
  }, 1000);
  paint();
}

// Cycle the edge nearest the tap: blank -> line -> cross -> blank.
function tapEdge(px, py) {
  const C = puzzle.C, G = puzzle.graph, g = gridGeometry(C);
  let best = -1, bd = (g.cell * 0.55) * (g.cell * 0.55);
  for (let e = 0; e < G.edges.length; e++) {
    const ed = G.edges[e];
    const ax = g.ox + ed.c * g.cell, ay = g.oy + ed.r * g.cell;
    const mx = ed.kind === 'h' ? ax + g.cell / 2 : ax;
    const my = ed.kind === 'h' ? ay : ay + g.cell / 2;
    const d = (px - mx) * (px - mx) + (py - my) * (py - my);
    if (d < bd) { bd = d; best = e; }
  }
  if (best < 0) return;
  edgeState[best] = (edgeState[best] + 1) % 3;
  if (isSolved(puzzle, edgeState)) {
    solved = true;
    if (!save.cleared.includes(puzzle.levelIndex)) save.cleared.push(puzzle.levelIndex);
    const key = String(puzzle.levelIndex);
    const score = Math.max(1, 999 - elapsed);
    if (!save.best[key] || save.best[key] < score) save.best[key] = score;
    persist();
    screen = 'result';
  }
  paint();
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
    if (hit(p, retryRect())) { startLevel(puzzle.levelIndex); return; }
    if (puzzle.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
      const nxt = puzzle.levelIndex + 1;
      if (unlocked(nxt)) startLevel(nxt); else setScreen('levels');
      return;
    }
    if (hit(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect())) {
      screen = 'levels';
      if (timerHandle) { clearInterval(timerHandle); timerHandle = null; }
      paint();
      return;
    }
    tapEdge(p.x, p.y);
  }
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
  ctx.font = 'bold 20px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#aab0c0';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 124);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#aab0c0';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 388);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 404);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 420);
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
    ctx.fillStyle = can ? PALETTE.panel : '#1b1d24';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.good : (can ? PALETTE.accent : '#3a3d48');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#5b6072';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 20);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#aab0c0' : '#5b6072';
    ctx.fillText(LEVELS[i].C + '×' + LEVELS[i].C, r.x + 8, r.y + 36);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(t(lang, 'cleared') + ' · ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawBoard(ctx, puzzle, edgeState, lang, solved);
  drawHud(ctx, lang, puzzle, elapsed);
  ctx.fillStyle = PALETTE.hudDim;
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 452);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.66)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = PALETTE.good;
  ctx.font = 'bold 23px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 172);
  ctx.fillStyle = '#f3f1e6';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'timeStr') + ': ' + ((elapsed / 60) | 0) + ':' + (elapsed % 60).toString().padStart(2, '0'), CANVAS_W / 2, 210);
  drawButton(t(lang, 'retry'), retryRect());
  if (puzzle.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 312, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 22, 60, 18); }
function retryRect()      { return rect(60,  320, 100, 36); }
function nextRect()       { return rect(200, 320, 100, 36); }
function menuRect()       { return rect(130, 368, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 62 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#2b2f3a' : '#f4c44a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3b3f4c' : '#ffe08a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f3f1e6' : '#16181f';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
