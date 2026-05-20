// Pixel-art rendering for Pixel Bowl. 360x480 world units.

const PALETTE = {
  bg:        '#1d2240',
  laneEdge:  '#2c3252',
  lane:      '#c79b5f',
  laneShade: '#a4793f',
  gutter:    '#1b2240',
  pinHead:   '#f8f5e8',
  pinShade:  '#bfc7e6',
  pinStripe: '#e8554f',
  pinFall:   '#5a6188',
  ball:      '#4a9be8',
  ballShade: '#1f5494',
  ballSheen: '#cce6ff',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  aimGuide:  '#f7e69a',
  ok:        '#54c47c',
};

function drawScene(ctx, s, viewW, viewH, lang, best) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, viewW, viewH);
  drawLane(ctx);
  drawPins(ctx, s);
  drawBall(ctx, s);
  drawAimGuide(ctx, s);
  drawHud(ctx, s, lang, best);
  drawScorePane(ctx, s, lang);
}

function drawLane(ctx) {
  // Gutters (background already there).
  ctx.fillStyle = PALETTE.laneEdge;
  ctx.fillRect(86, 36, 4, 428);
  ctx.fillRect(270, 36, 4, 428);
  // Lane planks.
  ctx.fillStyle = PALETTE.lane;
  ctx.fillRect(90, 40, 180, 420);
  ctx.fillStyle = PALETTE.laneShade;
  for (let x = 96; x < 270; x += 18) ctx.fillRect(x, 40, 1, 420);
  // Foul line near the bottom and arrow marks closer to the player.
  ctx.fillStyle = PALETTE.laneShade;
  ctx.fillRect(90, 420, 180, 2);
  ctx.fillStyle = '#7d4a1a';
  for (let i = 0; i < 5; i++) {
    const x = 110 + i * 36;
    ctx.fillRect(x, 360, 6, 3);
  }
}

function drawPins(ctx, s) {
  for (const p of s.pins) {
    if (p.alive) drawPinStanding(ctx, p.x, p.y);
    else         drawPinFallen(ctx, p.x, p.y);
  }
}
function drawPinStanding(ctx, x, y) {
  ctx.fillStyle = PALETTE.pinShade;
  ctx.fillRect((x - 4) | 0, (y - 4) | 0, 8, 9);
  ctx.fillStyle = PALETTE.pinHead;
  ctx.fillRect((x - 3) | 0, (y - 5) | 0, 6, 8);
  ctx.fillStyle = PALETTE.pinStripe;
  ctx.fillRect((x - 3) | 0, (y - 2) | 0, 6, 1);
  ctx.fillStyle = '#fff';
  ctx.fillRect((x - 2) | 0, (y - 4) | 0, 1, 1);
}
function drawPinFallen(ctx, x, y) {
  ctx.fillStyle = PALETTE.pinFall;
  ctx.fillRect((x - 4) | 0, (y + 1) | 0, 8, 3);
}

function drawBall(ctx, s) {
  // Ball: either rolling, or at the spawn point while the player aims.
  let bx, by;
  if (s.ball) { bx = s.ball.x; by = s.ball.y; }
  else        { bx = 180;       by = 448; }
  ctx.fillStyle = PALETTE.ballShade;
  ctx.beginPath(); ctx.arc(bx | 0, by | 0, 8, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.ball;
  ctx.beginPath(); ctx.arc(bx | 0, by | 0, 7, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.ballSheen;
  ctx.fillRect(((bx - 2) | 0), ((by - 4) | 0), 2, 2);
  // Three finger holes.
  ctx.fillStyle = '#0c1230';
  ctx.fillRect(((bx - 1) | 0), ((by - 2) | 0), 1, 1);
  ctx.fillRect(((bx + 1) | 0), ((by - 2) | 0), 1, 1);
  ctx.fillRect(((bx) | 0),     ((by) | 0),     1, 1);
}

function drawAimGuide(ctx, s) {
  if (!s.aim) return;
  const sx = 180, sy = 448;
  const dx = s.aim.x - sx, dy = s.aim.y - sy;
  if (dy >= -6) return;
  const len = Math.hypot(dx, dy);
  const ux = dx / len, uy = dy / len;
  ctx.fillStyle = PALETTE.aimGuide;
  for (let d = 14; d < Math.min(140, len); d += 8) {
    const px = sx + ux * d, py = sy + uy * d;
    ctx.fillRect((px - 1) | 0, (py - 1) | 0, 2, 2);
  }
  // Power dot at end.
  const power = Math.min(1, len / 100);
  ctx.fillStyle = power > 0.85 ? '#e8554f' : PALETTE.aimGuide;
  const px = sx + ux * Math.min(140, len), py = sy + uy * Math.min(140, len);
  ctx.fillRect((px - 2) | 0, (py - 2) | 0, 4, 4);
}

function drawHud(ctx, s, lang, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(t(lang, 'frame') + ' ' + s.frame + '/' + 10, 8, 16);
  ctx.textAlign = 'center';
  ctx.fillText('TOTAL ' + gameScore(s), 180, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'best') + ' ' + (best | 0), 352, 16);
}

// Compact frame strip at the very top, between the HUD and the lane.
function drawScorePane(ctx, s, lang) {
  // (Score strip lives just below the HUD bar at y=32-40.)
  const stripY = 33, stripH = 6;
  for (let i = 0; i < 10; i++) {
    const x = 4 + i * 35;
    const w = 33;
    ctx.fillStyle = (i + 1 === s.frame) ? '#54c47c' : '#262d54';
    ctx.fillRect(x, stripY, w, stripH);
    const f = s.frames[i];
    ctx.fillStyle = PALETTE.hudText;
    ctx.font = '9px monospace';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    // Throws as small symbols.
    const t0 = f.throws[0], t1 = f.throws[1], t2 = f.throws[2];
    const sym = (a, b) => {
      if (a == null) return ' ';
      if (a === 10) return 'X';
      if (b != null && a + b === 10) return '/';
      return String(a);
    };
    let s1 = '', s2 = '';
    if (i === 9) {
      s1 = t0 == null ? '' : (t0 === 10 ? 'X' : String(t0));
      if (t1 != null) {
        if (t0 === 10) s2 = (t1 === 10 ? 'X' : String(t1));
        else s2 = (t0 + t1 === 10 ? '/' : String(t1));
      }
      if (t2 != null) s2 += (t2 === 10 ? 'X' : (t1 + t2 === 10 ? '/' : String(t2)));
    } else {
      s1 = (t0 == null) ? '' : (t0 === 10 ? '' : String(t0));
      s2 = (t0 === 10) ? 'X' : sym(t1, t0);
    }
    ctx.fillText((s1 || '') + (s2 ? (' ' + s2) : ''), x + 2, stripY + 8);
    if (f.score != null) {
      ctx.fillText(String(f.score), x + 2, stripY + 18);
    }
  }
}
