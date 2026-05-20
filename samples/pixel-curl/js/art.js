// Pixel-art rendering for Pixel Curl. 360x480 world units.

const PALETTE = {
  bg:        '#0d1228',
  ice:       '#cce6ff',
  iceShade:  '#9cc1e0',
  iceLine:   '#7fa9c8',
  edge:      '#1c2240',
  ring4:     '#fff',         // outer (white) — ring index 0 (largest)
  ring3:     '#4a9be8',      // blue
  ring2:     '#fff',         // white again
  ring1:     '#e8554f',      // red bull
  player:    '#e8554f',
  playerDark:'#a8373a',
  ai:        '#4a9be8',
  aiDark:    '#1f5494',
  stoneRim:  '#070b16',
  stoneHi:   '#fff',
  aimGuide:  '#f7e69a',
  aimMax:    '#e8554f',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function drawScene(ctx, s, lang) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, 360, 480);
  drawSheet(ctx);
  drawHouse(ctx);
  drawStones(ctx, s);
  drawAimGuide(ctx, s);
  drawSpawnPad(ctx, s);
  drawHud(ctx, s, lang);
}

function drawSheet(ctx) {
  ctx.fillStyle = PALETTE.edge;
  ctx.fillRect(SHEET_X0 - 4, SHEET_Y0 - 4, (SHEET_X1 - SHEET_X0) + 8, (SHEET_Y1 - SHEET_Y0) + 8);
  ctx.fillStyle = PALETTE.ice;
  ctx.fillRect(SHEET_X0, SHEET_Y0, SHEET_X1 - SHEET_X0, SHEET_Y1 - SHEET_Y0);
  // Subtle horizontal lines (scribed centre line + hog lines).
  ctx.fillStyle = PALETTE.iceLine;
  // Centre line vertical (down the sheet).
  ctx.fillRect((SHEET_X0 + SHEET_X1) / 2 - 1, SHEET_Y0, 2, SHEET_Y1 - SHEET_Y0);
  // Hog line just above SPAWN_Y.
  ctx.fillRect(SHEET_X0, SPAWN_Y - 40, SHEET_X1 - SHEET_X0, 1);
  ctx.fillRect(SHEET_X0, HOUSE_Y + HOUSE_R + 18, SHEET_X1 - SHEET_X0, 1);
}

function drawHouse(ctx) {
  for (let i = 0; i < RINGS.length; i++) {
    const r = RINGS[i];
    ctx.fillStyle = i === 0 ? '#fff' : i === 1 ? PALETTE.ring3 : i === 2 ? '#fff' : PALETTE.ring1;
    ctx.beginPath(); ctx.arc(HOUSE_X, HOUSE_Y, r, 0, Math.PI * 2); ctx.fill();
  }
  // Centre dot.
  ctx.fillStyle = '#0c1230';
  ctx.fillRect((HOUSE_X | 0) - 1, (HOUSE_Y | 0) - 1, 2, 2);
}

function drawStones(ctx, s) {
  for (const st of s.stones) {
    if (!st.alive) continue;
    drawStone(ctx, st);
  }
}
function drawStone(ctx, st) {
  const base = st.owner === 'p' ? PALETTE.player : PALETTE.ai;
  const dark = st.owner === 'p' ? PALETTE.playerDark : PALETTE.aiDark;
  ctx.fillStyle = 'rgba(0,0,0,0.25)';
  ctx.beginPath(); ctx.arc(st.x + 1, st.y + 2, STONE_R + 1, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.stoneRim;
  ctx.beginPath(); ctx.arc(st.x, st.y, STONE_R + 1, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = dark;
  ctx.beginPath(); ctx.arc(st.x, st.y, STONE_R, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = base;
  ctx.fillRect((st.x | 0) - 4, (st.y | 0) - 2, 8, 4);
  ctx.fillStyle = PALETTE.stoneHi;
  ctx.fillRect((st.x | 0) - 2, (st.y | 0) - 4, 2, 1);
}

function drawSpawnPad(ctx, s) {
  // Marker where the next player stone will spawn.
  if (s.over || s.flying || s.turn !== 'p' || s.playerLeft <= 0) return;
  const x = (SHEET_X0 + SHEET_X1) / 2, y = SPAWN_Y;
  ctx.strokeStyle = PALETTE.player;
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.arc(x, y, STONE_R + 3, 0, Math.PI * 2); ctx.stroke();
}

function drawAimGuide(ctx, s) {
  if (!s.aim || s.turn !== 'p') return;
  const sx = (SHEET_X0 + SHEET_X1) / 2, sy = SPAWN_Y;
  const dx = s.aim.x - sx, dy = s.aim.y - sy;
  const len = Math.hypot(dx, dy);
  if (len < 10) return;
  const ux = dx / len, uy = dy / len;
  ctx.fillStyle = PALETTE.aimGuide;
  for (let d = 12; d < Math.min(120, len); d += 8) {
    ctx.fillRect(((sx + ux * d - 1) | 0), ((sy + uy * d - 1) | 0), 2, 2);
  }
  // Forward (slingshot) preview.
  ctx.fillStyle = len > 110 ? PALETTE.aimMax : PALETTE.aimGuide;
  const fwdLen = Math.min(120, len) * 1.4;
  for (let d = 14; d < fwdLen; d += 10) {
    ctx.fillRect(((sx - ux * d - 1) | 0), ((sy - uy * d - 1) | 0), 2, 2);
  }
  ctx.fillRect(((sx + ux * Math.min(120, len) - 3) | 0), ((sy + uy * Math.min(120, len) - 3) | 0), 6, 6);
}

function drawHud(ctx, s, lang) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  // Stones remaining indicators.
  let cx = 110;
  for (let i = 0; i < STONES_PER_SIDE; i++) {
    ctx.fillStyle = (i < s.playerLeft) ? PALETTE.player : '#3a4274';
    ctx.fillRect(cx + i * 10, 11, 6, 10);
  }
  cx = 200;
  for (let i = 0; i < STONES_PER_SIDE; i++) {
    ctx.fillStyle = (i < s.aiLeft) ? PALETTE.ai : '#3a4274';
    ctx.fillRect(cx + i * 10, 11, 6, 10);
  }
  ctx.textAlign = 'right';
  ctx.fillStyle = s.turn === 'p' ? PALETTE.player : PALETTE.ai;
  ctx.fillText(s.turn === 'p' ? t(lang, 'yourTurn') : t(lang, 'aiTurn'), 352, 16);
}
