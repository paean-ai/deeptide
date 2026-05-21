// Pixel Carousel - screen flow, swipe input, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-carousel:save';
const G = {
  screen: 'title',          // title | select | play
  play: null,
  levelIndex: 0,
  save: { cleared: [], stars: [] },
};

function loadSave() {
  try {
    const o = JSON.parse(localStorage.getItem(SAVE_KEY) || '{}');
    if (Array.isArray(o.cleared)) G.save.cleared = o.cleared;
    if (Array.isArray(o.stars)) G.save.stars = o.stars;
  } catch (e) { /* fresh save */ }
}
function persist() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(G.save)); } catch (e) { /* ignore */ }
}
function unlocked(i) { return i === 0 || G.save.cleared[i - 1] === true; }
function hitR(r, p) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

// ---- geometry ------------------------------------------------------------
const startBtn = { x: 96, y: 322, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };
const backBtn = { x: 110, y: 422, w: 140, h: 38 };
function levelRow(i) { return { x: 36, y: 92 + i * 52, w: 288, h: 44 }; }
const cardX = 44, cardY = 128, cardW = 272, cardH = 220;
const rRetry = { x: cardX + 18, y: cardY + 146, w: 118, h: 36 };
const rNext = { x: cardX + 136, y: cardY + 146, w: 118, h: 36 };
const rRetryWide = { x: cardX + 18, y: cardY + 146, w: 236, h: 36 };
const rMenu = { x: cardX + 18, y: cardY + 190, w: 236, h: 32 };

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 56, 26, PAL.text);
  fillText(ctx, t('subtitle'), 180, 84, 9.5, PAL.dim);
  drawBtn(ctx, startBtn, t('start'), PAL.good, true);
  drawBtn(ctx, langBtn, lang === 'en' ? '中文' : 'EN', PAL.star, false);
  fillText(ctx, t('rules1'), 180, 388, 9, PAL.dim);
  fillText(ctx, t('rules2'), 180, 406, 9, PAL.dim);
  fillText(ctx, t('rules3'), 180, 424, 9, PAL.dim);
}

function renderSelect() {
  drawBackdrop(ctx);
  fillText(ctx, t('levelSelect'), 180, 56, 18, PAL.text);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelRow(i), open = unlocked(i), lv = LEVELS[i];
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, open ? PAL.panel : PAL.bg0);
    px(ctx, r.x + 2, r.y + 2, 5, r.h - 4, open ? TILE[i % TILE.length].base : PAL.panelHi);
    fillText(ctx, (i + 1) + '.', r.x + 24, r.y + r.h / 2, 12, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? L(lv.name).toUpperCase() : t('locked'),
      r.x + 46, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    fillText(ctx, lv.cols + '×' + lv.rows, r.x + r.w - 102, r.y + r.h / 2, 11, PAL.dim, 'left');
    if (open && G.save.cleared[i]) drawStars(ctx, r.x + r.w - 42, r.y + r.h / 2, G.save.stars[i] || 1, 10);
  }
  drawBtn(ctx, backBtn, t('back'), PAL.dim, false);
}

function renderPlay() {
  drawBackdrop(ctx);
  const s = G.play;
  drawHud(ctx, s);
  drawBoard(ctx, s);
  drawBtn(ctx, UNDO_BTN, t('undo'), PAL.dim, s.history.length > 0 && !s.over);
  drawBtn(ctx, RESTART_BTN, t('restart'), PAL.dim, s.moves > 0);
  if (s.over) renderResultCard(s);
}

function renderResultCard(s) {
  ctx.globalAlpha = 0.74; px(ctx, 0, 0, 360, 480, PAL.bg0); ctx.globalAlpha = 1;
  px(ctx, cardX - 3, cardY - 3, cardW + 6, cardH + 6, PAL.ink);
  px(ctx, cardX, cardY, cardW, cardH, PAL.panel);
  px(ctx, cardX, cardY, cardW, 4, PAL.panelHi);
  const cx = cardX + cardW / 2;
  fillText(ctx, t('win'), cx, cardY + 38, 17, PAL.good);
  drawStars(ctx, cx, cardY + 84, stars(s.moves, s.level.depth), 22);
  fillText(ctx, t('moves') + ' ' + s.moves + '   ' + t('par') + ' ' + s.level.depth,
    cx, cardY + 122, 12, PAL.dim);
  const last = G.levelIndex >= LEVEL_COUNT - 1;
  if (!last) {
    drawBtn(ctx, rRetry, t('retry'), PAL.dim, false);
    drawBtn(ctx, rNext, t('next'), PAL.good, true);
  } else {
    drawBtn(ctx, rRetryWide, t('retry'), PAL.good, true);
  }
  drawBtn(ctx, rMenu, t('back'), PAL.dim, false);
}

