// Pixel Duelist - all canvas drawing. Blocky pixel sprites + the timing track.

const PAL = {
  bg0: '#171320', bg1: '#241c34', floor: '#2e2545', floorHi: '#403258',
  hero: '#4f9be8', heroDk: '#2c5b92', heroHi: '#bfe2ff',
  boss: '#c64a55', bossDk: '#7c2832', bossHi: '#f0b8a0',
  steel: '#ccd3e0', steelDk: '#79829a',
  slash: '#5cc8ff', thrust: '#f2a13e',
  perfect: '#ffe27a', hurt: '#ff5d6c', exec: '#ff8a3c', good: '#9be88a',
  ink: '#0d0a12', panel: '#2a2238', panelHi: '#3c3254',
  text: '#f3f1e6', dim: '#9a92ac',
  heart: '#ff5d6c', heartDk: '#4a2530',
  posture: '#e0556b', postureGlow: '#ffd24a', barBg: '#1d1828',
};

// Fixed layout regions; game.js reads these for hit testing.
const LAYOUT = {
  track: { x: 28, y: 318, w: 304, h: 30 },
  parry: { x: 22, y: 362, w: 152, h: 104 },
  dodge: { x: 186, y: 362, w: 152, h: 104 },
  exec: { x: 22, y: 362, w: 316, h: 104 },
};

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }

function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}

// ---- backdrop ------------------------------------------------------------
function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
  // arena pillars
  for (let i = 0; i < 2; i++) {
    const x = i === 0 ? 16 : 320;
    px(ctx, x, 96, 24, 210, PAL.panel);
    px(ctx, x, 96, 24, 6, PAL.panelHi);
    for (let b = 0; b < 5; b++) px(ctx, x, 116 + b * 38, 24, 3, PAL.bg0);
  }
  // floor
  px(ctx, 0, 286, 360, 22, PAL.floor);
  px(ctx, 0, 286, 360, 3, PAL.floorHi);
  for (let x = 0; x < 360; x += 30) px(ctx, x, 289, 2, 17, PAL.bg0);
}

// ---- a blocky knight -----------------------------------------------------
// facing: +1 looks down-screen (hero), -1 looks up-screen (boss)
function drawKnight(ctx, cx, footY, u, body, dark, hi, opts) {
  opts = opts || {};
  const shake = opts.shake || 0;
  cx += shake;
  const lean = opts.lean || 0;             // body lean during a windup
  // legs
  px(ctx, cx - 5 * u, footY - 6 * u, 4 * u, 6 * u, dark);
  px(ctx, cx + 1 * u, footY - 6 * u, 4 * u, 6 * u, dark);
  // cloak behind torso
  px(ctx, cx - 7 * u, footY - 17 * u, 14 * u, 12 * u, opts.cloak || dark);
  // torso
  const tx = cx - 6 * u + lean;
  px(ctx, tx, footY - 18 * u, 12 * u, 11 * u, body);
  px(ctx, tx, footY - 18 * u, 12 * u, 2 * u, hi);
  px(ctx, tx + 5 * u, footY - 15 * u, 2 * u, 6 * u, dark);
  // shoulders
  px(ctx, tx - 2 * u, footY - 19 * u, 16 * u, 3 * u, dark);
  // head + helm
  const hx = cx - 4 * u + lean;
  px(ctx, hx, footY - 27 * u, 8 * u, 8 * u, PAL.steel);
  px(ctx, hx, footY - 27 * u, 8 * u, 2 * u, PAL.steelDk);
  px(ctx, hx + 1 * u, footY - 23 * u, 6 * u, 2 * u, PAL.ink);   // visor slit
  if (opts.crest) px(ctx, hx + 2 * u, footY - 30 * u, 4 * u, 3 * u, opts.crest);
}

