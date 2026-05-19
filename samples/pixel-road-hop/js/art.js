// Pixel Road Hop - lane, car, log and player rendering.

function drawRowBand(ctx, row, sy) {
  if (row.type === 'grass') {
    ctx.fillStyle = (row.shade ? '#4ba84a' : '#56b855');
    ctx.fillRect(0, sy, VW, TILE);
    ctx.fillStyle = 'rgba(255,255,255,0.05)';
    for (let c = 0; c < COLS; c++) ctx.fillRect(c * TILE + 6, sy + 6, 5, 5);
  } else if (row.type === 'road') {
    ctx.fillStyle = '#3a3f4a';
    ctx.fillRect(0, sy, VW, TILE);
    ctx.fillStyle = '#21252e';
    ctx.fillRect(0, sy, VW, 3);
    ctx.fillRect(0, sy + TILE - 3, VW, 3);
    ctx.fillStyle = 'rgba(242,207,63,0.5)';            // lane dashes
    for (let x = 6; x < VW; x += 26) ctx.fillRect(x, sy + TILE / 2 - 2, 14, 4);
  } else {
    ctx.fillStyle = '#2f6fd0';                          // river
    ctx.fillRect(0, sy, VW, TILE);
    ctx.fillStyle = '#3f86e8';
    for (let x = 0; x < VW; x += 18) ctx.fillRect(x + ((sy * 3) % 18), sy + 9, 9, 3);
    ctx.fillStyle = '#1f4f9a';
    ctx.fillRect(0, sy, VW, 3);
  }
}

function drawTree(ctx, cx, sy) {
  ctx.fillStyle = '#5a3f24';
  ctx.fillRect(cx - 4, sy + TILE - 16, 8, 14);
  ctx.fillStyle = '#1f6f33';
  ctx.fillRect(cx - 14, sy + 2, 28, 22);
  ctx.fillStyle = '#2f8f47';
  ctx.fillRect(cx - 14, sy + 2, 28, 7);
  ctx.fillStyle = '#155226';
  ctx.fillRect(cx - 14, sy + 19, 28, 5);
}

function drawCar(ctx, x, sy, dir, color) {
  const w = TILE - 8, h = TILE - 14;
  const px = x, py = sy + 7;
  ctx.fillStyle = '#11141c';
  ctx.fillRect(px - 1, py - 1, w + 2, h + 2);
  ctx.fillStyle = color;
  ctx.fillRect(px, py, w, h);
  ctx.fillStyle = 'rgba(255,255,255,0.85)';            // windshield
  ctx.fillRect(dir > 0 ? px + w - 11 : px + 4, py + 4, 7, h - 8);
  ctx.fillStyle = '#ffe14d';                            // headlights
  const hx = dir > 0 ? px + w - 3 : px;
  ctx.fillRect(hx, py + 2, 3, 3);
  ctx.fillRect(hx, py + h - 5, 3, 3);
}

function drawLog(ctx, x, sy, w) {
  ctx.fillStyle = '#6b4a2a';
  ctx.fillRect(x, sy + 6, w, TILE - 12);
  ctx.fillStyle = '#85633c';
  ctx.fillRect(x, sy + 6, w, 5);
  ctx.fillStyle = '#553a20';
  ctx.fillRect(x, sy + TILE - 11, w, 5);
  ctx.fillStyle = '#4a3018';
  ctx.fillRect(x + 3, sy + 13, 4, 4);
  ctx.fillRect(x + w - 8, sy + 16, 4, 4);
}

function drawPlayer(ctx, cx, cy, hopZ, dead) {
  const y = cy - hopZ;
  // shadow
  ctx.fillStyle = 'rgba(0,0,0,0.28)';
  ctx.fillRect(cx - 11, cy + 9, 22, 6);
  if (dead) {
    ctx.fillStyle = '#d8b94a';
    ctx.fillRect(cx - 13, cy + 2, 26, 8);
    return;
  }
  // body
  ctx.fillStyle = '#f2cf3f';
  ctx.fillRect(cx - 10, y - 10, 20, 20);
  ctx.fillStyle = '#ffe884';
  ctx.fillRect(cx - 10, y - 10, 20, 7);
  // wings
  ctx.fillStyle = '#e0b631';
  ctx.fillRect(cx - 13, y - 4, 4, 9);
  ctx.fillRect(cx + 9, y - 4, 4, 9);
  // eyes + beak
  ctx.fillStyle = '#16181d';
  ctx.fillRect(cx - 6, y - 6, 4, 4);
  ctx.fillRect(cx + 2, y - 6, 4, 4);
  ctx.fillStyle = '#ef7d3e';
  ctx.fillRect(cx - 3, y, 6, 4);
}
