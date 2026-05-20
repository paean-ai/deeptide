// Pixel-art rendering for Pixel Hop. Everything is drawn in 360x480 world
// units; the host scales the canvas with image-rendering: pixelated.

const COLORS = {
  skyTop: '#1f2240',
  skyBot: '#4f5dab',
  star: '#f4f4ff',
  cloud: '#dde6ff',
  hud: '#0d1228',
  hudText: '#f8f5e8',
  hudDim: '#9aa6cc',
  player: '#ffd86a',
  playerShade: '#c2832a',
  playerOutline: '#3b2f1e',
  platStatic: '#7d4f29',
  platStaticTop: '#a36835',
  platMover: '#3d7fb0',
  platMoverTop: '#65a9d8',
  platSpring: '#54c47c',
  platSpringTop: '#8be59d',
  platCloud: '#e6edff',
  platCloudShade: '#aeb9d6',
  gem: '#ff5d92',
  gemShade: '#a83363',
};

function drawScene(ctx, s, viewW, viewH) {
  // Sky gradient with parallax stars based on cameraY.
  const grad = ctx.createLinearGradient(0, 0, 0, viewH);
  grad.addColorStop(0, COLORS.skyTop);
  grad.addColorStop(1, COLORS.skyBot);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, viewW, viewH);
  drawStars(ctx, s, viewW, viewH);
  // World-to-screen offset: worldY -> worldY - cameraY.
  // Platforms.
  for (const pl of s.platforms) {
    if (!pl.alive) continue;
    const sy = pl.y - s.cameraY;
    if (sy < -20 || sy > viewH + 20) continue;
    drawPlatform(ctx, pl, sy);
  }
  // Gems.
  for (const g of s.gems) {
    if (!g.alive) continue;
    const sy = g.y - s.cameraY;
    if (sy < -20 || sy > viewH + 20) continue;
    drawGem(ctx, g.x, sy, (s.tickCount || 0));
  }
  // Player.
  const psy = s.player.y - s.cameraY;
  drawPlayer(ctx, s.player.x, psy, s.player.vx, s.player.vy);
  // Wrap ghost when near the edge so the player never visually jumps.
  if (s.player.x < 20) drawPlayer(ctx, s.player.x + viewW, psy, s.player.vx, s.player.vy);
  else if (s.player.x > viewW - 20) drawPlayer(ctx, s.player.x - viewW, psy, s.player.vx, s.player.vy);
  // HUD bar.
  drawHud(ctx, s, viewW, viewH);
}

function drawStars(ctx, s, w, h) {
  ctx.fillStyle = COLORS.star;
  // Deterministic star field offset by cameraY * 0.25 for parallax.
  const off = ((s.cameraY * 0.25) % h + h) % h;
  for (let i = 0; i < 36; i++) {
    const sx = (i * 53 + 17) % w;
    const sy = ((i * 31 + 11) % h + off) % h;
    const sz = (i % 5 === 0) ? 2 : 1;
    ctx.fillRect(sx | 0, sy | 0, sz, sz);
  }
  // Far clouds drifting horizontally - very subtle.
  ctx.fillStyle = 'rgba(255,255,255,0.06)';
  for (let i = 0; i < 4; i++) {
    const cx = ((i * 97 + (s.cameraY * 0.18) | 0) % (w + 80)) - 40;
    const cy = (i * 71) % h;
    ctx.fillRect(cx, cy, 36, 6);
    ctx.fillRect(cx + 6, cy - 3, 24, 6);
  }
}

function drawPlatform(ctx, pl, sy) {
  if (pl.type === 'cloud') {
    ctx.fillStyle = COLORS.platCloudShade;
    ctx.fillRect(pl.x | 0, (sy + 2) | 0, pl.w, 8);
    ctx.fillStyle = COLORS.platCloud;
    ctx.fillRect((pl.x + 2) | 0, sy | 0, pl.w - 4, 6);
    ctx.fillRect((pl.x + 6) | 0, (sy - 2) | 0, pl.w - 12, 4);
    return;
  }
  const top = pl.type === 'spring' ? COLORS.platSpringTop
            : pl.type === 'mover'  ? COLORS.platMoverTop
            :                        COLORS.platStaticTop;
  const base = pl.type === 'spring' ? COLORS.platSpring
             : pl.type === 'mover'  ? COLORS.platMover
             :                        COLORS.platStatic;
  ctx.fillStyle = base;
  ctx.fillRect(pl.x | 0, sy | 0, pl.w, 10);
  ctx.fillStyle = top;
  ctx.fillRect(pl.x | 0, sy | 0, pl.w, 3);
  if (pl.type === 'spring') {
    // Coil decoration.
    ctx.fillStyle = '#2c6c41';
    for (let i = 6; i < pl.w - 4; i += 6) ctx.fillRect((pl.x + i) | 0, (sy - 3) | 0, 3, 3);
  } else if (pl.type === 'mover') {
    ctx.fillStyle = '#1d4d72';
    ctx.fillRect((pl.x + 4) | 0, (sy + 6) | 0, pl.w - 8, 2);
  } else {
    ctx.fillStyle = '#5a3819';
    ctx.fillRect((pl.x + 4) | 0, (sy + 7) | 0, pl.w - 8, 1);
  }
}

