// Pixel Suguru - screen flow, input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-suguru:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';        // 'title' | 'levels' | 'play' | 'result'
let puzzle = null;           // { n, regions, solution, clues, levelIndex }
let user = null;             // user-entered digits (0 = empty)
let notes = null;            // notes[i] = Set of pencil-mark digits
let selected = null;
let notesMode = false;
let mistakes = 0;
let startTime = 0;
let elapsed = 0;
const padHits = [];

const save = loadSave();

function loadSave() {
  try {
    const raw = localStorage.getItem(SAVE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return { cleared: [], best: {} };
}
function persist() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {}
}

function unlocked(i) { return i === 0 || save.cleared.includes(i - 1); }

function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startLevel(i) {
  puzzle = buildPuzzle(i);
  if (!puzzle) return;
  user = new Array(puzzle.n * puzzle.n).fill(0);
  notes = new Array(puzzle.n * puzzle.n).fill(null).map(() => new Set());
  selected = null;
  mistakes = 0;
  notesMode = false;
  startTime = Date.now();
  elapsed = 0;
  screen = 'play';
  tickTimer();
  paint();
}

let timerHandle = null;
function tickTimer() {
  if (timerHandle) clearInterval(timerHandle);
  timerHandle = setInterval(() => {
    if (screen !== 'play') { clearInterval(timerHandle); timerHandle = null; return; }
    elapsed = Math.floor((Date.now() - startTime) / 1000);
    paint();
  }, 1000);
}

// ---- input -------------------------------------------------------------
function relPoint(e) {
  const r = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) * (CANVAS_W / r.width),
    y: (e.clientY - r.top)  * (CANVAS_H / r.height),
  };
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
    if (hit(p, retryRect())) { startLevel(puzzle.levelIndex); return; }
    if (puzzle.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
      const next = puzzle.levelIndex + 1;
      if (unlocked(next)) startLevel(next);
      else setScreen('levels');
      return;
    }
    if (hit(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect())) { screen = 'levels'; if (timerHandle) { clearInterval(timerHandle); timerHandle = null; } paint(); return; }
    // Pad hits.
    for (const h of padHits) {
      if (p.x >= h.x && p.x <= h.x + h.w && p.y >= h.y && p.y <= h.y + h.h) {
        handlePad(h);
        return;
      }
    }
    // Grid hit.
    const g = gridGeometry(puzzle.n);
    if (p.x >= g.ox && p.x <= g.ox + g.total && p.y >= g.oy && p.y <= g.oy + g.total) {
      const c = ((p.x - g.ox) / g.cell) | 0;
      const r = ((p.y - g.oy) / g.cell) | 0;
      selected = r * puzzle.n + c;
      paint();
    }
  }
});

function handlePad(h) {
  if (selected == null) return;
  if (puzzle.clues[selected]) return;           // given - cannot edit
  if (h.kind === 'erase') {
    user[selected] = 0;
    notes[selected].clear();
  } else if (h.kind === 'notes') {
    notesMode = !notesMode;
  } else if (h.kind === 'digit') {
    if (notesMode) {
      if (notes[selected].has(h.v)) notes[selected].delete(h.v);
      else if (h.v <= regionSizeOf(selected)) notes[selected].add(h.v);
    } else {
      if (h.v > regionSizeOf(selected)) return;  // value larger than region
      if (user[selected] !== h.v && h.v !== puzzle.solution[selected]) mistakes++;
      user[selected] = h.v;
      notes[selected].clear();
    }
  }
  // Win check.
  const filled = puzzle.clues.map((v, i) => v || user[i]);
  if (isSolved(puzzle.n, puzzle.regions, filled)) {
    if (!save.cleared.includes(puzzle.levelIndex)) save.cleared.push(puzzle.levelIndex);
    const key = String(puzzle.levelIndex);
    const score = Math.max(1, 999 - elapsed - mistakes * 30);
    if (!save.best[key] || save.best[key] < score) save.best[key] = score;
    persist();
    screen = 'result';
  }
  paint();
}

function regionSizeOf(cellIdx) {
  for (const r of puzzle.regions) if (r.includes(cellIdx)) return r.length;
  return 5;
}

// Keyboard for desktop.
window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || selected == null) return;
  if (e.key >= '1' && e.key <= '9') handlePad({ kind: 'digit', v: parseInt(e.key, 10) });
  else if (e.key === 'Backspace' || e.key === 'Delete' || e.key === '0') handlePad({ kind: 'erase' });
  else if (e.key === 'n' || e.key === 'N') handlePad({ kind: 'notes' });
});

// ---- painting ----------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 26px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 110);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 138);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  // Demo glyph: a tiny 3x3 region preview.
  ctx.fillStyle = '#3a4274';
  ctx.fillRect(140, 200, 80, 80);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const demo = ['1','2','3','3',' ','1','2','3','1'];
  for (let i = 0; i < 9; i++) {
    const r = (i / 3) | 0, c = i % 3;
    ctx.fillText(demo[i], 140 + 13 + c * 27, 200 + 14 + r * 27);
  }
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '10px monospace';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 320);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 338);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 356);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintLevels() {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? PALETTE.card : '#1c2240';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.ok : (can ? '#bfc7e6' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(`L${i + 1} · ${LEVELS[i].n}×${LEVELS[i].n}`, r.x + 8, r.y + 20);
    ctx.fillStyle = can ? '#bfc7e6' : '#5a6188';
    ctx.font = '10px monospace';
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(`${t(lang, 'cleared')} · ${save.best[String(i)] || 0}`, r.x + r.w - 8, r.y + 22);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 22);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  drawHud(ctx, lang, puzzle.levelIndex, mistakes, elapsed);
  const conflicts = findViolations(puzzle.n, puzzle.regions, puzzle.clues.map((v, i) => v || user[i]));
  drawGrid(ctx, puzzle.n, puzzle.regions, puzzle.clues, user, notes, selected, conflicts);
  drawNumberPad(ctx, puzzle.n, { x: 12, y: 380, w: 336, h: 56 }, lang, notesMode, padHits);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.65)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(`${t(lang, 'timeStr')}: ${(elapsed/60|0)}:${(elapsed%60).toString().padStart(2,'0')}`, CANVAS_W / 2, 210);
  ctx.fillText(`${t(lang, 'mistakes')}: ${mistakes}`, CANVAS_W / 2, 232);
  drawButton(t(lang, 'retry'), retryRect());
  if (puzzle.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 380, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 24, 60, 18); }
function retryRect()      { return rect(60,  320, 100, 36); }
function nextRect()       { return rect(200, 320, 100, 36); }
function menuRect()       { return rect(130, 370, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 46);
}
function hit(p, r) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}

function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#28315c' : '#54c47c';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#86df9d';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
