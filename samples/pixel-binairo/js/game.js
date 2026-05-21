// Pixel Binairo - screen flow, tap input, timer, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-binairo:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
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
  state = buildGame(i);
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
  if (screen === 'play') {
    if (hit(p, backRect())) {
      screen = 'levels';
      if (timerHandle) { clearInterval(timerHandle); timerHandle = null; }
      paint();
      return;
    }
    const g = gridGeometry(state.n);
    for (let r = 0; r < state.n; r++) for (let c = 0; c < state.n; c++) {
      const rect = cellRect(state.n, c, r);
      if (hit(p, rect)) {
        if (cycleCell(state, r, c)) {
          if (isSolved(state)) {
            state.solved = true;
            if (!save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
            const key = String(state.levelIndex);
            const score = Math.max(1, 999 - elapsed);
            if (!save.best[key] || save.best[key] < score) save.best[key] = score;
            persist();
            screen = 'result';
          }
        }
        paint();
        return;
      }
    }
  }
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
  ctx.fillStyle = '#ff9b3e';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 124);
  // Inline demo: a small 4-row strip of 0/1 tiles.
  const demo = [0,1,1,0, 1,0,0,1, 0,0,1,1, 1,1,0,0];
  const dc = 30, dx = 120, dy = 168;
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
    const v = demo[r*4+c];
    ctx.fillStyle = v === 0 ? '#3a4274' : '#ff9b3e';
    ctx.fillRect(dx + c*dc, dy + r*dc, dc - 2, dc - 2);
    ctx.fillStyle = '#0d1228';
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    ctx.fillText(String(v), dx + c*dc + dc/2 - 1, dy + r*dc + dc/2);
  }
  ctx.textBaseline = 'alphabetic';
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 326);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 342);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 358);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintLevels() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#ff9b3e';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#1c2240' : '#141a30';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#ff9b3e' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a0a8b8' : '#5a6188';
    ctx.fillText(LEVELS[i].n + '×' + LEVELS[i].n, r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(t(lang, 'cleared') + ' ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 34);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 34);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawHud(ctx, lang, state, elapsed);
  drawScene(ctx, state, conflicts(state));
  drawButton(t(lang, 'backToMenu'), backRect(), true);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 432);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#5fc06e';
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 180);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'timeStr') + ': ' + ((elapsed/60)|0) + ':' + (elapsed%60).toString().padStart(2,'0'),
    CANVAS_W / 2, 218);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 380, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 24, 60, 18); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 350, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#28315c' : '#ff9b3e';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#ffc070';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#0d1228';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
