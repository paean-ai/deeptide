// Pixel Stargaze - screen flow, idle loop, input, save + offline accrual.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-stargaze:save';
const G = {
  screen: 'title',          // title | game
  state: null,
  tab: 'scopes',
  fx: [],
  away: 0,                  // offline Light to show in the popup
  saveAcc: 0,
};

function persist() {
  try {
    const s = G.state;
    localStorage.setItem(SAVE_KEY, JSON.stringify({
      light: s.light, lifetime: s.lifetime, totalRenown: s.totalRenown,
      scopes: s.scopes, research: s.research, taps: s.taps, ts: Date.now(),
    }));
  } catch (e) { /* ignore */ }
}
function loadGame() {
  const s = newGame();
  try {
    const raw = localStorage.getItem(SAVE_KEY);
    if (raw) {
      const o = JSON.parse(raw);
      if (typeof o.light === 'number') s.light = o.light;
      if (typeof o.lifetime === 'number') s.lifetime = o.lifetime;
      if (typeof o.totalRenown === 'number') s.totalRenown = o.totalRenown;
      if (Array.isArray(o.scopes) && o.scopes.length === TELESCOPE_COUNT) s.scopes = o.scopes;
      if (o.research && typeof o.research === 'object') s.research = o.research;
      if (typeof o.taps === 'number') s.taps = o.taps;
      if (typeof o.ts === 'number') {
        const sec = (Date.now() - o.ts) / 1000;
        if (sec > 30) G.away = applyOffline(s, sec);
      }
    }
  } catch (e) { /* fresh game */ }
  G.state = s;
}

function hitR(r, p) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

// ---- away popup geometry -------------------------------------------------
const awayCard = { x: 50, y: 168, w: 260, h: 144 };
const awayBtn = { x: awayCard.x + 50, y: awayCard.y + 96, w: 160, h: 34 };

// ---- title geometry ------------------------------------------------------
const startBtn = { x: 96, y: 360, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };

// ---- render --------------------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 110, 26, PAL.text);
  fillText(ctx, t('subtitle'), 180, 138, 10, PAL.dim);
  px(ctx, startBtn.x, startBtn.y, startBtn.w, startBtn.h, PAL.ink);
  px(ctx, startBtn.x + 2, startBtn.y + 2, startBtn.w - 4, startBtn.h - 4, PAL.good);
  fillText(ctx, t('start'), startBtn.x + startBtn.w / 2, startBtn.y + startBtn.h / 2, 17, PAL.ink);
  px(ctx, langBtn.x, langBtn.y, langBtn.w, langBtn.h, PAL.ink);
  px(ctx, langBtn.x + 2, langBtn.y + 2, langBtn.w - 4, langBtn.h - 4, PAL.panel);
  fillText(ctx, lang === 'en' ? '中文' : 'EN', langBtn.x + langBtn.w / 2, langBtn.y + langBtn.h / 2, 12, PAL.gold);
  fillText(ctx, t('rules1'), 180, 426, 9, PAL.dim);
  fillText(ctx, t('rules2'), 180, 442, 9, PAL.dim);
  fillText(ctx, t('rules3'), 180, 458, 9, PAL.dim);
}

