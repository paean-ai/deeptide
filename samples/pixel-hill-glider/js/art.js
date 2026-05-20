// Pixel Hill Glider - sky, hills, bird and orb rendering.

function lerp(a, b, f) { return a + (b - a) * f; }
function lerpColor(c1, c2, f) {
  const r = Math.round(lerp(c1[0], c2[0], f));
  const g = Math.round(lerp(c1[1], c2[1], f));
  const b = Math.round(lerp(c1[2], c2[2], f));
  return `rgb(${r},${g},${b})`;
}

// light 1 = bright day, 0 = dusk
function drawSky(ctx, light) {
  const top = lerpColor([42, 54, 92], [126, 200, 232], light);
  const bot = lerpColor([201, 122, 90], [196, 232, 238], light);
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, top);
  g.addColorStop(1, bot);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // sun / moon sinks as the light fades
  const sy = lerp(36, VH * 0.66, 1 - light);
  ctx.fillStyle = light > 0.4 ? '#fff2b0' : '#e8e2c4';
  ctx.beginPath(); ctx.arc(VW - 78, sy, 22, 0, Math.PI * 2); ctx.fill();
  ctx.globalAlpha = 0.3;
  ctx.beginPath(); ctx.arc(VW - 78, sy, 30, 0, Math.PI * 2); ctx.fill();
  ctx.globalAlpha = 1;
}

function bgLayer(ctx, camX, camY, par, baseY, amp, freq, color) {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.moveTo(0, VH);
  for (let sx = 0; sx <= VW; sx += 10) {
    const wx = camX * par + sx;
    const y = baseY - camY * par * 0.4
      + Math.sin(wx * freq) * amp + Math.sin(wx * freq * 2.4 + 1) * amp * 0.32;
    ctx.lineTo(sx, y);
  }
  ctx.lineTo(VW, VH);
  ctx.closePath();
  ctx.fill();
}

function drawHills(ctx, camX, camY, light) {
  // two parallax background ridges
  const dim = 0.5 + light * 0.5;
  bgLayer(ctx, camX, camY, 0.30, VH * 0.52, 30, 0.006,
    lerpColor([60, 70, 110], [150, 196, 170], dim));
  bgLayer(ctx, camX, camY, 0.55, VH * 0.66, 26, 0.010,
    lerpColor([46, 62, 86], [110, 170, 120], dim));

  // main playfield terrain
  const pts = [];
  for (let sx = -12; sx <= VW + 12; sx += 6) pts.push([sx, terrainY(camX + sx) - camY]);

  ctx.fillStyle = '#5a3f24';                      // soil body
  ctx.beginPath();
  ctx.moveTo(pts[0][0], VH);
  for (const [x, y] of pts) ctx.lineTo(x, y);
  ctx.lineTo(pts[pts.length - 1][0], VH);
  ctx.closePath();
  ctx.fill();

  ctx.lineJoin = 'round';
  ctx.strokeStyle = '#3f8f3a';                    // grass band
  ctx.lineWidth = 22;
  ctx.beginPath();
  pts.forEach((p, i) => (i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1])));
  ctx.stroke();
  ctx.strokeStyle = '#74d27e';                    // bright surface line
  ctx.lineWidth = 4;
  ctx.beginPath();
  pts.forEach((p, i) => (i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1])));
  ctx.stroke();
}

function drawOrb(ctx, sx, sy, time, kind) {
  const r = 7 + Math.sin(time * 6 + sx) * 1.6;
  const isGold = kind === 'gold';
  // Golden orbs render slightly larger with a warmer, brighter halo so the
  // player can tell them apart at a glance.
  const haloR = isGold ? r + 9 : r + 6;
  ctx.fillStyle = isGold ? 'rgba(255,180,80,0.36)' : 'rgba(255,236,140,0.28)';
  ctx.beginPath(); ctx.arc(sx, sy, haloR, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = isGold ? '#ff8f1f' : '#ffe14d';
  ctx.beginPath(); ctx.arc(sx, sy, r + (isGold ? 1 : 0), 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = isGold ? '#ffe8a8' : '#fff7c8';
  ctx.beginPath(); ctx.arc(sx - 1.5, sy - 1.5, r * 0.45, 0, Math.PI * 2); ctx.fill();
  if (isGold) {
    // Four pixel sparkles around a golden orb on alternating frames.
    ctx.fillStyle = '#fff7ed';
    const blink = Math.floor(time * 6) % 2 === 0 ? 1 : 0;
    if (blink) {
      ctx.fillRect((sx + r + 3) | 0, sy | 0, 1, 1);
      ctx.fillRect((sx - r - 3) | 0, sy | 0, 1, 1);
      ctx.fillRect(sx | 0, (sy + r + 3) | 0, 1, 1);
      ctx.fillRect(sx | 0, (sy - r - 3) | 0, 1, 1);
    }
  }
}

function drawBird(ctx, sx, sy, angle, diving, flap) {
  ctx.save();
  ctx.translate(sx, sy);
  ctx.rotate(angle);
  // shadow tail
  ctx.fillStyle = '#e8893a';
  ctx.fillRect(-13, -3, 6, 6);
  // body
  ctx.fillStyle = '#ffd24a';
  ctx.fillRect(-9, -6, 18, 12);
  ctx.fillStyle = '#f0a93a';
  ctx.fillRect(-9, 1, 18, 5);
  // head + beak + eye
  ctx.fillStyle = '#ffd24a';
  ctx.fillRect(4, -10, 10, 10);
  ctx.fillStyle = '#e8554f';
  ctx.fillRect(13, -6, 6, 4);
  ctx.fillStyle = '#16181d';
  ctx.fillRect(9, -7, 3, 3);
  // wing
  ctx.fillStyle = '#e8893a';
  if (diving) {
    ctx.fillRect(-7, -4, 10, 4);
  } else {
    const wy = Math.sin(flap) * 4;
    ctx.fillRect(-8, -2 + wy, 13, 5);
    ctx.fillStyle = '#f6b85a';
    ctx.fillRect(-8, -2 + wy, 6, 5);
  }
  ctx.restore();
}
