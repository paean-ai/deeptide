// Pixel Fillomino - screen flow, tap input, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-fillomino:save';
const G = {
  screen: 'title',          // title | select | play
  play: null,
  levelIndex: 0,
  sel: -1,                  // selected cell index
  save: { cleared: [] },
};

function loadSave() {
  try {
    const o = JSON.parse(localStorage.getItem(SAVE_KEY) || '{}');
    if (Array.isArray(o.cleared)) G.save.cleared = o.cleared;
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
const backBtn = { x: 110, y: 424, w: 140, h: 38 };
function levelRow(i) { return { x: 36, y: 92 + i * 52, w: 288, h: 44 }; }
const cardX = 44, cardY = 134, cardW = 272, cardH = 200;
const rRetry = { x: cardX + 18, y: cardY + 110, w: 118, h: 36 };
const rNext = { x: cardX + 136, y: cardY + 110, w: 118, h: 36 };
const rRetryWide = { x: cardX + 18, y: cardY + 110, w: 236, h: 36 };
const rMenu = { x: cardX + 18, y: cardY + 154, w: 236, h: 32 };

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 56, 25, PAL.text);
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
    fillText(ctx, (i + 1) + '.', r.x + 24, r.y + r.h / 2, 12, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? L(lv.name).toUpperCase() : t('locked'),
      r.x + 46, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    fillText(ctx, lv.n + '×' + lv.n, r.x + r.w - 96, r.y + r.h / 2, 11, PAL.dim, 'left');
    if (open && G.save.cleared[i]) fillText(ctx, '✓', r.x + r.w - 30, r.y + r.h / 2, 16, PAL.good);
  }
  drawBtn(ctx, backBtn, t('back'), PAL.dim, false);
}

function renderPlay() {
  drawBackdrop(ctx);
  const s = G.play;
  drawHud(ctx, s);
  drawBoard(ctx, s, G.sel);
  drawNumPad(ctx, s);
  drawBtn(ctx, ERASE_BTN, t('erase'), PAL.dim, true);
  drawBtn(ctx, RESTART_BTN, t('restart'), PAL.dim, true);
  if (s.over) renderResultCard(s);
}

function renderResultCard(s) {
  ctx.globalAlpha = 0.76; px(ctx, 0, 0, 360, 480, PAL.bg0); ctx.globalAlpha = 1;
  px(ctx, cardX - 3, cardY - 3, cardW + 6, cardH + 6, PAL.ink);
  px(ctx, cardX, cardY, cardW, cardH, PAL.panel);
  px(ctx, cardX, cardY, cardW, 4, PAL.panelHi);
  const cx = cardX + cardW / 2;
  fillText(ctx, t('win'), cx, cardY + 44, 19, PAL.good);
  drawStars(ctx, cx, cardY + 80, 3, 18);
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
  G.sel = -1;
  G.screen = 'play';
}
function onWin() {
  G.save.cleared[G.levelIndex] = true;
  persist();
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * 360, y: (e.clientY - r.top) / r.height * 480 };
}
function apply(val) {
  const s = G.play;
  if (G.sel < 0) return;
  if (setCell(s, G.sel, val) && s.over && s.won) onWin();
}
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
    if (hitR(ERASE_BTN, p)) { apply(0); return; }
    if (hitR(RESTART_BTN, p)) { restart(s); G.sel = -1; return; }
    for (let v = 1; v <= MAX_NUM; v++) {
      if (hitR(numBtn(v - 1), p)) { apply(v); return; }
    }
    const g = gridGeom(s.n);
    if (p.x >= g.ox && p.x < g.ox + g.bpx && p.y >= g.oy && p.y < g.oy + g.bpx) {
      const c = (p.x - g.ox) / g.cell | 0, r = (p.y - g.oy) / g.cell | 0;
      const idx = r * s.n + c;
      G.sel = isGiven(s, idx) ? -1 : idx;
    }
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); onTap(eventPos(e)); });

window.addEventListener('keydown', (e) => {
  if (G.screen === 'play' && G.play && !G.play.over) {
    if (e.key >= '1' && e.key <= '8') apply(parseInt(e.key, 10));
    else if (e.key === '0' || e.key === 'Backspace' || e.key === 'Delete') apply(0);
    else if (e.key === 'r' || e.key === 'R') { restart(G.play); G.sel = -1; }
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
