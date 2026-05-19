// Pixel Crate Cosmos - pixel art. Tiles and entities draw into a square of
// side `s` at canvas pixel (px, py); a tile is an 8x8 sub-grid.

function cc(ctx, px, py, gx, gy, gw, gh, u, c) {
  ctx.fillStyle = c;
  ctx.fillRect(Math.round(px + gx * u), Math.round(py + gy * u),
    Math.ceil(gw * u), Math.ceil(gh * u));
}

function drawFloor(ctx, px, py, s) {
  const u = s / 8;
  cc(ctx, px, py, 0, 0, 8, 8, u, '#222a3c');
  cc(ctx, px, py, 0, 0, 8, 1, u, '#2b3450');
  cc(ctx, px, py, 0, 7, 8, 1, u, '#1a2030');
  cc(ctx, px, py, 1, 1, 2, 2, u, '#283150');
}

function drawWall(ctx, px, py, s) {
  const u = s / 8;
  cc(ctx, px, py, 0, 0, 8, 8, u, '#4a5170');
  cc(ctx, px, py, 0, 0, 8, 2, u, '#5e6890');
  cc(ctx, px, py, 0, 6, 8, 2, u, '#363c54');
  cc(ctx, px, py, 1, 3, 2, 2, u, '#565d80');
  cc(ctx, px, py, 5, 2, 2, 2, u, '#565d80');
  cc(ctx, px, py, 4, 5, 2, 1, u, '#3f4664');
}

function drawIce(ctx, px, py, s, t) {
  const u = s / 8;
  cc(ctx, px, py, 0, 0, 8, 8, u, '#2c5a78');
  cc(ctx, px, py, 0, 0, 8, 1, u, '#5fb8d8');
  cc(ctx, px, py, 0, 7, 8, 1, u, '#1d3f56');
  const sh = (Math.sin(t * 2) + 1) * 1.4;
  cc(ctx, px, py, 1 + sh, 1, 2, 1, u, '#bfeeff');
  cc(ctx, px, py, 4, 4 + Math.cos(t * 2) * 0.8, 2, 1, u, '#9fdcf2');
}

function drawSocket(ctx, px, py, s, t) {
  const u = s / 8;
  const pulse = (Math.sin(t * 3) + 1) * 0.5;
  cc(ctx, px, py, 1, 1, 6, 6, u, '#1c3328');
  cc(ctx, px, py, 2, 2, 4, 4, u, '#2f6b48');
  ctx.globalAlpha = 0.4 + pulse * 0.5;
  cc(ctx, px, py, 3, 3, 2, 2, u, '#7dff9f');
  ctx.globalAlpha = 1;
  cc(ctx, px, py, 1, 1, 6, 1, u, '#49d27a');
  cc(ctx, px, py, 1, 6, 6, 1, u, '#49d27a');
}

function drawCrate(ctx, px, py, s, onTarget, t) {
  const u = s / 8;
  const body = onTarget ? '#49d27a' : '#d9a23f';
  const lt = onTarget ? '#7dff9f' : '#f0c463';
  const dk = onTarget ? '#277d4a' : '#9a6c20';
  cc(ctx, px, py, 1, 1, 6, 6, u, dk);
  cc(ctx, px, py, 1, 1, 6, 5, u, body);
  cc(ctx, px, py, 1, 1, 6, 1, u, lt);
  cc(ctx, px, py, 1, 1, 1, 5, u, lt);
  // rivets
  cc(ctx, px, py, 2, 2, 1, 1, u, dk);
  cc(ctx, px, py, 5, 2, 1, 1, u, dk);
  cc(ctx, px, py, 2, 5, 1, 1, u, dk);
  cc(ctx, px, py, 5, 5, 1, 1, u, dk);
  // glowing core in the middle
  const pulse = (Math.sin((t || 0) * 4) + 1) * 0.5;
  ctx.globalAlpha = 0.6 + pulse * 0.4;
  cc(ctx, px, py, 3, 3, 2, 2, u, onTarget ? '#eafff0' : '#fff0c4');
  ctx.globalAlpha = 1;
}

function drawRobot(ctx, px, py, s, facing, t) {
  const u = s / 8;
  const bob = Math.round(Math.sin(t * 4) * 0.4);
  // shadow
  ctx.globalAlpha = 0.3;
  cc(ctx, px, py, 1.5, 6.6, 5, 1.2, u, '#000000');
  ctx.globalAlpha = 1;
  // legs / tread
  cc(ctx, px, py, 1, 6 + bob, 6, 1.4, u, '#2a3046');
  // body
  cc(ctx, px, py, 1.5, 2.4 + bob, 5, 4, u, '#c9d2e6');
  cc(ctx, px, py, 1.5, 2.4 + bob, 5, 1, u, '#eef3fb');
  cc(ctx, px, py, 1.5, 5.4 + bob, 5, 1, u, '#8c95ad');
  // visor - shifts toward facing direction
  const fx = facing === 1 ? 1 : facing === -1 ? -1 : 0;
  const fy = facing === 2 ? 1 : facing === -2 ? -1 : 0;
  cc(ctx, px, py, 2.4, 3.2 + bob, 3.2, 1.8, u, '#10131c');
  cc(ctx, px, py, 3 + fx * 0.9, 3.5 + bob + fy * 0.5, 1.4, 1.1, u, '#5fd9ff');
  // antenna
  cc(ctx, px, py, 3.6, 0.8 + bob, 0.8, 1.7, u, '#8c95ad');
  cc(ctx, px, py, 3.3, 0.3 + bob, 1.4, 0.9, u, '#ff7ad0');
}
