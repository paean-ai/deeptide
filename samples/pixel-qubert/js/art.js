// Pixel-art rendering for Pixel Qubert. 360x480 world units.

const PALETTE = {
  bg:        '#0a0a1c',
  bgGlow:    '#161630',
  cubeTop:   '#36306a',          // untouched cube top
  cubeLeft:  '#23204a',
  cubeRight: '#14122e',
  cubeEdge:  '#070318',
  shadow:    'rgba(0,0,0,0.45)',
  player:    '#ffaa40',
  playerHi:  '#ffd078',
  playerLo:  '#8a4c10',
  eye:       '#0a0a1c',
  enemy:     '#ff5a5a',
  enemyHi:   '#ff9090',
  hud:       '#070318',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  heart:     '#ff4a5a',
  warn:      '#ff5a3a',
  win:       '#5fc06e',
};

// Cube geometry: top diamond CUBE_W wide, side faces CUBE_H/2 tall.
const CUBE_W = 36;        // top-face horizontal span
const CUBE_H = 36;        // total cube height (top + walls)
const TOP_H  = 18;        // top diamond height (CUBE_W/2)
const ORIGIN_X = 180;     // apex sits at canvas centre x
const ORIGIN_Y = 80;      // top of the pyramid sits 80 px below the HUD