function renderGame(now) {
  px(ctx, 0, 0, 360, 480, PAL.ground);
  const s = G.state;
  drawHud(ctx, s, now);
  drawTab(ctx, TAB_SCOPES, t('scopesTab'), G.tab === 'scopes');
  drawTab(ctx, TAB_RESEARCH, t('researchTab'), G.tab === 'research');
  if (G.tab === 'scopes') {
    for (let i = 0; i < TELESCOPE_COUNT; i++) drawScopeRow(ctx, s, i);
  } else {
    for (let i = 0; i < RESEARCH.length; i++) drawResRow(ctx, s, i);
  }
  drawPublish(ctx, s);
  // floating tap numbers
  for (const f of G.fx) {
    ctx.globalAlpha = Math.max(0, f.life / 0.8);
    fillText(ctx, '+' + fmtNum(f.v), f.x, f.y - (0.8 - f.life) * 30, 12, PAL.lit);
    ctx.globalAlpha = 1;
  }
  if (G.away > 0) {
    ctx.globalAlpha = 0.72; px(ctx, 0, 0, 360, 480, PAL.sky0); ctx.globalAlpha = 1;
    px(ctx, awayCard.x - 3, awayCard.y - 3, awayCard.w + 6, awayCard.h + 6, PAL.ink);
    px(ctx, awayCard.x, awayCard.y, awayCard.w, awayCard.h, PAL.panel);
    px(ctx, awayCard.x, awayCard.y, awayCard.w, 4, PAL.panelHi);
    const cx = awayCard.x + awayCard.w / 2;
    fillText(ctx, t('awayTitle'), cx, awayCard.y + 32, 12, PAL.text);
    fillText(ctx, '+' + fmtNum(G.away), cx, awayCard.y + 64, 22, PAL.lit);
    px(ctx, awayBtn.x, awayBtn.y, awayBtn.w, awayBtn.h, PAL.ink);
    px(ctx, awayBtn.x + 2, awayBtn.y + 2, awayBtn.w - 4, awayBtn.h - 4, PAL.good);
    fillText(ctx, t('collect'), awayBtn.x + awayBtn.w / 2, awayBtn.y + awayBtn.h / 2, 13, PAL.ink);
  }
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * 360, y: (e.clientY - r.top) / r.height * 480 };
}
function addFx(x, y, v) { G.fx.push({ x, y, v, life: 0.8 }); }

function onTap(p) {
  if (G.screen === 'title') {
    if (hitR(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hitR(startBtn, p)) { G.screen = 'game'; return; }
    return;
  }
  const s = G.state;
  if (G.away > 0) {
    if (hitR(awayBtn, p)) G.away = 0;
    return;
  }
  if (hitR(SKY, p)) { const v = tap(s); addFx(p.x, p.y, v); persist(); return; }
  if (hitR(TAB_SCOPES, p)) { G.tab = 'scopes'; return; }
  if (hitR(TAB_RESEARCH, p)) { G.tab = 'research'; return; }
  if (hitR(PUBLISH_BTN, p)) {
    if (canPublish(s)) { publish(s); G.tab = 'scopes'; persist(); }
    return;
  }
  if (G.tab === 'scopes') {
    for (let i = 0; i < TELESCOPE_COUNT; i++) {
      if (hitR(rowBuyBtn(scopeRow(i)), p)) { if (buyScope(s, i)) persist(); return; }
    }
  } else {
    for (let i = 0; i < RESEARCH.length; i++) {
      if (hitR(rowBuyBtn(resRow(i)), p)) { if (buyResearch(s, RESEARCH[i].key)) persist(); return; }
    }
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); onTap(eventPos(e)); });
window.addEventListener('keydown', (e) => {
  if (G.screen === 'title' && (e.key === ' ' || e.key === 'Enter')) G.screen = 'game';
  else if (G.screen === 'game' && G.away === 0 && (e.key === ' ')) {
    const v = tap(G.state); addFx(180, 110, v); e.preventDefault();
  }
});

// ---- loop ----------------------------------------------------------------
let last = 0;
function frame(now) {
  const dt = last ? Math.min(0.25, (now - last) / 1000) : 0;
  last = now;
  if (G.screen === 'game') {
    tick(G.state, dt);
    for (let i = G.fx.length - 1; i >= 0; i--) {
      G.fx[i].life -= dt;
      if (G.fx[i].life <= 0) G.fx.splice(i, 1);
    }
    G.saveAcc += dt;
    if (G.saveAcc >= 4) { G.saveAcc = 0; persist(); }
    renderGame(now);
  } else {
    renderTitle(now);
  }
  requestAnimationFrame(frame);
}

loadLang();
loadGame();
requestAnimationFrame(frame);
