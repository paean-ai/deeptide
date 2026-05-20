// Pixel Nurikabe - screen flow, input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-nurikabe:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';        // 'title' | 'levels' | 'play' | 'result'
let puzzle = null;
let grid = null;             // live grid of tags (0/1/k+2)
let mistakes = 0;
let startTime = 0;
let elapsed = 0;
let timerHandle = null;

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
  grid = new Array(puzzle.n * puzzle.n).fill(0);
  // Pre-mark clue cells as "island k+2" so the player can't paint them.
  for (let k = 0; k < puzzle.clues.length; k++) {
    const c = puzzle.clues[k];
    grid[c.y * puzzle.n + c.x] = k + 2;
  }
  mistakes = 0;
  startTime = Date.now();
  elapsed = 0;
  screen = 'play';
  if (timerHandle) clearInterval(timerHandle);
  timerHandle = setInterval(() => {
    if (screen !== 'play') { clearInterval(timerHandle); timerHandle = null; return; }
    elapsed = Math.floor((Date.now() - startTime) / 1000);
    paint();
  }, 1000);
  paint();
}

function whichIsland(cellIdx) {
  // For "marked as island" cells the player paints, we don't know which
  // island id it belongs to — store as `2` (a generic "island" mark). The
  // solver uses the same scheme for the head's k+2 tag, and for the
  // user-painted cells we just track non-sea.
  // To keep islands distinct in findViolations / isSolved we colour each
  // user-painted cell with its nearest-orthogonal-clue tag at win check.
  return 2;
}

function cycleCell(i) {
  if (i < 0 || i >= puzzle.n * puzzle.n) return;
  // Clue cell: not editable.
  for (const c of puzzle.clues) if (c.y * puzzle.n + c.x === i) return;
  // Cycle: unknown -> sea -> island-mark -> unknown.
  const v = grid[i];
  if (v === 0)      grid[i] = 1;            // sea
  else if (v === 1) grid[i] = 99;           // generic island mark
  else              grid[i] = 0;
  checkWin();
  paint();
}

function infer(grid, n, clues) {
  // For win-check + violation, propagate each user-marked island cell to its
  // (unique) connected island clue. A user mark connected to clue k via
  // 4-connected non-sea cells -> tag k+2. Cells not reachable from any clue
  // stay as 99 ("dangling" — flagged on win check as not a real island).
  const out = grid.slice();
  // Replace each clue cell tag (already k+2) so BFS finds it.
  const n2 = n * n;
  for (let i = 0; i < n2; i++) if (out[i] === 99) out[i] = 100; // mark for propagation
  for (let k = 0; k < clues.length; k++) {
    const start = clues[k].y * n + clues[k].x;
    const q = [start];
    const seen = new Set([start]);
    while (q.length) {
      const v = q.shift();
      const x = v % n, y = (v / n) | 0;
      for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
        const nx = x + dx, ny = y + dy;
        if (nx<0||nx>=n||ny<0||ny>=n) continue;
        const ni = ny * n + nx;
        if (seen.has(ni)) continue;
        if (out[ni] === k + 2 || out[ni] === 100) {
          out[ni] = k + 2;
          seen.add(ni); q.push(ni);
        }
      }
    }
  }
  return out;
}

function checkWin() {
  const inferred = infer(grid, puzzle.n, puzzle.clues);
  // Cells still at 100 mean orphan island marks — not a win.
  for (let i = 0; i < puzzle.n * puzzle.n; i++) if (inferred[i] === 100) return;
  if (isSolved(puzzle.n, puzzle.clues, inferred)) {
    if (!save.cleared.includes(puzzle.levelIndex)) save.cleared.push(puzzle.levelIndex);
    const key = String(puzzle.levelIndex);
    const score = Math.max(1, 999 - elapsed - mistakes * 30);
    if (!save.best[key] || save.best[key] < score) save.best[key] = score;
    persist();
    screen = 'result';
  }
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
      const next = puzzle.levelIndex + 1;
      if (unlocked(next)) startLevel(next);
      else setScreen('levels');
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
    const g = gridGeometry(puzzle.n);
    if (p.x >= g.ox && p.x <= g.ox + g.total && p.y >= g.oy && p.y <= g.oy + g.total) {
      const c = ((p.x - g.ox) / g.cell) | 0;
      const r = ((p.y - g.oy) / g.cell) | 0;
      cycleCell(r * puzzle.n + c);
    }
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
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 26px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 126);
  // Demo: tiny 4x4 nurikabe icon.
  const dx = 132, dy = 160, dc = 24;
  const demo = [
    [1,1,1,1],
    [1,'2',1,'1'],
    [1,1,1,1],
    [1,'3','x',1],
  ];
  for (let y = 0; y < 4; y++) for (let x = 0; x < 4; x++) {
    const v = demo[y][x];
    ctx.fillStyle = v === 1 ? PALETTE.cellSea : PALETTE.cellBlank;
    ctx.fillRect(dx + x * dc, dy + y * dc, dc - 1, dc - 1);
    if (typeof v === 'string' && v !== 'x') {
      ctx.fillStyle = PALETTE.clueBg;
      ctx.fillRect(dx + x * dc, dy + y * dc, dc - 1, dc - 1);
      ctx.fillStyle = PALETTE.clueText;
      ctx.font = 'bold 14px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(v, dx + x * dc + dc / 2 - 1, dy + y * dc + dc / 2);
    } else if (v === 'x') {
      ctx.fillStyle = PALETTE.dotColor;
      ctx.fillRect(dx + x * dc + dc/2 - 2, dy + y * dc + dc/2 - 2, 4, 4);
    }
  }
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 320);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 336);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 352);
  ctx.fillText(t(lang, 'rulesTxt4'), CANVAS_W / 2, 368);
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
    ctx.fillText(`islands ${LEVELS[i].sizes.length}`, r.x + 8, r.y + 36);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(`${t(lang, 'cleared')} · ${save.best[String(i)] || 0}`, r.x + r.w - 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  drawHud(ctx, lang, puzzle.levelIndex, mistakes, elapsed);
  const inferred = infer(grid, puzzle.n, puzzle.clues);
  const conflicts = findViolations(puzzle.n, puzzle.clues, inferred);
  drawGrid(ctx, puzzle.n, puzzle.clues, grid, conflicts);
  // Legend.
  ctx.fillStyle = PALETTE.hudDim;
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt4'), CANVAS_W / 2, 426);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.6)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(`${t(lang, 'timeStr')}: ${(elapsed/60|0)}:${(elapsed%60).toString().padStart(2,'0')}`, CANVAS_W / 2, 210);
  drawButton(t(lang, 'retry'), retryRect());
  if (puzzle.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 390, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 24, 60, 18); }
function retryRect()      { return rect(60,  320, 100, 36); }
function nextRect()       { return rect(200, 320, 100, 36); }
function menuRect()       { return rect(130, 370, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
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
