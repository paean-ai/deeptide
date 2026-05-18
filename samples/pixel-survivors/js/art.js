// Pixel Survivors - top-down pixel art

function shade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
function pr(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, Math.ceil(w), Math.ceil(h)); }

function shadowBlob(ctx, x, y, r) {
  ctx.fillStyle = 'rgba(0,0,0,0.3)';
  ctx.beginPath();
  ctx.ellipse(x, y, r, r * 0.42, 0, 0, Math.PI * 2);
  ctx.fill();
}

// --- hero: small armoured survivor, top-down ---------------------------
function drawHero(ctx, x, y, t, face, hitFlash, invuln) {
  const step = Math.sin(t * 11) * 2;
  ctx.save();
  ctx.translate(x, y);
  shadowBlob(ctx, 0, 13, 13);
  if (invuln > 0 && Math.floor(t * 16) % 2) ctx.globalAlpha = 0.45;
  const sk = '#e8b98a', body = '#5b78c4', bD = '#3a4f8c', bL = '#88a0e0', metal = '#c2c9d6';
  // legs
  pr(ctx, -7, 6 + step, 5, 8, bD);
  pr(ctx, 2, 6 - step, 5, 8, bD);
  // body
  pr(ctx, -9, -8, 18, 16, body);
  pr(ctx, -9, -8, 18, 4, bL);
  pr(ctx, -3, -6, 6, 12, metal);
  // head
  pr(ctx, -7, -19, 14, 12, sk);
  pr(ctx, -7, -19, 14, 5, metal);
  // facing eyes
  const ex = face >= 0 ? 1 : -4;
  pr(ctx, ex, -13, 3, 3, '#1a1422');
  pr(ctx, ex + (face >= 0 ? 4 : 0) - (face >= 0 ? 0 : 1), -13, 3, 3, '#1a1422');
  pr(ctx, -1, -23, 4, 4, '#ff5a5a'); // crest
  // sword (points toward facing)
  if (face >= 0) pr(ctx, 9, -4, 4, 16, metal);
  else pr(ctx, -13, -4, 4, 16, metal);
  ctx.restore();
  if (hitFlash > 0) {
    ctx.globalAlpha = hitFlash * 0.7;
    ctx.fillStyle = '#ff5a5a';
    ctx.beginPath(); ctx.arc(x, y, 20, 0, Math.PI * 2); ctx.fill();
    ctx.globalAlpha = 1;
  }
}

// --- enemies -----------------------------------------------------------
function drawEnemy(ctx, sprite, x, y, size, t, color, hpRatio, hitFlash, slowed) {
  ctx.save();
  ctx.translate(x, y);
  const s = size / 20;
  shadowBlob(ctx, 0, size * 0.5, size * 0.5);
  const wob = Math.sin(t * 8) * 1.5 * s;
  const draw = ENEMY_ART[sprite] || ENEMY_ART.zombie;
  draw(ctx, s, color, t, wob);
  if (slowed > 0) {
    ctx.globalAlpha = 0.5;
    ctx.strokeStyle = '#9ce6ff';
    ctx.lineWidth = 2;
    ctx.beginPath(); ctx.arc(0, 0, size * 0.6, 0, Math.PI * 2); ctx.stroke();
    ctx.globalAlpha = 1;
  }
  if (hitFlash > 0) {
    ctx.globalAlpha = hitFlash;
    pr(ctx, -size * 0.7, -size * 0.9, size * 1.4, size * 1.5, '#fff');
    ctx.globalAlpha = 1;
  }
  ctx.restore();
  // hp bar
  if (hpRatio < 1 && hpRatio > 0) {
    const w = size * 1.1;
    pr(ctx, x - w / 2 - 1, y - size * 0.85, w + 2, 5, 'rgba(0,0,0,0.7)');
    pr(ctx, x - w / 2, y - size * 0.85 + 1, w * hpRatio, 3,
      hpRatio > 0.4 ? '#5fe07a' : '#ff5a5a');
  }
}

