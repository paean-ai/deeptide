// Pixel Shadow Heist - pixel art for tiles, the thief, guards, vision cones.

function sh(ctx, px, py, gx, gy, gw, gh, u, c) {
  ctx.fillStyle = c;
  ctx.fillRect(Math.round(px + gx * u), Math.round(py + gy * u),
    Math.ceil(gw * u), Math.ceil(gh * u));
}

function drawFloor(ctx, px, py, s) {
  const u = s / 8;
  sh(ctx, px, py, 0, 0, 8, 8, u, '#1c2230');
  sh(ctx, px, py, 0, 0, 8, 1, u, '#242b3c');
  sh(ctx, px, py, 1, 1, 1, 1, u, '#28304200');
  sh(ctx, px, py, 2, 4, 2, 1, u, '#222a3a');
}

function drawWall(ctx, px, py, s) {
  const u = s / 8;
  sh(ctx, px, py, 0, 0, 8, 8, u, '#46506a');
  sh(ctx, px, py, 0, 0, 8, 2, u, '#5a6586');
  sh(ctx, px, py, 0, 6, 8, 2, u, '#343c52');
  sh(ctx, px, py, 4, 0, 1, 8, u, '#3c4458');
  sh(ctx, px, py, 0, 4, 8, 1, u, '#3c4458');
}

function drawExit(ctx, px, py, s, t) {
  const u = s / 8;
  sh(ctx, px, py, 0, 0, 8, 8, u, '#1c2230');
  const pulse = (Math.sin(t * 3) + 1) * 0.5;
  ctx.globalAlpha = 0.4 + pulse * 0.5;
  sh(ctx, px, py, 1, 1, 6, 6, u, '#f4c85a');
  ctx.globalAlpha = 1;
  sh(ctx, px, py, 2, 2, 4, 4, u, '#1c2230');
  sh(ctx, px, py, 3, 3, 2, 2, u, '#ffe9a0');
}

function drawVision(ctx, px, py, s) {
  ctx.fillStyle = 'rgba(255,90,90,0.26)';
  ctx.fillRect(px, py, s, s);
  ctx.fillStyle = 'rgba(255,90,90,0.5)';
  ctx.fillRect(px, py, s, 2);
  ctx.fillRect(px, py, 2, s);
}

function drawThief(ctx, px, py, s, t) {
  const u = s / 8;
  const bob = Math.round(Math.sin(t * 4) * 0.3);
  sh(ctx, px, py, 2, 1 + bob, 4, 3, u, '#2f9d6a');   // hood
  sh(ctx, px, py, 2, 1 + bob, 4, 1, u, '#46c98c');
  sh(ctx, px, py, 3, 3 + bob, 2, 1, u, '#1a1620');   // face shadow
  sh(ctx, px, py, 2, 4 + bob, 4, 3, u, '#236f4d');   // cloak
  sh(ctx, px, py, 2, 4 + bob, 1, 3, u, '#2f9d6a');
  sh(ctx, px, py, 3, 6 + bob, 1, 1, u, '#16121c');   // feet
  sh(ctx, px, py, 4, 6 + bob, 1, 1, u, '#16121c');
}

function drawGuard(ctx, px, py, s, facing, t) {
  const u = s / 8;
  const R = (x, y, w, h, c) => sh(ctx, px, py, x, y, w, h, u, c);
  R(2, 1, 4, 3, '#c0473f');       // helmet
  R(2, 1, 4, 1, '#e0635a');
  R(2, 4, 4, 3, '#9aa6bc');       // armor
  R(2, 4, 4, 1, '#c2ccdc');
  R(3, 6, 1, 1, '#2a3040');
  R(4, 6, 1, 1, '#2a3040');
  // facing indicator - a visor slit toward the look direction
  const fx = facing.x, fy = facing.y;
  R(3.4 + fx * 1.8, 2.4 + fy * 1.3, 1.4, 1.4, '#ffd34d');
}