// ---- transitions ---------------------------------------------------------
function startLevel(i) {
  G.levelIndex = i;
  G.play = newPlay(i);
  G.screen = 'play';
}
function afterShift() {
  const s = G.play;
  if (s.over && s.won) {
    G.save.cleared[G.levelIndex] = true;
    const st = stars(s.moves, s.level.depth);
    if (!(G.save.stars[G.levelIndex] >= st)) G.save.stars[G.levelIndex] = st;
    persist();
  }
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * 360, y: (e.clientY - r.top) / r.height * 480 };
}
let pointerStart = null;

function onTap(p) {
  if (G.screen === 'title') {
    if (hitR(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hitR(startBtn, p)) { G.screen = 'select'; return; }
  } else if (G.screen === 'select') {
    if (hitR(backBtn, p)) { G.screen = 'title'; return; }
    for (let i = 0; i < LEVEL_COUNT; i++) {
      if (hitR(levelRow(i), p) && unlocked(i)) { startLevel(i); return; }
    }
  } else if (G.screen === 'play') {
    const s = G.play;
    if (s.over) {
      const last = G.levelIndex >= LEVEL_COUNT - 1;
      if (hitR(rMenu, p)) { G.screen = 'select'; return; }
      if (!last) {
        if (hitR(rRetry, p)) { startLevel(G.levelIndex); return; }
        if (hitR(rNext, p)) { startLevel(G.levelIndex + 1); return; }
      } else if (hitR(rRetryWide, p)) { startLevel(G.levelIndex); return; }
      return;
    }
    if (hitR(UNDO_BTN, p)) { undo(s); return; }
    if (hitR(RESTART_BTN, p)) { restart(s); return; }
  }
}

function onSwipe(start, dx, dy) {
  if (G.screen !== 'play') return;
  const s = G.play;
  if (s.over) return;
  const g = gridGeom(s.cols, s.rows);
  const c = (start.x - g.ox) / g.cell | 0;
  const r = (start.y - g.oy) / g.cell | 0;
  if (start.x < g.ox || start.y < g.oy || c < 0 || r < 0 || c >= s.cols || r >= s.rows) return;
  if (Math.abs(dx) > Math.abs(dy)) {
    if (doShift(s, 'row', r, dx > 0 ? 1 : -1)) afterShift();
  } else {
    if (doShift(s, 'col', c, dy > 0 ? 1 : -1)) afterShift();
  }
}

canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); pointerStart = eventPos(e); });
canvas.addEventListener('pointerup', (e) => {
  e.preventDefault();
  if (!pointerStart) return;
  const up = eventPos(e), start = pointerStart;
  pointerStart = null;
  const dx = up.x - start.x, dy = up.y - start.y;
  if (Math.abs(dx) > 18 || Math.abs(dy) > 18) onSwipe(start, dx, dy);
  else onTap(start);
});

window.addEventListener('keydown', (e) => {
  if (G.screen === 'play' && G.play && !G.play.over) {
    if (e.key === 'u' || e.key === 'U') undo(G.play);
    else if (e.key === 'r' || e.key === 'R') restart(G.play);
  } else if (G.screen === 'play' && G.play && G.play.over && e.key === 'Enter') {
    const last = G.levelIndex >= LEVEL_COUNT - 1;
    startLevel(last ? G.levelIndex : G.levelIndex + 1);
  } else if (G.screen === 'title' && (e.key === ' ' || e.key === 'Enter')) {
    G.screen = 'select';
  }
});

// ---- loop ----------------------------------------------------------------
function frame(now) {
  if (G.screen === 'title') renderTitle(now);
  else if (G.screen === 'select') renderSelect();
  else if (G.screen === 'play') renderPlay();
  requestAnimationFrame(frame);
}

loadLang();
loadSave();
requestAnimationFrame(frame);
