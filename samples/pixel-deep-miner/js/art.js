// Pixel Deep Miner - pixel art for terrain blocks, the mining rig and the shop.

function dmShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// Draw one terrain tile at canvas (ox, oy). `drill` 0..1 shows crack damage.
function drawBlock(ctx, ox, oy, id, variant, drill, t) {
  const b = BLOCKS[id];
  if (!b) return;
  ctx.fillStyle = b.color;
  ctx.fillRect(ox, oy, TILE, TILE);
  // top lip + bevel
  ctx.fillStyle = b.top || dmShade(b.color, 22);
  ctx.fillRect(ox, oy, TILE, 4);
  ctx.fillStyle = dmShade(b.color, -28);
  ctx.fillRect(ox, oy + TILE - 3, TILE, 3);
  ctx.fillRect(ox + TILE - 3, oy, 3, TILE);

  if (id === B_LAVA) {
    ctx.fillStyle = '#ffb454';
    for (let i = 0; i < 4; i++) {
      const bx = ox + 5 + ((i * 11 + variant * 7) % (TILE - 12));
      const by = oy + 7 + Math.sin(t * 3 + i + variant) * 3;
      ctx.fillRect(bx, by, 5, 5);
    }
  } else if (b.ore) {
    const o = ORES[b.ore];
    const spots = [[7, 9], [19, 7], [12, 18], [22, 20], [6, 21]];
    for (let i = 0; i < 4; i++) {
      const s = spots[(i + variant) % spots.length];
      ctx.fillStyle = o.color;
      ctx.fillRect(ox + s[0], oy + s[1], 6, 6);
      ctx.fillStyle = o.glow;
      ctx.fillRect(ox + s[0], oy + s[1], 3, 3);
    }
  } else if (id !== B_BEDROCK) {
    // speckle texture
    ctx.fillStyle = dmShade(b.color, 16);
    const sp = [[6, 8], [20, 12], [11, 22], [24, 24]];
    const s = sp[variant % sp.length];
    ctx.fillRect(ox + s[0], oy + s[1], 3, 3);
    ctx.fillStyle = dmShade(b.color, -22);
    ctx.fillRect(ox + ((s[0] + 13) % (TILE - 4)), oy + ((s[1] + 9) % (TILE - 4)), 3, 3);
  }
  // drilling cracks
  if (drill > 0) {
    ctx.strokeStyle = 'rgba(8,8,12,0.75)';
    ctx.lineWidth = 2;
    const cx = ox + TILE / 2, cy = oy + TILE / 2;
    const arms = Math.ceil(drill * 5);
    for (let i = 0; i < arms; i++) {
      const a = i * 2.4 + 0.5;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx + Math.cos(a) * TILE * 0.5 * drill, cy + Math.sin(a) * TILE * 0.5 * drill);
      ctx.stroke();
    }
  }
}

// The mining rig. `face` -1/1, `dir` of action, `drilling` bool, `t` clock.
function drawMiner(ctx, ox, oy, face, drilling, t) {
  const S = TILE / 16;
  const P = (x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(ox + x * S, oy + y * S, w * S, h * S); };
  ctx.save();
  // treads
  P(1, 12, 14, 4, '#23242c');
  for (let i = 0; i < 4; i++) P(2 + i * 3.4, 13, 2, 2, '#3a3c47');
  // hull
  P(2, 4, 12, 9, '#d8772f');
  P(2, 4, 12, 2, '#ffa54d');
  P(2, 11, 12, 2, '#9c5018');
  // cockpit glass
  P(face > 0 ? 8 : 4, 6, 5, 4, '#7fd9ff');
  P(face > 0 ? 8 : 4, 6, 5, 1, '#d6f4ff');
  // drill on the facing side
  const wob = drilling ? Math.round(Math.sin(t * 40) * S) : 0;
  if (face > 0) {
    P(14 + wob / S, 7, 2, 3, '#c9ced8');
    P(15 + wob / S, 6, 2, 5, '#e6ebf2');
    P(16 + wob / S, 8, 2, 1, '#9aa0ad');
  } else {
    P(0 - wob / S, 7, 2, 3, '#c9ced8');
    P(-1 - wob / S, 6, 2, 5, '#e6ebf2');
    P(-2 - wob / S, 8, 2, 1, '#9aa0ad');
  }
  // headlamp glow
  ctx.globalAlpha = 0.5;
  P(face > 0 ? 13 : 1, 6, 2, 2, '#fff2c0');
  ctx.globalAlpha = 1;
  ctx.restore();
}

// Surface shop hut.
function drawShop(ctx, ox, oy) {
  const S = TILE / 16;
  const P = (x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(ox + x * S, oy + y * S, w * S, h * S); };
  P(1, 6, 14, 10, '#caa06f');
  P(1, 6, 14, 2, '#e3bd88');
  P(0, 3, 16, 4, '#b8443c');     // roof
  P(0, 3, 16, 1, '#d8645c');
  P(6, 9, 5, 7, '#5a3f2c');      // door
  P(2, 9, 3, 3, '#7fd9ff');      // window
  P(11, 9, 3, 3, '#7fd9ff');
  P(4, 0, 2, 4, '#8c5f38');      // chimney
}
