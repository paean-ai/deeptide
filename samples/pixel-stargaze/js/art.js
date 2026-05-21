// Pixel Stargaze - all canvas drawing: night sky, shop list, title.

const PAL = {
  sky0: '#0a0b1f', sky1: '#1c1a3a', ground: '#141228',
  panel: '#221f3a', panelHi: '#322e50', row: '#1c1a32', ink: '#080711',
  text: '#f3f1e6', dim: '#8f88a8', gold: '#ffe27a', goldDk: '#9c7d1c',
  good: '#7bd88f', lit: '#bfe0ff', star: '#fff6d8', renown: '#ffc24a',
  buy: '#5fc06e', buyOff: '#2f3a44', tab: '#2a2748', tabOn: '#4a4378',
};

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }
function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}

// compact large-number formatting: 1234 -> 1.23K, 4.5e7 -> 45.0M
function fmtNum(n) {
  if (!isFinite(n)) return '∞';
  if (n < 1000) return n < 10 ? n.toFixed(1) : Math.floor(n).toString();
  const u = ['K', 'M', 'B', 'T', 'Qa', 'Qi', 'Sx', 'Sp'];
  let i = -1;
  while (n >= 1000 && i < u.length - 1) { n /= 1000; i++; }
  return (n < 10 ? n.toFixed(2) : n.toFixed(1)) + u[i];
}

// ---- layout --------------------------------------------------------------
const SKY = { x: 0, y: 0, w: 360, h: 154 };
const TAB_SCOPES = { x: 10, y: 160, w: 168, h: 28 };
const TAB_RESEARCH = { x: 182, y: 160, w: 168, h: 28 };
function scopeRow(i) { return { x: 10, y: 194 + i * 44, w: 340, h: 40 }; }
function resRow(i) { return { x: 10, y: 194 + i * 27, w: 340, h: 24 }; }
function rowBuyBtn(r) { return { x: r.x + r.w - 84, y: r.y + (r.h - 24) / 2, w: 80, h: 24 }; }
const PUBLISH_BTN = { x: 56, y: 426, w: 248, h: 40 };

// fixed twinkle layout for the starfield
const STARS = [];
(function () {
  let s = 90210;
  for (let i = 0; i < 46; i++) {
    s = (s * 16807) % 2147483647;
    const x = (s % 356) + 2;
    s = (s * 16807) % 2147483647;
    const y = (s % 118) + 4;
    s = (s * 16807) % 2147483647;
    STARS.push({ x, y, ph: s % 1000, big: (s % 5) === 0 });
  }
})();

function drawSky(ctx, s, now) {
  const g = ctx.createLinearGradient(0, 0, 0, SKY.h);
  g.addColorStop(0, PAL.sky0); g.addColorStop(1, PAL.sky1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, SKY.w, SKY.h);
  for (const st of STARS) {
    const tw = 0.45 + 0.55 * Math.abs(Math.sin((now + st.ph) / 620));
    ctx.globalAlpha = tw;
    const sz = st.big ? 3 : 2;
    px(ctx, st.x, st.y, sz, sz, PAL.star);
    ctx.globalAlpha = 1;
  }
}

// ---- game screen pieces --------------------------------------------------
function drawHud(ctx, s, now) {
  drawSky(ctx, s, now);
  fillText(ctx, t('light'), 180, 44, 11, PAL.dim);
  fillText(ctx, fmtNum(s.light), 180, 70, 30, PAL.lit);
  fillText(ctx, '+' + fmtNum(rate(s)) + ' /s', 180, 96, 12, PAL.good);
  fillText(ctx, t('tapHint'), 180, 138, 9, PAL.dim);
  // renown badge
  px(ctx, 268, 10, 84, 24, PAL.ink);
  px(ctx, 270, 12, 80, 20, PAL.panel);
  fillText(ctx, '✦ ' + s.totalRenown + ' ' + t('renown'), 310, 22, 9, PAL.renown);
}

function drawTab(ctx, r, label, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? PAL.tabOn : PAL.tab);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, 12, active ? PAL.text : PAL.dim);
}

