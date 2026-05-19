// Pixel Street Brawl - pixel art for fighters and the street backdrop.

function sbShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// A fighter drawn so the feet sit at (cx, footY). `scale` for bosses.
function drawFighter(ctx, cx, footY, facing, body, pose, poseT, t, scale) {
  const u = (scale || 1);
  const skin = '#e8b98a', ink = '#181420';
  const dark = sbShade(body, -54), lite = sbShade(body, 44);
  ctx.save();
  ctx.translate(Math.round(cx), Math.round(footY));
  ctx.scale(facing, 1);
  ctx.scale(u, u);
  const R = (x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(x, y, w, h); };
  // shadow
  ctx.fillStyle = 'rgba(0,0,0,0.32)';
  ctx.fillRect(-12, -3, 24, 4);

  // legs
  let la = 0, lb = 0;
  if (pose === 'walk') { const f = Math.floor(t * 11) % 2; la = f ? -4 : 4; lb = -la; }
  else if (pose === 'jump') { la = -3; lb = 4; }
  else if (pose === 'kick') { la = 0; lb = 12; }
  R(-8 + la, -16, 6, 16, ink);
  R(2 + lb, -16, 6, 16, ink);
  R(-8 + la, -4, 7, 4, '#0c0a12');
  R(2 + lb, -4, 7, 4, '#0c0a12');
  if (pose === 'kick') { R(8, -22, 14, 6, ink); R(20, -24, 6, 5, '#0c0a12'); }  // kick leg

  // torso
  const lean = pose === 'hurt' ? -4 : pose === 'punch' ? 3 : 0;
  R(-9 + lean, -38, 18, 24, body);
  R(-9 + lean, -38, 18, 5, lite);
  R(-9 + lean, -19, 18, 3, dark);
  R(-5 + lean, -34, 10, 5, sbShade(body, 26));

  // head
  R(-7 + lean, -52, 14, 14, skin);
  R(-7 + lean, -52, 14, 4, sbShade(skin, -34));
  R(facing > 0 ? 1 + lean : -5 + lean, -47, 3, 3, ink);   // eye

  // arms
  if (pose === 'punch') {
    const reach = 6 + Math.min(1, poseT / 0.12) * 16;
    R(8, -34, reach, 6, skin);
    R(8 + reach, -36, 7, 9, body);
  } else if (pose === 'attack') {
    const wind = poseT < 0.5 ? -8 : 14;                    // windup back, then forward
    R(6, -34, Math.abs(wind) + 4, 6, skin);
    if (wind > 0) R(6 + wind, -36, 7, 8, dark);
  } else if (pose === 'hurt') {
    R(-16, -36, 8, 6, skin);
  } else {
    const sw = pose === 'walk' ? Math.sin(t * 11) * 2 : 0;
    R(7, -36 + sw, 6, 13, skin);
    R(-12, -36 - sw, 6, 13, skin);
  }
  ctx.restore();
}

function drawStreet(ctx, t) {
  // sky
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#241b3a');
  g.addColorStop(1, '#3a2c44');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, GROUND);
  // skyline
  for (let i = 0; i < 9; i++) {
    const bx = i * 58 - 10, bh = 60 + (i * 53 % 70);
    ctx.fillStyle = i % 2 ? '#1c1730' : '#221b38';
    ctx.fillRect(bx, GROUND - bh, 50, bh);
    ctx.fillStyle = '#3a3258';
    for (let wy = GROUND - bh + 8; wy < GROUND - 8; wy += 16) {
      for (let wx = bx + 6; wx < bx + 44; wx += 14) {
        if ((wx + wy) % 3 === 0) ctx.fillRect(wx, wy, 6, 8);
      }
    }
  }
  // road
  ctx.fillStyle = '#2b2730';
  ctx.fillRect(0, GROUND, VW, VH - GROUND);
  ctx.fillStyle = '#3a3642';
  ctx.fillRect(0, GROUND, VW, 4);
  ctx.fillStyle = '#5a5466';
  for (let x = (t * 10 % 48); x < VW; x += 48) ctx.fillRect(x, GROUND + 24, 22, 4);
}