const ENEMY_ART = {
  bat(ctx, s, c, t, wob) {
    const flap = Math.sin(t * 14) * 6 * s;
    pr(ctx, -16 * s, -4 * s - flap, 11 * s, 7 * s, shade(c, -25));
    pr(ctx, 5 * s, -4 * s - flap, 11 * s, 7 * s, shade(c, -25));
    pr(ctx, -7 * s, -8 * s + wob, 14 * s, 12 * s, c);
    pr(ctx, -7 * s, -8 * s + wob, 14 * s, 4 * s, shade(c, 30));
    pr(ctx, -5 * s, -3 * s + wob, 3 * s, 3 * s, '#ffd34d');
    pr(ctx, 2 * s, -3 * s + wob, 3 * s, 3 * s, '#ffd34d');
  },
  skel(ctx, s, c, t, wob) {
    pr(ctx, -6 * s, 2 * s, 4 * s, 8 * s, shade(c, -30));
    pr(ctx, 2 * s, 2 * s, 4 * s, 8 * s, shade(c, -30));
    pr(ctx, -7 * s, -7 * s + wob, 14 * s, 11 * s, c);
    pr(ctx, -8 * s, -18 * s + wob, 16 * s, 12 * s, c);
    pr(ctx, -4 * s, -13 * s + wob, 3 * s, 4 * s, '#1a1422');
    pr(ctx, 2 * s, -13 * s + wob, 3 * s, 4 * s, '#1a1422');
    pr(ctx, -3 * s, -6 * s + wob, 7 * s, 2 * s, shade(c, -40));
  },
  zombie(ctx, s, c, t, wob) {
    pr(ctx, -7 * s, 3 * s, 5 * s, 9 * s, shade(c, -35));
    pr(ctx, 2 * s, 3 * s, 5 * s, 9 * s, shade(c, -35));
    pr(ctx, -9 * s, -8 * s + wob, 18 * s, 14 * s, c);
    pr(ctx, -9 * s, -8 * s + wob, 18 * s, 4 * s, shade(c, 30));
    pr(ctx, -7 * s, -20 * s + wob, 14 * s, 13 * s, shade(c, 18));
    pr(ctx, -4 * s, -15 * s + wob, 3 * s, 4 * s, '#1a1422');
    pr(ctx, 2 * s, -15 * s + wob, 3 * s, 4 * s, '#1a1422');
    pr(ctx, -3 * s, -8 * s + wob, 7 * s, 2 * s, '#7d1f16');
  },
  ghost(ctx, s, c, t, wob) {
    const drift = Math.sin(t * 4) * 2 * s;
    ctx.globalAlpha = 0.86;
    pr(ctx, -8 * s, -12 * s + drift, 16 * s, 16 * s, c);
    for (let i = 0; i < 4; i++)
      pr(ctx, -8 * s + i * 4 * s, 4 * s + drift + (i % 2) * 3 * s, 4 * s, 4 * s, c);
    pr(ctx, -8 * s, -12 * s + drift, 16 * s, 5 * s, shade(c, 30));
    pr(ctx, -4 * s, -6 * s + drift, 3 * s, 4 * s, '#1a1422');
    pr(ctx, 2 * s, -6 * s + drift, 3 * s, 4 * s, '#1a1422');
    ctx.globalAlpha = 1;
  },
  slime(ctx, s, c, t, wob) {
    const sq = 1 + Math.sin(t * 5) * 0.1;
    const w = 22 * s * sq, h = 16 * s / sq;
    pr(ctx, -w / 2, -h, w, h, c);
    pr(ctx, -w / 2, -h, w, h * 0.4, shade(c, 35));
    pr(ctx, -5 * s, -h * 0.6, 4 * s, 5 * s, '#fff');
    pr(ctx, 2 * s, -h * 0.6, 4 * s, 5 * s, '#fff');
    pr(ctx, -4 * s, -h * 0.5, 2 * s, 3 * s, '#1a1422');
    pr(ctx, 3 * s, -h * 0.5, 2 * s, 3 * s, '#1a1422');
  },
  brute(ctx, s, c, t, wob) {
    pr(ctx, -11 * s, 6 * s, 8 * s, 12 * s, shade(c, -38));
    pr(ctx, 3 * s, 6 * s, 8 * s, 12 * s, shade(c, -38));
    pr(ctx, -15 * s, -12 * s + wob, 30 * s, 22 * s, c);
    pr(ctx, -15 * s, -12 * s + wob, 30 * s, 6 * s, shade(c, 32));
    pr(ctx, -19 * s, -8 * s + wob, 7 * s, 16 * s, shade(c, -12));
    pr(ctx, 12 * s, -8 * s + wob, 7 * s, 16 * s, shade(c, -12));
    pr(ctx, -10 * s, -26 * s + wob, 20 * s, 15 * s, c);
    pr(ctx, -6 * s, -20 * s + wob, 4 * s, 5 * s, '#ffd34d');
    pr(ctx, 2 * s, -20 * s + wob, 4 * s, 5 * s, '#ffd34d');
    pr(ctx, -6 * s, -12 * s + wob, 12 * s, 3 * s, '#1a1422');
  },
};