function drawScopeRow(ctx, s, i) {
  const r = scopeRow(i), ts = TELESCOPES[i];
  px(ctx, r.x, r.y, r.w, r.h, PAL.row);
  px(ctx, r.x, r.y, 4, r.h, PAL.lit);
  fillText(ctx, L(ts.name).toUpperCase(), r.x + 12, r.y + 13, 12, PAL.text, 'left');
  fillText(ctx, 'x' + s.scopes[i] + '   +' + fmtNum(ts.output * tierMult(s, i) * globalMult(s)) + '/s',
    r.x + 12, r.y + 29, 9, PAL.dim, 'left');
  const cost = scopeCost(s, i), can = s.light >= cost;
  const b = rowBuyBtn(r);
  px(ctx, b.x, b.y, b.w, b.h, can ? PAL.buy : PAL.buyOff);
  fillText(ctx, fmtNum(cost), b.x + b.w / 2, b.y + 9, 9, can ? PAL.ink : PAL.dim);
  fillText(ctx, t('buy'), b.x + b.w / 2, b.y + 18, 8, can ? PAL.ink : PAL.dim);
}

function drawResRow(ctx, s, i) {
  const r = resRow(i), rs = RESEARCH[i], bought = researchBought(s, rs.key);
  px(ctx, r.x, r.y, r.w, r.h, PAL.row);
  px(ctx, r.x, r.y, 4, r.h, bought ? PAL.good : PAL.gold);
  fillText(ctx, L(rs.name), r.x + 12, r.y + r.h / 2, 10, bought ? PAL.dim : PAL.text, 'left');
  const b = rowBuyBtn(r);
  if (bought) {
    fillText(ctx, '✓ ' + t('done'), b.x + b.w / 2, b.y + b.h / 2, 9, PAL.good);
  } else {
    const can = s.light >= rs.cost;
    px(ctx, b.x, b.y, b.w, b.h, can ? PAL.gold : PAL.buyOff);
    fillText(ctx, fmtNum(rs.cost), b.x + b.w / 2, b.y + b.h / 2, 9, can ? PAL.ink : PAL.dim);
  }
}

function drawPublish(ctx, s) {
  const can = canPublish(s);
  px(ctx, PUBLISH_BTN.x, PUBLISH_BTN.y, PUBLISH_BTN.w, PUBLISH_BTN.h, PAL.ink);
  px(ctx, PUBLISH_BTN.x + 2, PUBLISH_BTN.y + 2, PUBLISH_BTN.w - 4, PUBLISH_BTN.h - 4,
    can ? PAL.renown : PAL.panel);
  const cx = PUBLISH_BTN.x + PUBLISH_BTN.w / 2;
  if (can) {
    fillText(ctx, t('publish') + '  ✦ +' + renownFor(s), cx, PUBLISH_BTN.y + 20, 14, PAL.ink);
  } else {
    fillText(ctx, t('publish') + '  ' + fmtNum(s.lifetime) + ' / ' + fmtNum(PUBLISH_MIN),
      cx, PUBLISH_BTN.y + 20, 10, PAL.dim);
  }
}

// ---- title art -----------------------------------------------------------
function drawTitleArt(ctx, now) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.sky0); g.addColorStop(1, PAL.sky1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
  for (const st of STARS) {
    const tw = 0.4 + 0.6 * Math.abs(Math.sin((now + st.ph) / 560));
    ctx.globalAlpha = tw;
    px(ctx, st.x, st.y + 60, st.big ? 3 : 2, st.big ? 3 : 2, PAL.star);
    ctx.globalAlpha = 1;
  }
  // a telescope silhouette
  const bx = 180, by = 300;
  px(ctx, bx - 4, by, 8, 36, '#3a3556');
  px(ctx, bx - 16, by + 34, 32, 5, '#3a3556');
  ctx.save();
  ctx.translate(bx, by);
  ctx.rotate(-0.6);
  px(ctx, -6, -52, 12, 52, '#4a4576');
  px(ctx, -8, -56, 16, 8, PAL.lit);
  ctx.restore();
}
