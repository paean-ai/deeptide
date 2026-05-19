// Pixel Boulder Siege - scenery and pixel sprite drawing.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, GROUND_Y);
  g.addColorStop(0, '#3a2a52');
  g.addColorStop(1, '#7a5a6e');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, GROUND_Y);
  // distant hills
  ctx.fillStyle = '#4a3a5e';
  for (let i = 0; i < 4; i++) {
    const hx = i * 110 - 30, hr = 70 + i * 12;
    ctx.beginPath();
    ctx.arc(hx, GROUND_Y, hr, Math.PI, 0);
    ctx.fill();
  }
  // ground
  ctx.fillStyle = '#5a3f2a';
  ctx.fillRect(0, GROUND_Y, VW, VH - GROUND_Y);
  ctx.fillStyle = '#3f7a3a';
  ctx.fillRect(0, GROUND_Y, VW, 6);
}

function drawCannon(ctx, angle, charging) {
  const x = CANNON_X, y = CANNON_Y;
  // barrel aimed along the launch angle
  ctx.save();
  ctx.translate(x, y - 6);
  ctx.rotate(angle);
  ctx.fillStyle = charging ? '#ffd98a' : '#caa15a';
  ctx.fillRect(0, -7, 30, 14);
  ctx.fillStyle = '#1a1018';
  ctx.fillRect(24, -5, 6, 10);
  ctx.restore();
  // wheel + frame
  ctx.fillStyle = '#6e4a2e';
  ctx.fillRect(x - 12, y - 2, 24, 12);
  ctx.fillStyle = '#2a1a12';
  ctx.beginPath();
  ctx.arc(x - 4, y + 12, 9, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#caa15a';
  ctx.fillRect(x - 6, y + 10, 4, 4);
}

function drawBlock(ctx, e) {
  const x = e.x, y = e.y, def = BLOCKS[e.type];
  const ratio = e.hp / e.maxhp;
  if (e.type === 'W') {
    ctx.fillStyle = '#9c6b3c';
    ctx.fillRect(x, y, B, B);
    ctx.fillStyle = '#7a4f2a';
    ctx.fillRect(x, y + B / 2 - 1, B, 2);
    ctx.fillStyle = '#b78550';
    ctx.fillRect(x + 1, y + 1, B - 2, 3);
  } else if (e.type === 'S') {
    ctx.fillStyle = '#8a8a96';
    ctx.fillRect(x, y, B, B);
    ctx.fillStyle = '#6e6e7c';
    ctx.fillRect(x + 3, y + 4, 5, 5);
    ctx.fillRect(x + B - 9, y + B - 9, 6, 5);
    ctx.fillStyle = '#a6a6b2';
    ctx.fillRect(x + 1, y + 1, B - 2, 3);
  } else { // glass
    ctx.fillStyle = 'rgba(150, 224, 240, 0.82)';
    ctx.fillRect(x, y, B, B);
    ctx.fillStyle = 'rgba(255, 255, 255, 0.7)';
    ctx.fillRect(x + 3, y + 3, 4, B - 8);
  }
  // outline + damage cracks
  ctx.strokeStyle = 'rgba(0, 0, 0, 0.35)';
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, B - 1, B - 1);
  if (ratio < 0.66) {
    ctx.strokeStyle = 'rgba(0, 0, 0, 0.55)';
    ctx.beginPath();
    ctx.moveTo(x + B * 0.3, y + 2);
    ctx.lineTo(x + B * 0.5, y + B * 0.55);
    if (ratio < 0.33) ctx.lineTo(x + B * 0.3, y + B - 2);
    else ctx.lineTo(x + B * 0.62, y + B - 3);
    ctx.stroke();
  }
}

function drawGoblin(ctx, e) {
  const x = e.x, y = e.y;
  const cx = x + B / 2;
  // body
  ctx.fillStyle = '#5fb24a';
  ctx.fillRect(x + 4, y + 6, B - 8, B - 8);
  // ears
  ctx.fillRect(x + 1, y + 7, 4, 5);
  ctx.fillRect(x + B - 5, y + 7, 4, 5);
  // head highlight
  ctx.fillStyle = '#7ed066';
  ctx.fillRect(x + 6, y + 8, B - 12, 4);
  // eyes
  ctx.fillStyle = '#1c0f12';
  ctx.fillRect(cx - 5, y + 11, 3, 3);
  ctx.fillRect(cx + 2, y + 11, 3, 3);
  // snarl
  ctx.fillStyle = '#fff';
  ctx.fillRect(cx - 3, y + B - 7, 6, 2);
}

function drawProjectile(ctx, p) {
  ctx.fillStyle = '#4a3f3a';
  ctx.beginPath();
  ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#665a52';
  ctx.fillRect(p.x - 3, p.y - 4, 4, 4);
  ctx.fillRect(p.x + 1, p.y, 3, 3);
  ctx.fillStyle = '#2e2622';
  ctx.fillRect(p.x - 1, p.y + 2, 3, 3);
}
