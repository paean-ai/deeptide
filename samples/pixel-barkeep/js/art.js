// Pixel-art rendering for Pixel Barkeep. 360x480 world units.

const PALETTE = {
  wall:     '#2c2330',
  wallHi:   '#3c3142',
  counter:  '#7a4e28',
  counterHi:'#9c6a3a',
  counterDk:'#522f15',
  rail:     '#caa05a',
  barkeep:  '#e8d8c0',
  apron:    '#c2453f',
  apronHi:  '#e0685f',
  skin:     '#e8b890',
  mugFull:  '#e7a93f',
  mugFoam:  '#f6ecd0',
  mugEmpty: '#9aa6b8',
  mugGlass: '#cdd6e2',
  hud:      '#15101a',
  hudText:  '#f3eee0',
  heart:    '#ff5a6a',
  accent:   '#f4c44a',
  text:     '#e8e2d0',
};

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.wall;
  ctx.fillRect(0, 0, VW, VH);
  // Faint panelling.
  ctx.fillStyle = PALETTE.wallHi;
  for (let x = 16; x < VW; x += 48) ctx.fillRect(x, 32, 2, VH - 32);
  // Counters.
  for (let i = 0; i < LANES; i++) {
    const cy = laneCenterY(i);
    ctx.fillStyle = PALETTE.counterDk;
    ctx.fillRect(0, cy + 14, BAR_X + 26, 22);
    ctx.fillStyle = PALETTE.counter;
    ctx.fillRect(0, cy + 14, BAR_X + 26, 18);
    ctx.fillStyle = PALETTE.counterHi;
    ctx.fillRect(0, cy + 14, BAR_X + 26, 3);
    // Sliding rail the mugs run along.
    ctx.fillStyle = PALETTE.rail;
    ctx.fillRect(0, cy + 12, BAR_X + 26, 2);
  }
  // The bar end where the barkeep stands.
  ctx.fillStyle = PALETTE.counterDk;
  ctx.fillRect(BAR_X + 18, 32, VW - BAR_X - 18, VH - 32);
}

function drawPatron(ctx, p) {
  const cy = laneCenterY(p.lane);
  const k = KINDS[p.kind];
  const x = p.x, footY = cy + 14;
  // Legs.
  ctx.fillStyle = '#2a2230';
  ctx.fillRect((x + 5) | 0, footY - 8, 5, 8);
  ctx.fillRect((x + 14) | 0, footY - 8, 5, 8);
  // Body.
  ctx.fillStyle = k.color;
  ctx.fillRect((x + 3) | 0, footY - 26, 18, 19);
  ctx.fillStyle = 'rgba(255,255,255,0.18)';
  ctx.fillRect((x + 3) | 0, footY - 26, 18, 3);
  // Head.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect((x + 7) | 0, footY - 38, 11, 11);
  ctx.fillStyle = '#1d1518';
  ctx.fillRect((x + 8) | 0, footY - 41, 9, 4);   // hair
  // Eye looking toward the bar.
  ctx.fillRect((x + 14) | 0, footY - 33, 2, 2);
}

function drawMug(ctx, m) {
  const cy = laneCenterY(m.lane);
  const y = cy + 2;
  const full = m.dir < 0;
  ctx.fillStyle = full ? '#6a3f12' : '#5a6472';
  ctx.fillRect(m.x | 0, (y - 9) | 0, MUG_W, 14);
  ctx.fillStyle = full ? PALETTE.mugFull : PALETTE.mugGlass;
  ctx.fillRect((m.x + 1) | 0, (y - 8) | 0, MUG_W - 2, 12);
  if (full) {
    ctx.fillStyle = PALETTE.mugFoam;
    ctx.fillRect((m.x + 1) | 0, (y - 8) | 0, MUG_W - 2, 3);
  }
  // Handle.
  ctx.strokeStyle = full ? '#6a3f12' : '#5a6472';
  ctx.lineWidth = 2;
  ctx.strokeRect((m.x + MUG_W - 1) | 0, (y - 6) | 0, 4, 8);
}

function drawBarkeep(ctx, s) {
  const cy = laneCenterY(s.barkeepLane);
  const x = BAR_X + 6, footY = cy + 14;
  ctx.fillStyle = '#2a2230';
  ctx.fillRect((x + 4) | 0, footY - 8, 5, 8);
  ctx.fillRect((x + 13) | 0, footY - 8, 5, 8);
  // Apron body.
  ctx.fillStyle = PALETTE.apron;
  ctx.fillRect((x + 2) | 0, footY - 28, 19, 21);
  ctx.fillStyle = PALETTE.apronHi;
  ctx.fillRect((x + 2) | 0, footY - 28, 19, 3);
  // Arm reaching to the counter.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect((x - 4) | 0, footY - 20, 8, 5);
  // Head.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect((x + 6) | 0, footY - 40, 12, 12);
  ctx.fillStyle = '#3a2a1a';
  ctx.fillRect((x + 5) | 0, footY - 43, 14, 4);
  // Pour-ready glint.
  if (s.pourCD <= 0 && !s.over) {
    ctx.fillStyle = PALETTE.accent;
    ctx.fillRect((x - 6) | 0, footY - 19, 3, 3);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 30);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 11px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText(t(lang, 'round') + ' ' + s.round, 8, 15);
  for (let i = 0; i < s.lives; i++) drawHeart(ctx, 96 + i * 13, 15);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 8, 15);
}

function drawHeart(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.heart;
  ctx.fillRect(cx - 4, cy - 3, 3, 3);
  ctx.fillRect(cx + 1, cy - 3, 3, 3);
  ctx.fillRect(cx - 4, cy, 8, 2);
  ctx.fillRect(cx - 3, cy + 2, 6, 1);
  ctx.fillRect(cx - 1, cy + 3, 2, 1);
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  ctx.fillStyle = 'rgba(255,90,74,' + Math.min(0.45, s.flash) + ')';
  ctx.fillRect(0, 30, VW, VH - 30);
}

function drawRoundBanner(ctx, lang, s) {
  if (s.roundBanner <= 0) return;
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 20px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'round') + ' ' + s.round, VW / 2, 250);
}

function drawScene(ctx, lang, s, best) {
  drawBackdrop(ctx);
  for (const m of s.mugs) drawMug(ctx, m);
  for (const p of s.patrons) drawPatron(ctx, p);
  drawBarkeep(ctx, s);
  drawFlash(ctx, s);
  drawRoundBanner(ctx, lang, s);
  drawHud(ctx, lang, s, best);
}

// A row of mugs sliding along a counter, for the title screen.
function drawTitleArt(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.counterDk;
  ctx.fillRect(cx - 120, cy + 14, 240, 20);
  ctx.fillStyle = PALETTE.counter;
  ctx.fillRect(cx - 120, cy + 14, 240, 16);
  ctx.fillStyle = PALETTE.rail;
  ctx.fillRect(cx - 120, cy + 12, 240, 2);
  for (let i = 0; i < 4; i++) {
    const mx = cx - 96 + i * 52, y = cy + 2;
    ctx.fillStyle = '#6a3f12';
    ctx.fillRect(mx, y - 9, MUG_W, 14);
    ctx.fillStyle = PALETTE.mugFull;
    ctx.fillRect(mx + 1, y - 8, MUG_W - 2, 12);
    ctx.fillStyle = PALETTE.mugFoam;
    ctx.fillRect(mx + 1, y - 8, MUG_W - 2, 3);
  }
}