// the weapon: an arc above the wielder, glow rising with windup progress p
function drawWeapon(ctx, cx, baseY, u, color, p, kind) {
  const lift = 4 + p * 9;
  if (kind === 'thrust') {
    // a forward spear, length grows
    const len = (3 + p * 9) * u;
    px(ctx, cx + 5 * u, baseY - len, 3 * u, len, color);
    px(ctx, cx + 4 * u, baseY - len - 2 * u, 5 * u, 3 * u, PAL.text);
  } else {
    // a raised blade arc
    px(ctx, cx + 4 * u, baseY - lift * u, 3 * u, (lift) * u, PAL.steel);
    px(ctx, cx + 1 * u, baseY - (lift + 3) * u, 9 * u, 3 * u, color);
  }
  if (p > 0.05) {
    ctx.globalAlpha = 0.25 + 0.45 * p;
    px(ctx, cx - 9 * u, baseY - (lift + 4) * u, 18 * u, 3 * u, color);
    ctx.globalAlpha = 1;
  }
}

// ---- duel scene ----------------------------------------------------------
function drawDuelScene(ctx, s, now) {
  drawBackdrop(ctx);
  const atk = s.atk;
  const p = atk ? Math.min(1, atk.t / atk.windup) : 0;
  const kindColor = atk ? (atk.kind === 'slash' ? PAL.slash : PAL.thrust) : PAL.dim;
  const fl = s.flash;
  const hurtShake = fl && fl.kind === 'hurt' ? Math.sin(now / 18) * 3 : 0;

  // foe (top)
  const bossY = 168, bossX = 180;
  const recoil = s.phase === 'stagger' ? 8 : 0;
  drawKnight(ctx, bossX, bossY + recoil, 3.4, PAL.boss, PAL.bossDk, PAL.bossHi,
    { lean: p * 4, cloak: PAL.bossDk, crest: PAL.exec });
  if (s.phase === 'attack') drawWeapon(ctx, bossX, bossY - 92, 3.4, kindColor, p, atk.kind);
  if (s.phase === 'stagger') {
    ctx.globalAlpha = 0.6 + Math.sin(now / 90) * 0.3;
    fillText(ctx, '! STAGGERED !', bossX, 92, 11, PAL.postureGlow);
    ctx.globalAlpha = 1;
  }

  // hero (bottom)
  const heroY = 286, heroX = 180;
  let heroOpts = { shake: hurtShake, cloak: PAL.heroDk, crest: PAL.heroHi };
  drawKnight(ctx, heroX, heroY, 2.7, PAL.hero, PAL.heroDk, PAL.heroHi, heroOpts);
  // hero guard spark on a successful parry/dodge
  if (fl && (fl.kind === 'parry' || fl.kind === 'dodge' || fl.kind === 'perfect')) {
    const a = fl.t / fl.dur;
    ctx.globalAlpha = a;
    const c = fl.kind === 'perfect' ? PAL.perfect : (fl.kind === 'dodge' ? PAL.thrust : PAL.slash);
    for (let i = 0; i < 6; i++) {
      const ang = (i / 6) * Math.PI * 2 + (1 - a) * 2;
      const r = 18 + (1 - a) * 26;
      px(ctx, heroX + Math.cos(ang) * r - 2, heroY - 34 + Math.sin(ang) * r - 2, 5, 5, c);
    }
    ctx.globalAlpha = 1;
  }
}

// ---- timing track --------------------------------------------------------
function drawTrack(ctx, s) {
  const T = LAYOUT.track;
  px(ctx, T.x - 3, T.y - 3, T.w + 6, T.h + 6, PAL.panel);
  px(ctx, T.x, T.y, T.w, T.h, PAL.barBg);

  if (s.phase === 'attack' && s.atk) {
    const atk = s.atk;
    const kc = atk.kind === 'slash' ? PAL.slash : PAL.thrust;
    const reactFrac = Math.min(1, s.cfg.react / atk.windup);
    const perfFrac = Math.min(1, PERFECT_WIN / atk.windup);
    // react window band, perfect sliver at the far right
    px(ctx, T.x + T.w * (1 - reactFrac), T.y, T.w * reactFrac, T.h, '#2f3a2c');
    px(ctx, T.x + T.w * (1 - perfFrac), T.y, T.w * perfFrac, T.h, '#4a4322');
    // strike line
    px(ctx, T.x + T.w - 3, T.y - 2, 3, T.h + 4, PAL.hurt);
    // travelling marker
    const p = Math.min(1, atk.t / atk.windup);
    const mx = T.x + T.w * p;
    px(ctx, mx - 4, T.y - 4, 8, T.h + 8, kc);
    px(ctx, mx - 2, T.y - 4, 4, T.h + 8, PAL.text);
  } else if (s.phase === 'stagger') {
    const f = Math.max(0, s.staggerT / STAGGER_WIN);
    px(ctx, T.x, T.y, T.w * f, T.h, PAL.exec);
    fillText(ctx, 'EXECUTE WINDOW', T.x + T.w / 2, T.y + T.h / 2, 11, PAL.ink);
  } else {
    fillText(ctx, '— READY —', T.x + T.w / 2, T.y + T.h / 2, 11, PAL.dim);
  }
}