// Pixel coords (cube top-centre) for the cube at (r, c).
function cubeCenter(r, c) {
  const x = ORIGIN_X + (c - r / 2) * CUBE_W;
  const y = ORIGIN_Y + r * (CUBE_H * 0.75);
  return { x, y };
}
// Where the player sprite sits on cube (r, c) when standing on it.
function standCenter(r, c) {
  const ctr = cubeCenter(r, c);
  return { x: ctr.x, y: ctr.y - 14 };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  // Faint speckle for warmth.
  ctx.fillStyle = PALETTE.bgGlow;
  for (let i = 0; i < 32; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function stageColors(level, stage) {
  if (stage === 0) return { top: PALETTE.cubeTop, hi: '#4a4280' };
  const target = level.cfg.stages[stage - 1];
  const hi = lighten(target, 0.25);
  return { top: target, hi };
}

function drawPyramid(ctx, s) {
  // Draw back-to-front so closer cubes overlap farther ones. Iterate over
  // the actual cubes array so a smaller pyramid (e.g. the title demo)
  // still renders cleanly.
  for (let r = 0; r < s.cubes.length; r++) for (let c = 0; c < s.cubes[r].length; c++) {
    drawCube(ctx, s, r, c);
  }
}

function drawCube(ctx, s, r, c) {
  const stage = s.cubes[r][c];
  const { top, hi } = stageColors(s, stage);
  const { x, y } = cubeCenter(s.player ? r : r, c);
  // Side walls (always dark — left side a touch lighter than right).
  ctx.fillStyle = PALETTE.cubeEdge;
  // left face polygon
  fillQuad(ctx,
    x - CUBE_W / 2, y,
    x,             y + TOP_H,
    x,             y + TOP_H + CUBE_H / 2,
    x - CUBE_W / 2, y + CUBE_H / 2);
  ctx.fillStyle = PALETTE.cubeLeft;
  fillQuad(ctx,
    x - CUBE_W / 2 + 1, y + 1,
    x - 1,              y + TOP_H,
    x - 1,              y + TOP_H + CUBE_H / 2 - 1,
    x - CUBE_W / 2 + 1, y + CUBE_H / 2 - 1);
  // right face
  ctx.fillStyle = PALETTE.cubeEdge;
  fillQuad(ctx,
    x + CUBE_W / 2, y,
    x,              y + TOP_H,
    x,              y + TOP_H + CUBE_H / 2,
    x + CUBE_W / 2, y + CUBE_H / 2);
  ctx.fillStyle = PALETTE.cubeRight;
  fillQuad(ctx,
    x + CUBE_W / 2 - 1, y + 1,
    x + 1,              y + TOP_H,
    x + 1,              y + TOP_H + CUBE_H / 2 - 1,
    x + CUBE_W / 2 - 1, y + CUBE_H / 2 - 1);
  // Top diamond (colour changes per stage).
  ctx.fillStyle = PALETTE.cubeEdge;
  fillQuad(ctx,
    x - CUBE_W / 2, y,
    x,              y - TOP_H,
    x + CUBE_W / 2, y,
    x,              y + TOP_H);
  ctx.fillStyle = top;
  fillQuad(ctx,
    x - CUBE_W / 2 + 1, y,
    x,                  y - TOP_H + 1,
    x + CUBE_W / 2 - 1, y,
    x,                  y + TOP_H - 1);
  // Highlight stripe on the top-left half.
  ctx.fillStyle = hi;
  fillQuad(ctx,
    x - CUBE_W / 2 + 3, y - 1,
    x - 4,              y - TOP_H + 3,
    x - 4,              y - TOP_H + 5,
    x - CUBE_W / 2 + 5, y);
}

function fillQuad(ctx, ax, ay, bx, by, cx, cy, dx, dy) {
  ctx.beginPath();
  ctx.moveTo(ax, ay);
  ctx.lineTo(bx, by);
  ctx.lineTo(cx, cy);
  ctx.lineTo(dx, dy);
  ctx.closePath();
  ctx.fill();
}

function drawPlayer(ctx, s) {
  const p = s.player;
  if (!p.alive) return;
  if (p.hitFlash > 0 && Math.floor(p.hitFlash * 12) % 2 === 0) return;
  let cx, cy;
  if (p.hopT > 0 && p.hopFrom) {
    // Interpolate between hopFrom and current target, with a parabolic arc.
    const from = standCenter(p.hopFrom.r, p.hopFrom.c);
    const toR = inBounds(p.r, p.c) ? p.r : p.hopFrom.r;
    const toC = inBounds(p.r, p.c) ? p.c : p.hopFrom.c;
    const to = standCenter(toR, toC);
    const t = 1 - p.hopT / (p.falling ? 0.35 : 0.22);
    cx = from.x + (to.x - from.x) * t;
    cy = from.y + (to.y - from.y) * t - Math.sin(t * Math.PI) * 16;
    if (p.falling) {
      // Once past midpoint, accelerate it downward and offscreen.
      cy += Math.max(0, t - 0.5) * 200;
    }
  } else {
    const here = standCenter(p.r, p.c);
    cx = here.x; cy = here.y;
  }
  // Shadow on the cube top under the player.
  ctx.fillStyle = PALETTE.shadow;
  const sh = standCenter(p.hopT > 0 && p.hopFrom ? p.hopFrom.r : p.r,
                         p.hopT > 0 && p.hopFrom ? p.hopFrom.c : p.c);
  ctx.fillRect(sh.x - 8, sh.y + 12, 16, 3);
  // Body
  ctx.fillStyle = PALETTE.playerLo;
  fillDisk(ctx, cx, cy + 1, 9);
  ctx.fillStyle = PALETTE.player;
  fillDisk(ctx, cx, cy, 9);
  ctx.fillStyle = PALETTE.playerHi;
  fillDisk(ctx, cx - 2, cy - 2, 4);
  // Snout
  ctx.fillStyle = PALETTE.player;
  ctx.fillRect(cx - 4, cy + 1, 8, 4);
  ctx.fillStyle = PALETTE.playerLo;
  ctx.fillRect(cx - 4, cy + 4, 8, 1);
  // Eyes
  ctx.fillStyle = '#fff7ed';
  ctx.fillRect(cx - 5, cy - 5, 3, 3);
  ctx.fillRect(cx + 2, cy - 5, 3, 3);
  ctx.fillStyle = PALETTE.eye;
  ctx.fillRect(cx - 4, cy - 4, 1, 1);
  ctx.fillRect(cx + 3, cy - 4, 1, 1);
}

function drawEnemies(ctx, s) {
  for (const e of s.enemies) {
    // Same hop arc as the player.
    let cx, cy;
    if (e.hopT > 0) {
      const from = standCenter(e.r, e.c);
      const toR = inBounds(e.nextR, e.nextC) ? e.nextR : e.r;
      const toC = inBounds(e.nextR, e.nextC) ? e.nextC : e.c;
      const to = standCenter(toR, toC);
      const t = 1 - e.hopT / s.cfg.enemyDescend;
      cx = from.x + (to.x - from.x) * t;
      cy = from.y + (to.y - from.y) * t - Math.sin(t * Math.PI) * 14;
    } else {
      const here = standCenter(e.r, e.c);
      cx = here.x; cy = here.y;
    }
    ctx.fillStyle = PALETTE.shadow;
    ctx.fillRect(cx - 7, cy + 12, 14, 3);
    ctx.fillStyle = '#3a0c0c';
    fillDisk(ctx, cx, cy + 1, 8);
    ctx.fillStyle = e.color;
    fillDisk(ctx, cx, cy, 8);
    ctx.fillStyle = PALETTE.enemyHi;
    fillDisk(ctx, cx - 2, cy - 2, 4);
  }
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
  for (let i = 0; i < Math.max(0, s.lives + 1); i++) drawHeart(ctx, 130 + i * 12, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'cubes') + ' ' + s.completed + '/' + CUBE_COUNT, VW / 2 + 16, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 6, 16);
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
  const a = Math.min(1, s.flash / 0.5);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.35 * a})` :
                  s.player.alive ? `rgba(255,255,255,${0.18 * a})` :
                                   `rgba(255,80,80,${0.5 * a})`;
  ctx.fillRect(0, 32, VW, VH - 32);
}

// Hex lighten helper.
function lighten(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  const r = Math.min(255, ((n >> 16) & 0xff) + 255 * amt) | 0;
  const g = Math.min(255, ((n >> 8) & 0xff)  + 255 * amt) | 0;
  const b = Math.min(255, (n & 0xff)         + 255 * amt) | 0;
  return '#' + [r,g,b].map(v => v.toString(16).padStart(2,'0')).join('');
}
