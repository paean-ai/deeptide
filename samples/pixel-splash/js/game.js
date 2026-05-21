// Pixel Splash - screen flow, input, rendering, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-splash:save';
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

function hit(r, p) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

// ---- fixed button geometry ----------------------------------------------
const startBtn = { x: 96, y: 322, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };
const backBtn = { x: 110, y: 420, w: 140, h: 38 };
function levelRow(i) { return { x: 36, y: 92 + i * 52, w: 288, h: 44 }; }

const cardX = 44, cardY = 124, cardW = 272, cardH = 224;
const rRetry = { x: cardX + 18, y: cardY + 150, w: 118, h: 36 };
const rNext = { x: cardX + 136, y: cardY + 150, w: 118, h: 36 };
const rRetryWide = { x: cardX + 18, y: cardY + 150, w: 236, h: 36 };
const rMenu = { x: cardX + 18, y: cardY + 194, w: 236, h: 32 };

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 52, 30, PAL.text);
  fillText(ctx, t('subtitle'), 180, 82, 12, PAL.dim);
  drawBtn(ctx, startBtn, t('start'), PAL.good, true);
  drawBtn(ctx, langBtn, lang === 'en' ? '中文' : 'EN', PAL.warn, false);
  fillText(ctx, t('rules1'), 180, 392, 10, PAL.dim);
  fillText(ctx, t('rules2'), 180, 410, 10, PAL.dim);
  fillText(ctx, t('rules3'), 180, 428, 10, PAL.dim);
}

function renderSelect() {
  drawBackdrop(ctx);
  fillText(ctx, t('levelSelect'), 180, 56, 18, PAL.text);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelRow(i), open = unlocked(i), cfg = LEVELS[i];
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, open ? PAL.panel : PAL.bg0);
    px(ctx, r.x + 2, r.y + 2, 5, r.h - 4, open ? PAINTS[i % PAINTS.length] : PAL.panelHi);
    fillText(ctx, (i + 1) + '.', r.x + 26, r.y + r.h / 2, 13, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? cfg.name[lang === 'zh' ? 1 : 0].toUpperCase() : t('locked'),
      r.x + 48, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    fillText(ctx, cfg.n + '×' + cfg.n, r.x + r.w - 100, r.y + r.h / 2, 11, PAL.dim, 'left');
    if (open && G.save.cleared[i]) drawStars(ctx, r.x + r.w - 42, r.y + r.h / 2, G.save.stars[i] || 1, 10);
  }
  drawBtn(ctx, backBtn, t('back'), PAL.dim, false);
}

function renderPlay() {
  drawBackdrop(ctx);
  const s = G.play;
  drawHud(ctx, s);
  drawCanvasBoard(ctx, s);
  drawSwatches(ctx, s);
  drawBtn(ctx, UNDO_BTN, t('undo'), PAL.warn, s.history.length > 0 && !s.over);
  drawBtn(ctx, RESTART_BTN, t('restart'), PAL.dim, !s.over);
  if (s.over) renderResultCard(s);
}

function renderResultCard(s) {
  ctx.globalAlpha = 0.72;
  px(ctx, 0, 0, 360, 480, PAL.bg0);
  ctx.globalAlpha = 1;
  px(ctx, cardX - 3, cardY - 3, cardW + 6, cardH + 6, PAL.ink);
  px(ctx, cardX, cardY, cardW, cardH, PAL.panel);
  px(ctx, cardX, cardY, cardW, 4, PAL.panelHi);
  const cx = cardX + cardW / 2;
  fillText(ctx, s.won ? t('win') : t('lose'), cx, cardY + 40, 19,
    s.won ? PAL.good : PAL.bad);
  const last = G.levelIndex >= LEVEL_COUNT - 1;
  if (s.won) {
    drawStars(ctx, cx, cardY + 90, stars(s.moves, s.level.par), 18);
    fillText(ctx, t('moves') + ' ' + s.moves + '   ' + t('par') + ' ' + s.level.par,
      cx, cardY + 128, 12, PAL.dim);
  } else {
    fillText(ctx, t('moves') + ' ' + s.moves + ' / ' + s.level.budget,
      cx, cardY + 96, 13, PAL.dim);
  }
  if (s.won && !last) {
    drawBtn(ctx, rRetry, t('retry'), PAL.warn, false);
    drawBtn(ctx, rNext, t('next'), PAL.good, true);
  } else {
    drawBtn(ctx, rRetryWide, t('retry'), PAL.good, true);
  }
  drawBtn(ctx, rMenu, t('back'), PAL.dim, false);
}

// ---- transitions ---------------------------------------------------------
function startLevel(i) {
  G.levelIndex = i;
  G.play = newPlay(buildLevel(i));
  G.screen = 'play';
}
function onWin(s) {
  G.save.cleared[G.levelIndex] = true;
  const st = stars(s.moves, s.level.par);
  if (!(G.save.stars[G.levelIndex] >= st)) G.save.stars[G.levelIndex] = st;
  persist();
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) / r.width * 360,
    y: (e.clientY - r.top) / r.height * 480,
  };
}
function pickAndCheck(c) {
  const s = G.play;
  if (pickColor(s, c) && s.over && s.won) onWin(s);
}
function onTap(p) {
  if (G.screen === 'title') {
    if (hit(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hit(startBtn, p)) { G.screen = 'select'; return; }
  } else if (G.screen === 'select') {
    if (hit(backBtn, p)) { G.screen = 'title'; return; }
    for (let i = 0; i < LEVEL_COUNT; i++) {
      if (hit(levelRow(i), p) && unlocked(i)) { startLevel(i); return; }
    }
  } else if (G.screen === 'play') {
    const s = G.play;
    if (s.over) {
      const last = G.levelIndex >= LEVEL_COUNT - 1;
      if (hit(rMenu, p)) { G.screen = 'select'; return; }
      if (s.won && !last) {
        if (hit(rRetry, p)) { startLevel(G.levelIndex); return; }
        if (hit(rNext, p)) { startLevel(G.levelIndex + 1); return; }
      } else if (hit(rRetryWide, p)) { startLevel(G.levelIndex); return; }
      return;
    }
    if (hit(UNDO_BTN, p)) { undo(s); return; }
    if (hit(RESTART_BTN, p)) { restart(s); return; }
    const rects = swatchRects(s.level.colors);
    for (let c = 0; c < rects.length; c++) {
      if (hit(rects[c], p)) { pickAndCheck(c); return; }
    }
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); onTap(eventPos(e)); });

window.addEventListener('keydown', (e) => {
  if (G.screen === 'play' && G.play) {
    const s = G.play;
    if (!s.over) {
      if (e.key >= '1' && e.key <= '6') pickAndCheck(parseInt(e.key, 10) - 1);
      else if (e.key === 'u' || e.key === 'U') undo(s);
      else if (e.key === 'r' || e.key === 'R') restart(s);
    } else if (e.key === 'Enter') {
      const last = G.levelIndex >= LEVEL_COUNT - 1;
      if (s.won && !last) startLevel(G.levelIndex + 1);
      else startLevel(G.levelIndex);
    }
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