// ---- HUD: foe name, foe HP, posture, hearts, combo -----------------------
function drawHud(ctx, s) {
  const cfg = s.cfg;
  fillText(ctx, cfg.name[lang === 'zh' ? 1 : 0].toUpperCase(), 180, 18, 13, PAL.text);
  // foe HP pips
  for (let i = 0; i < cfg.hp; i++) {
    const x = 180 - cfg.hp * 9 + i * 18;
    px(ctx, x, 30, 14, 8, i < s.bossHP ? PAL.boss : PAL.barBg);
    if (i < s.bossHP) px(ctx, x, 30, 14, 2, PAL.bossHi);
  }
  // posture bar
  const pw = 220, px0 = 180 - pw / 2;
  px(ctx, px0 - 2, 44, pw + 4, 12, PAL.panel);
  px(ctx, px0, 46, pw, 8, PAL.barBg);
  const pf = Math.min(1, s.posture / cfg.postureMax);
  px(ctx, px0, 46, pw * pf, 8, pf >= 1 ? PAL.postureGlow : PAL.posture);
  fillText(ctx, 'POSTURE', 180, 50, 8, PAL.dim);

  // hero hearts
  for (let i = 0; i < PLAYER_HP; i++) {
    const x = 14 + i * 17;
    const on = i < s.playerHP;
    px(ctx, x, 64, 12, 6, on ? PAL.heart : PAL.heartDk);
    px(ctx, x + 2, 62, 8, 3, on ? PAL.heart : PAL.heartDk);
  }
  // combo
  if (s.combo > 1) {
    fillText(ctx, s.combo + ' ' + t('combo'), 346, 67, 11, PAL.perfect, 'right');
  }
}

// ---- a soft full-screen flash for hits / perfects ------------------------
function drawFlash(ctx, s) {
  const fl = s.flash;
  if (!fl) return;
  const a = fl.t / fl.dur;
  if (fl.kind === 'hurt') {
    ctx.globalAlpha = a * 0.5;
    px(ctx, 0, 0, 360, 480, PAL.hurt);
    ctx.globalAlpha = 1;
  } else if (fl.kind === 'perfect') {
    ctx.globalAlpha = a;
    fillText(ctx, t('perfect') + '!', 180, 250, 26, PAL.perfect);
    ctx.globalAlpha = 1;
  } else if (fl.kind === 'execute') {
    ctx.globalAlpha = a * 0.6;
    px(ctx, 0, 0, 360, 480, PAL.exec);
    ctx.globalAlpha = 1;
  }
}

// ---- title art -----------------------------------------------------------
function drawTitleArt(ctx, now) {
  drawBackdrop(ctx);
  const sway = Math.sin(now / 520) * 4;
  drawKnight(ctx, 116 + sway, 250, 3.0, PAL.hero, PAL.heroDk, PAL.heroHi,
    { cloak: PAL.heroDk, crest: PAL.heroHi });
  drawKnight(ctx, 244 - sway, 250, 3.0, PAL.boss, PAL.bossDk, PAL.bossHi,
    { cloak: PAL.bossDk, crest: PAL.exec });
  // crossed blades between them
  px(ctx, 150, 150, 60, 6, PAL.steel);
  px(ctx, 150, 170, 60, 6, PAL.slash);
  ctx.globalAlpha = 0.5 + Math.sin(now / 240) * 0.3;
  px(ctx, 150, 160, 60, 4, PAL.thrust);
  ctx.globalAlpha = 1;
}
