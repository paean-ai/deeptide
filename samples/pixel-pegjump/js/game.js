// Pixel Peg Jump - screen flow, tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-pegjump:save';

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

function checkEndAndRecord() {
  if (!state.over) return;
  if (state.won) {
    if (!save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
    const key = String(state.levelIndex);
    const left = pegCount(state);
    const s = stars(left);
    if (!save.best[key] || save.best[key] < s) save.best[key] = s;
    persist();
    screen = 'result';
  } else {
    screen = 'stuck';
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
    if (hit(p, langRect()))       { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'levels') {
    if (hit(p, backRect())) { setScreen('title'); return; }
    const idx = hitLevelCell(p);
    if (idx >= 0 && unlocked(idx)) startLevel(idx);
    return;
  }
  if (screen === 'result' || screen === 'stuck') {
    if (hit(p, retryRect())) { startLevel(state.levelIndex); return; }
    if (screen === 'result' && state.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
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
    // Cell hit-test.
    for (let y = 0; y < state.h; y++) for (let x = 0; x < state.w; x++) {
      if (state.grid[y][x] === ' ') continue;
      const r = cellRect(state.w, state.h, x, y);
      if (hit(p, r)) {
        if (tapCell(state, x, y)) {
          checkEndAndRecord();
          paint();
        }
        return;
      }
    }
  }
});

// ---- painting ----------------------------------------------------------
function paint() {
  if      (screen === 'title')   paintTitle();
  else if (screen === 'levels')  paintLevels();
  else if (screen === 'play')    paintPlay();
  else if (screen === 'result' || screen === 'stuck') paintResult();
}

function paintTitle() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#f4d27b';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 96);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 122);
  // Inline demo: 3 pegs + 1 hole, plus an arrow showing a jump.
  const dx = 130, dy = 200, dc = 30;
  for (let i = 0; i < 4; i++) {
    const cx = dx + i * dc + dc / 2, cy = dy + dc / 2;
    ctx.fillStyle = PALETTE.boardEdge;
    ctx.fillRect(dx + i * dc, dy, dc - 1, dc - 1);
    ctx.fillStyle = PALETTE.board;
    ctx.fillRect(dx + i * dc + 1, dy + 1, dc - 3, dc - 3);
    ctx.fillStyle = PALETTE.hole;
    drawDisk(ctx, cx, cy, 9);
    if (i !== 3) drawPeg(ctx, cx, cy, 8, false, false);
  }
  ctx.strokeStyle = PALETTE.pegTgt;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(dx + dc + dc / 2, dy + dc + 8);
  ctx.lineTo(dx + 3 * dc + dc / 2, dy + dc + 8);
  ctx.stroke();
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 320);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 336);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 352);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
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
    ctx.fillStyle = can ? PALETTE.cardLite : PALETTE.cardDark;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#bfc7e6' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    // Show row count as size proxy.
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#bfc7e6' : '#5a6188';
    const w = LEVELS[i].rows.reduce((m, rs) => Math.max(m, rs.length), 0);
    const pegs = LEVELS[i].rows.join('').split('O').length - 1;
    ctx.fillText(w + '×' + LEVELS[i].rows.length + ' · ' + pegs, r.x + 8, r.y + 32);
    ctx.textAlign = 'right';
    if (cleared) {
      drawStars(ctx, r.x + r.w - 60, r.y + 28, save.best[String(i)] || 0, 12);
    } else if (!can) {
      ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 32);
    }
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawHud(ctx, lang, state);
  // Light suggestion ring on legal landing cells when something selected.
  let suggested = null;
  if (state.sel) {
    suggested = new Set();
    for (const m of movesFromGame()) suggested.add(m.to[0] + ',' + m.to[1]);
  }
  drawBoard(ctx, state, suggested);
  drawButton(t(lang, 'undo'),    undoBtnRect(),    true);
  drawButton(t(lang, 'restart'), restartBtnRect(), true);
  drawButton(t(lang, 'backToMenu'), backRect(),    true);
}
function movesFromGame() {
  return movesFrom(state, state.sel[0], state.sel[1]);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  const left = pegCount(state);
  ctx.fillStyle = state.won ? '#f4d27b' : '#bfc7e6';
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(state.won ? t(lang, 'win') : t(lang, 'almost'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'pegsLeft') + ': ' + left + '   ' + t(lang, 'moves') + ': ' + state.history.length,
    CANVAS_W / 2, 208);
  if (state.won) drawStars(ctx, CANVAS_W / 2 - 30, 240, stars(left), 18);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.won && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 380, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(8,   CANVAS_H - 28, 60, 22); }
function retryRect()      { return rect(60,  320, 100, 36); }
function nextRect()       { return rect(200, 320, 100, 36); }
function menuRect()       { return rect(130, 370, 100, 32); }
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
  ctx.fillStyle = dim ? '#3c4576' : '#fcf0c4';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#10142a';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
