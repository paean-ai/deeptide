// Pixel Dice - screen flow, tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-dice:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let play = null;

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
  const puzzle = buildPuzzle(i);
  if (!puzzle) return;
  play = newPlay(puzzle);
  screen = 'play';
  paint();
}

function afterMove() {
  if (play.over) {
    const i = play.puzzle.levelIndex;
    if (!save.cleared.includes(i)) save.cleared.push(i);
    const key = String(i);
    if (save.best[key] === undefined || save.best[key] > play.moves) save.best[key] = play.moves;
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
    if (hit(p, retryRect())) { startLevel(play.puzzle.levelIndex); return; }
    if (play.puzzle.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
      const nxt = play.puzzle.levelIndex + 1;
      if (unlocked(nxt)) startLevel(nxt); else setScreen('levels');
      return;
    }
    if (hit(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect()))    { setScreen('levels'); return; }
    if (hit(p, undoRect()))    { undo(play); paint(); return; }
    if (hit(p, restartRect())) { restart(play); paint(); return; }
    const g = gridGeometry(play.puzzle.n);
    if (p.x >= g.ox && p.x < g.ox + g.span && p.y >= g.oy && p.y < g.oy + g.span) {
      const c = ((p.x - g.ox) / g.cell) | 0, r = ((p.y - g.oy) / g.cell) | 0;
      const cell = r * play.puzzle.n + c;
      const n = play.puzzle.n, dx = (cell % n) - (play.cell % n), dy = ((cell / n) | 0) - ((play.cell / n) | 0);
      let dir = null;
      if (dx === 1 && dy === 0) dir = 'E';
      else if (dx === -1 && dy === 0) dir = 'W';
      else if (dx === 0 && dy === 1) dir = 'S';
      else if (dx === 0 && dy === -1) dir = 'N';
      if (dir && rollDie(play, dir)) afterMove();
    }
  }
});

window.addEventListener('keydown', (e) => {
  if (screen !== 'play') return;
  let dir = null;
  if (e.key === 'ArrowRight' || e.key === 'd') dir = 'E';
  else if (e.key === 'ArrowLeft' || e.key === 'a') dir = 'W';
  else if (e.key === 'ArrowDown' || e.key === 's') dir = 'S';
  else if (e.key === 'ArrowUp' || e.key === 'w') dir = 'N';
  else if (e.key === 'z' || e.key === 'Z') { undo(play); paint(); return; }
  else if (e.key === 'r' || e.key === 'R') { restart(play); paint(); return; }
  if (dir && rollDie(play, dir)) afterMove();
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
  drawTitleArt(ctx, CANVAS_W / 2, 248);
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 23px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 104);
  ctx.fillStyle = '#aaa2c0';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#aaa2c0';
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
    ctx.fillStyle = can ? PALETTE.panel : '#1e1b2a';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.good : (can ? PALETTE.accent : '#3a3650');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#5f5a75';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 20);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#aaa2c0' : '#5f5a75';
    ctx.fillText(LEVELS[i].n + '×' + LEVELS[i].n + ' · ' + LEVELS[i].k + ' seals', r.x + 8, r.y + 36);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(t(lang, 'cleared') + ' · ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawBoard(ctx, play);
  drawHud(ctx, lang, play);
  drawButton(t(lang, 'undo'), undoRect(), true);
  drawButton(t(lang, 'restart'), restartRect(), true);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.66)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = PALETTE.good;
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 168);
  // Stars.
  const st = stars(play.moves, play.puzzle.par);
  ctx.font = 'bold 26px monospace';
  for (let i = 0; i < 3; i++) {
    ctx.fillStyle = i < st ? PALETTE.accent : '#3a3650';
    ctx.fillText('★', CANVAS_W / 2 + (i - 1) * 34, 206);
  }
  ctx.fillStyle = '#f3f1e6';
  ctx.font = '13px monospace';
  ctx.fillText(t(lang, 'moves') + ' ' + play.moves + '  ·  ' + t(lang, 'par') + ' ' + play.puzzle.par,
               CANVAS_W / 2, 240);
  drawButton(t(lang, 'retry'), retryRect());
  if (play.puzzle.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 312, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 22, 60, 18); }
function undoRect()       { return rect(48,  400, 120, 38); }
function restartRect()    { return rect(192, 400, 120, 38); }
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
  ctx.fillStyle = dim ? '#2e2a40' : '#f4c44a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3e3a56' : '#ffe08a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f3f1e6' : '#1a1726';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
