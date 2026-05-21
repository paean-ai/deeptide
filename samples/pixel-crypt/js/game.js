// Pixel Crypt - screen flow, tap input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-crypt:save';

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

function onCellTap(cell) {
  if (state.flagMode) toggleFlag(state, cell);
  else                reveal(state, cell);
  if (state.over) {
    if (state.won) {
      if (!save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
      const key = String(state.levelIndex);
      const sc = finalScore(state);
      if (!save.best[key] || save.best[key] < sc) save.best[key] = sc;
      persist();
    }
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
    if (hit(p, backRect())) { setScreen('levels'); return; }
    if (hit(p, modeRect())) { state.flagMode = !state.flagMode; paint(); return; }
    const g = gridGeometry(state.n);
    if (p.x >= g.ox && p.x < g.ox + g.span && p.y >= g.oy && p.y < g.oy + g.span) {
      const c = ((p.x - g.ox) / g.cell) | 0;
      const r = ((p.y - g.oy) / g.cell) | 0;
      onCellTap(r * state.n + c);
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
  drawBackdrop(ctx);
  drawTitleArt(ctx, CANVAS_W / 2, 246);
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 23px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 104);
  ctx.fillStyle = '#b8aece';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#b8aece';
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
    ctx.fillStyle = can ? '#2c2740' : '#1c1828';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.good : (can ? PALETTE.accent : '#403a4e');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#675f78';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 20);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#b8aece' : '#675f78';
    let total = 0; for (const m of LEVELS[i].mix) total += m;
    ctx.fillText(LEVELS[i].n + '×' + LEVELS[i].n + ' · ☠' + total, r.x + 8, r.y + 36);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(t(lang, 'cleared') + ' · ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawBoard(ctx, state);
  drawHud(ctx, lang, state);
  drawButton(t(lang, state.flagMode ? 'flag' : 'reveal'), modeRect(), !state.flagMode);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.68)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.textAlign = 'center';
  ctx.fillStyle = state.won ? PALETTE.good : '#ff7a72';
  ctx.font = 'bold 24px monospace';
  ctx.fillText(t(lang, state.won ? 'win' : 'lose'), CANVAS_W / 2, 188);
  ctx.fillStyle = '#f3f1e6';
  ctx.font = '14px monospace';
  if (state.won) ctx.fillText('HP ' + state.hp + '  ·  ' + t(lang, 'lvl') + ' ' + state.level, CANVAS_W / 2, 224);
  else ctx.fillText('☠ ' + state.slain + '/' + state.monsterTotal, CANVAS_W / 2, 224);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.won && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 312, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 30, 64, 22); }
function modeRect()       { return rect(124, CANVAS_H - 34, 112, 30); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 348, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#2c2740' : '#f4c44a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#403a52' : '#ffe08a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f3f1e6' : '#16131f';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