function drawGem(ctx, x, sy, t) {
  const bob = Math.sin((t || 0) * 0.15) * 1.5;
  const cy = (sy + bob) | 0, cx = x | 0;
  ctx.fillStyle = COLORS.gemShade;
  ctx.fillRect(cx - 4, cy - 1, 8, 4);
  ctx.fillRect(cx - 2, cy + 3, 4, 2);
  ctx.fillStyle = COLORS.gem;
  ctx.fillRect(cx - 3, cy - 2, 6, 3);
  ctx.fillRect(cx - 1, cy - 4, 2, 2);
  ctx.fillStyle = '#fff';
  ctx.fillRect(cx - 2, cy - 2, 1, 1);
}

function drawPlayer(ctx, x, sy, vx, vy) {
  const cx = x | 0, cy = sy | 0;
  // Shadow on the way down (faint).
  if (vy > 50) {
    ctx.fillStyle = 'rgba(0,0,0,0.18)';
    ctx.fillRect(cx - 8, cy + 14, 16, 2);
  }
  // Body.
  ctx.fillStyle = COLORS.playerShade;
  ctx.fillRect(cx - 8, cy - 6, 16, 14);
  ctx.fillStyle = COLORS.player;
  ctx.fillRect(cx - 7, cy - 7, 14, 12);
  // Outline.
  ctx.fillStyle = COLORS.playerOutline;
  ctx.fillRect(cx - 8, cy - 8, 16, 1);
  ctx.fillRect(cx - 8, cy + 7, 16, 1);
  ctx.fillRect(cx - 9, cy - 7, 1, 15);
  ctx.fillRect(cx + 8, cy - 7, 1, 15);
  // Eyes face the way you're moving.
  const eyeOff = vx > 30 ? 2 : vx < -30 ? -2 : 0;
  ctx.fillRect(cx - 3 + eyeOff, cy - 3, 2, 2);
  ctx.fillRect(cx + 1 + eyeOff, cy - 3, 2, 2);
  // Mouth - open on the way up.
  ctx.fillRect(cx - 2, cy + 2, 4, vy < 0 ? 2 : 1);
  // Ears: spring style ears that wiggle while rising.
  if (vy < 0) {
    ctx.fillStyle = COLORS.player;
    ctx.fillRect(cx - 6, cy - 10, 3, 3);
    ctx.fillRect(cx + 3, cy - 10, 3, 3);
  }
}

function drawHud(ctx, s, w, h) {
  ctx.fillStyle = COLORS.hud;
  ctx.fillRect(0, 0, w, 26);
  ctx.fillStyle = COLORS.hudText;
  ctx.font = '10px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  const alt = s.altitude | 0;
  const tgt = s.cfg.target | 0;
  ctx.fillText(`ALT ${alt}m / ${tgt}m`, 6, 13);
  ctx.textAlign = 'right';
  ctx.fillText(`GEMS ${s.gemsCollected}`, w - 6, 13);
  // Progress bar.
  const px = 110, pw = 130, py = 9, ph = 8;
  ctx.fillStyle = '#1c2440';
  ctx.fillRect(px, py, pw, ph);
  const ratio = Math.max(0, Math.min(1, alt / tgt));
  ctx.fillStyle = '#54c47c';
  ctx.fillRect(px + 1, py + 1, ((pw - 2) * ratio) | 0, ph - 2);
}

function drawLegendIcon(ctx, x, y, type) {
  if (type === 'gem') { drawGem(ctx, x + 6, y + 6, 0); return; }
  const pl = { x, y: 0, w: 24, type, alive: true };
  drawPlatform(ctx, pl, y);
}
