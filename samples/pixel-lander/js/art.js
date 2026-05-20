// Pixel-art rendering for Pixel Lander. 360x480 world units.

const PALETTE = {
  skyTop: '#0b0e26',
  skyMid: '#19204a',
  skyBot: '#2a2f5e',
  star:   '#f4f4ff',
  terrain:     '#7a7080',
  terrainShade:'#3f3848',
  pad:       '#54c47c',
  padStripe: '#8be59d',
  lander:       '#c8d6e0',
  landerDark:   '#7d8ba0',
  landerWindow: '#5dcef1',
  flame1:       '#fff19a',
  flame2:       '#ff8a3a',
  hud:        '#0d1228',
  hudText:    '#f8f5e8',
  hudDim:     '#9aa6cc',
  hudWarn:    '#e8554f',
  hudOk:      '#54c47c',
};

function drawScene(ctx, s, viewW, viewH) {
  // Sky gradient.
  const grad = ctx.createLinearGradient(0, 32, 0, viewH);
  grad.addColorStop(0, PALETTE.skyTop);
  grad.addColorStop(0.5, PALETTE.skyMid);
  grad.addColorStop(1, PALETTE.skyBot);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 32, viewW, viewH - 32);
  // Star field (deterministic).
  ctx.fillStyle = PALETTE.star;
  for (let i = 0; i < 60; i++) {
    const sx = (i * 53 + 17) % viewW;
    const sy = 40 + ((i * 31 + 11) % (viewH - 100));
    const sz = (i % 5 === 0) ? 2 : 1;
    ctx.fillRect(sx | 0, sy | 0, sz, sz);
  }
  // Terrain (silhouette + a shade line).
  drawTerrain(ctx, s, viewW, viewH);
  // Pad markings.
  drawPad(ctx, s);
  // Lander + thrust flame.
  drawLander(ctx, s.lander, s.thrust && s.fuel > 0);
  // HUD.
  drawHud(ctx, s);
  // Wind indicator at the top corner.
  if (s.cfg.wind) drawWindArrow(ctx, s.cfg.wind);
}

function drawTerrain(ctx, s, viewW, viewH) {
  ctx.fillStyle = PALETTE.terrain;
  ctx.beginPath();
  ctx.moveTo(0, viewH);
  for (const p of s.terrain) ctx.lineTo(p.x, p.y);
  ctx.lineTo(viewW, viewH);
  ctx.closePath();
  ctx.fill();
  // Surface shade.
  ctx.strokeStyle = PALETTE.terrainShade;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let i = 0; i < s.terrain.length; i++) {
    const p = s.terrain[i];
    if (i === 0) ctx.moveTo(p.x, p.y + 1);
    else         ctx.lineTo(p.x, p.y + 1);
  }
  ctx.stroke();
}

function drawPad(ctx, s) {
  const pad = s.cfg;
  ctx.fillStyle = PALETTE.pad;
  ctx.fillRect(pad.padX, pad.padY - 2, pad.padW, 4);
  ctx.fillStyle = PALETTE.padStripe;
  for (let x = pad.padX + 2; x < pad.padX + pad.padW - 2; x += 6) {
    ctx.fillRect(x, pad.padY - 2, 3, 2);
  }
  // Tall poles at the corners.
  ctx.fillStyle = '#f8f5e8';
  ctx.fillRect(pad.padX,          pad.padY - 14, 2, 12);
  ctx.fillRect(pad.padX + pad.padW - 2, pad.padY - 14, 2, 12);
}

function drawLander(ctx, L, thrusting) {
  ctx.save();
  ctx.translate(L.x, L.y);
  ctx.rotate(L.angle);
  // Body
  ctx.fillStyle = PALETTE.landerDark;
  ctx.fillRect(-7, -5, 14, 10);
  ctx.fillStyle = PALETTE.lander;
  ctx.fillRect(-6, -7, 12, 10);
  // Window
  ctx.fillStyle = PALETTE.landerWindow;
  ctx.fillRect(-2, -5, 4, 4);
  // Landing legs
  ctx.fillStyle = PALETTE.landerDark;
  ctx.fillRect(-8, 5, 2, 4);
  ctx.fillRect(6,  5, 2, 4);
  ctx.fillRect(-9, 8, 4, 2);
  ctx.fillRect(5,  8, 4, 2);
  // Thrust flame (alpha pulse from tickCount-ish randomness via L.angle).
  if (thrusting) {
    const f1 = Math.random() < 0.5 ? 4 : 6;
    ctx.fillStyle = PALETTE.flame2;
    ctx.fillRect(-3, 8, 6, f1 + 2);
    ctx.fillStyle = PALETTE.flame1;
    ctx.fillRect(-2, 8, 4, f1);
  }
  ctx.restore();
}