// --- ground ------------------------------------------------------------
function drawGround(ctx, camX, camY, w, h) {
  const TS = 64;
  const x0 = Math.floor((camX - w / 2) / TS) * TS;
  const y0 = Math.floor((camY - h / 2) / TS) * TS;
  for (let gx = x0; gx < camX + w / 2 + TS; gx += TS) {
    for (let gy = y0; gy < camY + h / 2 + TS; gy += TS) {
      const sx = gx - camX + w / 2, sy = gy - camY + h / 2;
      const checker = ((gx / TS) + (gy / TS)) & 1;
      ctx.fillStyle = checker ? '#243024' : '#2a382a';
      ctx.fillRect(sx, sy, TS, TS);
      const seed = ((gx * 73 + gy * 31) >> 4) % 11;
      if (seed < 3) {
        ctx.fillStyle = checker ? '#2e3a2e' : '#324232';
        ctx.fillRect(sx + 12 + seed * 9, sy + 18 + seed * 7, 7, 7);
      }
      if (seed === 5) {
        ctx.fillStyle = '#3a4a3a';
        ctx.fillRect(sx + 40, sy + 12, 9, 4);
      }
    }
  }
}

function drawGem(ctx, x, y, value, t) {
  const pulse = 1 + Math.sin(t * 6) * 0.12;
  const r = (value >= 5 ? 7 : value >= 2 ? 5 : 4) * pulse;
  const col = value >= 5 ? '#ff5ad0' : value >= 2 ? '#5fd0ff' : '#7fe07a';
  ctx.fillStyle = col;
  ctx.beginPath();
  ctx.moveTo(x, y - r); ctx.lineTo(x + r, y); ctx.lineTo(x, y + r); ctx.lineTo(x - r, y);
  ctx.closePath(); ctx.fill();
  ctx.fillStyle = '#fff';
  ctx.fillRect(x - 1, y - r + 2, 2, 2);
}
function drawCoin(ctx, x, y, t) {
  const w = 5 + Math.sin(t * 7) * 2.5;
  ctx.fillStyle = '#ffd34d';
  ctx.fillRect(x - w / 2, y - 6, w, 11);
  ctx.fillStyle = '#a8780f';
  ctx.fillRect(x - w / 2, y - 6, Math.max(1, w * 0.3), 11);
}
function drawHeart(ctx, x, y) {
  ctx.fillStyle = '#ff5a7a';
  ctx.fillRect(x - 5, y - 3, 4, 5); ctx.fillRect(x + 1, y - 3, 4, 5);
  ctx.fillRect(x - 5, y, 10, 3); ctx.fillRect(x - 3, y + 3, 6, 2); ctx.fillRect(x - 1, y + 5, 2, 2);
}
function drawChest(ctx, x, y, t) {
  const pulse = Math.sin(t * 4) * 2;
  ctx.fillStyle = '#8a5a2a';
  ctx.fillRect(x - 11, y - 8 + pulse, 22, 16);
  ctx.fillStyle = '#caa14a';
  ctx.fillRect(x - 11, y - 8 + pulse, 22, 5);
  ctx.fillStyle = '#ffd34d';
  ctx.fillRect(x - 3, y - 4 + pulse, 6, 6);
}
