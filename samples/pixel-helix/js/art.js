// Pixel-art rendering for Pixel Helix. 360x480 world units.

const PALETTE = {
  bg:        '#0d0a18',
  bgHi:      '#1a1530',
  discBody:  '#2a2452',
  discEdge:  '#070318',
  solidA:    '#7a5fff',
  solidB:    '#5a3fd0',
  solidHi:   '#bda6ff',
  gap:       '#1a1530',
  gapHi:     '#3a3260',
  danger:    '#ff5a5a',
  dangerHi:  '#ff9090',
  ball:      '#ffe04a',
  ballHi:    '#fff0c0',
  ballLo:    '#a07a14',
  hud:       '#06031a',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  win:       '#5fc06e',
  bad:       '#ff7a7a',
};

const DISC_CX = 180;
const DISC_CY = 252;
const DISC_R  = 122;
const DISC_INNER = 36;
const BALL_R  = 8;

function drawBackdrop(ctx) {
  const grad = ctx.createLinearGradient(0, 0, 0, VH);
  grad.addColorStop(0, PALETTE.bg);
  grad.addColorStop(1, PALETTE.bgHi);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, VW, VH);
  // Speckle.
  ctx.fillStyle = '#1c163a';
  for (let i = 0; i < 30; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

// Render the current disc as a top-down ring, plus a faint "next" disc
// behind it so the player can plan ahead.
function drawTower(ctx, s) {
  // Faint preview of the next disc.
  if (s.current + 1 < s.depth) drawDisc(ctx, s.discs[s.current + 1], 0.35, 12);
  drawDisc(ctx, s.discs[s.current], 1.0, 0);
}

function drawDisc(ctx, disc, alpha, sink) {
  const cx = DISC_CX, cy = DISC_CY + sink;
  // Disc edge ring (under each segment).
  ctx.globalAlpha = alpha;
  // Each segment is a wedge from (cy + SEG_ANGLE*k) to (cy + SEG_ANGLE*(k+1)).
  for (let k = 0; k < SEGMENTS; k++) {
    const t = disc.segments[k];
    const a0 = disc.rotation + k * SEG_ANGLE - Math.PI / 2;
    const a1 = a0 + SEG_ANGLE;
    let bodyA = PALETTE.solidA, bodyB = PALETTE.solidB, hi = PALETTE.solidHi;
    if (t === 0) { bodyA = PALETTE.gap; bodyB = PALETTE.gap; hi = PALETTE.gapHi; }
    if (t === 2) { bodyA = PALETTE.danger; bodyB = PALETTE.danger; hi = PALETTE.dangerHi; }
    // Wedge fill.
    ctx.fillStyle = PALETTE.discEdge;
    pathWedge(ctx, cx, cy, DISC_INNER - 1, DISC_R + 1, a0, a1);
    ctx.fill();
    ctx.fillStyle = (k % 2 === 0) ? bodyA : bodyB;
    pathWedge(ctx, cx, cy, DISC_INNER, DISC_R, a0, a1);
    ctx.fill();
    // Inner highlight band for solids.
    if (t === 1) {
      ctx.fillStyle = hi;
      pathWedge(ctx, cx, cy, DISC_R - 4, DISC_R - 1, a0 + 0.02, a1 - 0.02);
      ctx.fill();
    } else if (t === 2) {
      // Spike teeth around the outer edge.
      ctx.fillStyle = hi;
      const tooths = 3;
      for (let tt = 0; tt < tooths; tt++) {
        const am = a0 + (tt + 0.5) / tooths * SEG_ANGLE;
        const tx = cx + Math.cos(am) * (DISC_R + 4);
        const ty = cy + Math.sin(am) * (DISC_R + 4);
        const bx = cx + Math.cos(am) * DISC_R;
        const by = cy + Math.sin(am) * DISC_R;
        const px = -Math.sin(am) * 3, py = Math.cos(am) * 3;
        ctx.beginPath();
        ctx.moveTo(tx, ty);
        ctx.lineTo(bx + px, by + py);
        ctx.lineTo(bx - px, by - py);
        ctx.closePath();
        ctx.fill();
      }
    }
  }
  // Centre hub.
  ctx.fillStyle = PALETTE.discEdge;
  ctx.beginPath(); ctx.arc(cx, cy, DISC_INNER + 1, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.discBody;
  ctx.beginPath(); ctx.arc(cx, cy, DISC_INNER, 0, Math.PI * 2); ctx.fill();
  ctx.globalAlpha = 1;
}

function pathWedge(ctx, cx, cy, r0, r1, a0, a1) {
  ctx.beginPath();
  ctx.arc(cx, cy, r1, a0, a1);
  ctx.arc(cx, cy, r0, a1, a0, true);
  ctx.closePath();
}

function drawBall(ctx, s) {
  // Ball pinned at 12 o'clock above the disc, bouncing vertically.
  // bounce y-offset: 0 (rest) to -28 (peak).
  const ay = -Math.sin(Math.PI * s.ballT) * 22;
  let by = DISC_CY - DISC_R - 14 + ay;
  let bx = DISC_CX;
  if (s.falling) {
    // Slide downward into the disc hub, fade alpha for the transition.
    by = DISC_CY - DISC_R - 14 + s.fallT * (DISC_R + 24);
    ctx.globalAlpha = Math.max(0, 1 - s.fallT * 0.9);
  }
  ctx.fillStyle = PALETTE.ballLo;
  fillDisk(ctx, bx, by + 1, BALL_R);
  ctx.fillStyle = PALETTE.ball;
  fillDisk(ctx, bx, by, BALL_R);
  ctx.fillStyle = PALETTE.ballHi;
  fillDisk(ctx, bx - 2, by - 2, BALL_R - 3);
  ctx.globalAlpha = 1;
}

function fillDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const w = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - w) | 0, (cy + dy) | 0, w * 2 + 1, 1);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'depth') + ' ' + s.current + '/' + s.depth, VW / 2, 16);
  if (s.combo > 1) {
    ctx.fillStyle = '#bda6ff';
    ctx.fillText('×' + s.combo, VW / 2 + 56, 16);
  }
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 6, 16);
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.5);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.4 * a})` :
                  s.over ? `rgba(255,80,80,${0.5 * a})` :
                           `rgba(255,255,255,${0.18 * a})`;
  ctx.fillRect(0, 32, VW, VH - 32);
}
