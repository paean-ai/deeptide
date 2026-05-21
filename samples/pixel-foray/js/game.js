// Pixel Foray - screen flow, tap / keyboard input, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-foray:save';
const G = {
  screen: 'title',          // title | select | play
  play: null,
  roomIndex: 0,
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
const startBtn = { x: 96, y: 318, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };
const backBtn = { x: 110, y: 424, w: 140, h: 38 };
function roomRow(i) { return { x: 36, y: 92 + i * 52, w: 288, h: 44 }; }
const cardX = 44, cardY = 128, cardW = 272, cardH = 222;
const rRetry = { x: cardX + 18, y: cardY + 150, w: 118, h: 36 };
const rNext = { x: cardX + 136, y: cardY + 150, w: 118, h: 36 };
const rRetryWide = { x: cardX + 18, y: cardY + 150, w: 236, h: 36 };
const rMenu = { x: cardX + 18, y: cardY + 192, w: 236, h: 32 };

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 50, 28, PAL.text);
  fillText(ctx, t('subtitle'), 180, 80, 11, PAL.dim);
  drawBtn(ctx, startBtn, t('start'), PAL.good, true);
  drawBtn(ctx, langBtn, lang === 'en' ? '中文' : 'EN', PAL.star, false);
  fillText(ctx, t('rules1'), 180, 384, 9, PAL.dim);
  fillText(ctx, t('rules2'), 180, 402, 9, PAL.dim);
  fillText(ctx, t('rules3'), 180, 420, 9, PAL.dim);
}

function renderSelect() {
  drawBackdrop(ctx);
  fillText(ctx, t('roomSelect'), 180, 56, 18, PAL.text);
  for (let i = 0; i < ROOM_COUNT; i++) {
    const r = roomRow(i), open = unlocked(i), room = ROOMS[i];
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, open ? PAL.panel : PAL.bg0);
    fillText(ctx, (i + 1) + '.', r.x + 22, r.y + r.h / 2, 12, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? L(room.name).toUpperCase() : t('locked'),
      r.x + 44, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    fillText(ctx, room.enemies.length + ' ' + t('foes'), r.x + r.w - 108, r.y + r.h / 2, 10, PAL.dim, 'left');
    if (open && G.save.cleared[i]) drawStars(ctx, r.x + r.w - 42, r.y + r.h / 2, G.save.stars[i] || 1, 10);
  }
  drawBtn(ctx, backBtn, t('back'), PAL.dim, false);
}

function renderPlay() {
  const s = G.play;
  drawRoom(ctx, s);
  drawHud(ctx, s);
  drawBtn(ctx, RESTART_BTN, t('restart'), PAL.dim, !s.over);
  if (s.over) renderResultCard(s);
}

function renderResultCard(s) {
  ctx.globalAlpha = 0.74; px(ctx, 0, 0, 360, 480, PAL.bg0); ctx.globalAlpha = 1;
  px(ctx, cardX - 3, cardY - 3, cardW + 6, cardH + 6, PAL.ink);
  px(ctx, cardX, cardY, cardW, cardH, PAL.panel);
  px(ctx, cardX, cardY, cardW, 4, PAL.panelHi);
  const cx = cardX + cardW / 2;
  fillText(ctx, s.won ? t('win') : t('lose'), cx, cardY + 40, 18, s.won ? PAL.good : PAL.heart);
  const last = G.roomIndex >= ROOM_COUNT - 1;
  if (s.won) {
    drawStars(ctx, cx, cardY + 88, stars(s), 22);
    fillText(ctx, s.hero.hp + ' / ' + HERO_HP + ' HP left', cx, cardY + 126, 12, PAL.dim);
  } else {
    fillText(ctx, 'the foes overran you', cx, cardY + 94, 12, PAL.dim);
  }
  if (s.won && !last) {
    drawBtn(ctx, rRetry, t('retry'), PAL.dim, false);
    drawBtn(ctx, rNext, t('next'), PAL.good, true);
  } else {
    drawBtn(ctx, rRetryWide, t('retry'), PAL.good, true);
  }
  drawBtn(ctx, rMenu, t('back'), PAL.dim, false);
}

// ---- transitions ---------------------------------------------------------
function startRoom(i) {
  G.roomIndex = i;
  G.play = newRoom(i);
  G.screen = 'play';
}
function afterAction() {
  const s = G.play;
  if (s.over && s.won) {
    G.save.cleared[G.roomIndex] = true;
    const st = stars(s);
    if (!(G.save.stars[G.roomIndex] >= st)) G.save.stars[G.roomIndex] = st;
    persist();
  }
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * 360, y: (e.clientY - r.top) / r.height * 480 };
}
function onTap(p) {
  if (G.screen === 'title') {
    if (hitR(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hitR(startBtn, p)) { G.screen = 'select'; return; }
  } else if (G.screen === 'select') {
    if (hitR(backBtn, p)) { G.screen = 'title'; return; }
    for (let i = 0; i < ROOM_COUNT; i++) {
      if (hitR(roomRow(i), p) && unlocked(i)) { startRoom(i); return; }
    }
  } else if (G.screen === 'play') {
    const s = G.play;
    if (s.over) {
      const last = G.roomIndex >= ROOM_COUNT - 1;
      if (hitR(rMenu, p)) { G.screen = 'select'; return; }
      if (s.won && !last) {
        if (hitR(rRetry, p)) { startRoom(G.roomIndex); return; }
        if (hitR(rNext, p)) { startRoom(G.roomIndex + 1); return; }
      } else if (hitR(rRetryWide, p)) { startRoom(G.roomIndex); return; }
      return;
    }
    if (hitR(RESTART_BTN, p)) { startRoom(G.roomIndex); return; }
    const cell = cellAt(p);
    if (cell >= 0 && heroTap(s, cell)) afterAction();
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); onTap(eventPos(e)); });

window.addEventListener('keydown', (e) => {
  if (G.screen === 'play' && G.play && !G.play.over) {
    const s = G.play, hr = s.hero.cell, r = hr / 7 | 0, c = hr % 7;
    let target = -1;
    if (e.key === 'ArrowUp' && r > 0) target = hr - 7;
    else if (e.key === 'ArrowDown' && r < 6) target = hr + 7;
    else if (e.key === 'ArrowLeft' && c > 0) target = hr - 1;
    else if (e.key === 'ArrowRight' && c < 6) target = hr + 1;
    else if (e.key === ' ') target = hr;          // wait
    else if (e.key === 'r' || e.key === 'R') { startRoom(G.roomIndex); return; }
    if (target >= 0) { if (heroTap(s, target)) afterAction(); e.preventDefault(); }
  } else if (G.screen === 'play' && G.play && G.play.over && e.key === 'Enter') {
    const last = G.roomIndex >= ROOM_COUNT - 1;
    startRoom(G.play.won && !last ? G.roomIndex + 1 : G.roomIndex);
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