function drawWindArrow(ctx, wind) {
  const x = 320, y = 50;
  ctx.fillStyle = PALETTE.hudDim;
  ctx.font = '9px monospace';
  ctx.textAlign = 'center';
  ctx.fillText('WIND', x, y - 8);
  // Horizontal arrow body.
  ctx.fillStyle = wind > 0 ? '#a8d84a' : '#f2cf3f';
  if (wind > 0) {
    ctx.fillRect(x - 10, y, 16, 2);
    ctx.fillRect(x + 4,  y - 2, 2, 6);
    ctx.fillRect(x + 6,  y - 1, 2, 4);
    ctx.fillRect(x + 8,  y,     2, 2);
  } else {
    ctx.fillRect(x - 6, y, 16, 2);
    ctx.fillRect(x - 6, y - 2, 2, 6);
    ctx.fillRect(x - 8, y - 1, 2, 4);
    ctx.fillRect(x - 10, y,    2, 2);
  }
}

function drawHud(ctx, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.font = '10px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  // Fuel bar.
  const fuelMax = s.cfg.fuel;
  const fuelFrac = Math.max(0, s.fuel / fuelMax);
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText('FUEL', 6, 11);
  const fbX = 36, fbW = 70;
  ctx.fillStyle = '#222842';
  ctx.fillRect(fbX, 6, fbW, 10);
  ctx.fillStyle = fuelFrac < 0.25 ? PALETTE.hudWarn : PALETTE.hudOk;
  ctx.fillRect(fbX + 1, 7, ((fbW - 2) * fuelFrac) | 0, 8);
  // Stats.
  ctx.fillStyle = PALETTE.hudText;
  const vy = s.lander.vy;
  const vx = s.lander.vx;
  const tilt = s.lander.angle;
  const vyTxt = vy.toFixed(0);
  const vxTxt = vx.toFixed(0);
  ctx.textAlign = 'left';
  ctx.fillStyle = Math.abs(vy) > 28 ? PALETTE.hudWarn : PALETTE.hudText;
  ctx.fillText(`vY ${vyTxt}`, 116, 11);
  ctx.fillStyle = Math.abs(vx) > 18 ? PALETTE.hudWarn : PALETTE.hudText;
  ctx.fillText(`vX ${vxTxt}`, 168, 11);
  ctx.fillStyle = Math.abs(tilt) > 0.25 ? PALETTE.hudWarn : PALETTE.hudText;
  ctx.fillText(`TILT ${(tilt * 57.3).toFixed(0)}°`, 218, 11);
  ctx.textAlign = 'left';
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText('L' + (s.levelIndex + 1), 6, 25);
  ctx.fillStyle = PALETTE.hudDim;
  ctx.fillText(s.cfg.name[0], 36, 25);
}

function drawControlPad(ctx, hits) {
  const y = 410, h = 60;
  ctx.fillStyle = 'rgba(13, 18, 40, 0.85)';
  ctx.fillRect(0, y, 360, h);
  function btn(x, w, color, label) {
    ctx.fillStyle = color;
    ctx.fillRect(x, y + 4, w, h - 8);
    ctx.fillStyle = '#0c1230';
    ctx.fillRect(x, y + 4, w, 2);
    ctx.fillStyle = '#f8f5e8';
    ctx.font = 'bold 18px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(label, x + w / 2, y + h / 2);
    return { x, y: y + 4, w, h: h - 8 };
  }
  hits.length = 0;
  hits.push({ kind: 'rotL',   ...btn(8,   100, '#28315c', '◄') });
  hits.push({ kind: 'thrust', ...btn(124, 112, '#54c47c', '▲') });
  hits.push({ kind: 'rotR',   ...btn(252, 100, '#28315c', '►') });
}
