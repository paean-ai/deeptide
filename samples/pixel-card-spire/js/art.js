// Pixel Card Spire - pixel art for the battle scene

function shade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// pixel-rect helper
function pr(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }

// soft shadow under a unit
function groundShadow(ctx, x, y, w) {
  ctx.fillStyle = 'rgba(0,0,0,0.32)';
  ctx.beginPath();
  ctx.ellipse(x, y, w * 0.55, w * 0.2, 0, 0, Math.PI * 2);
  ctx.fill();
}

// --- player: armoured vanguard, facing right --------------------------
function drawPlayer(ctx, x, y, s, time, hitFlash) {
  const bob = Math.sin(time * 2.2) * 1.6 * s;
  ctx.save();
  ctx.translate(x, y + bob);
  groundShadow(ctx, 0, 2, 26 * s);
  const sk = '#e8b98a', ar = '#5b78c4', arD = '#3a4f8c', arL = '#88a0e0';
  const metal = '#c2c9d6', mD = '#7a8294';
  // legs
  pr(ctx, -9 * s, 6 * s, 7 * s, 12 * s, arD);
  pr(ctx, 3 * s, 6 * s, 7 * s, 12 * s, arD);
  pr(ctx, -9 * s, 16 * s, 8 * s, 4 * s, '#2a2535');
  pr(ctx, 2 * s, 16 * s, 8 * s, 4 * s, '#2a2535');
  // torso
  pr(ctx, -11 * s, -10 * s, 22 * s, 18 * s, ar);
  pr(ctx, -11 * s, -10 * s, 22 * s, 4 * s, arL);
  pr(ctx, -4 * s, -8 * s, 8 * s, 14 * s, metal);
  pr(ctx, -4 * s, -8 * s, 8 * s, 3 * s, '#e8eef8');
  // head + helm
  pr(ctx, -7 * s, -24 * s, 14 * s, 14 * s, sk);
  pr(ctx, -7 * s, -24 * s, 14 * s, 6 * s, metal);
  pr(ctx, -7 * s, -18 * s, 14 * s, 2 * s, mD);
  pr(ctx, -4 * s, -15 * s, 3 * s, 3 * s, '#1a1422'); // eye
  pr(ctx, 2 * s, -15 * s, 3 * s, 3 * s, '#1a1422');
  pr(ctx, 0 * s, -28 * s, 4 * s, 5 * s, '#ff5a5a'); // crest
  // shield arm (left/front)
  pr(ctx, -20 * s, -8 * s, 11 * s, 20 * s, '#caa14a');
  pr(ctx, -20 * s, -8 * s, 11 * s, 4 * s, '#e8c66a');
  pr(ctx, -17 * s, -2 * s, 5 * s, 6 * s, '#7d5f1f');
  // sword arm (right/back) + blade
  pr(ctx, 10 * s, -8 * s, 7 * s, 9 * s, sk);
  pr(ctx, 13 * s, -34 * s, 5 * s, 28 * s, metal);
  pr(ctx, 13 * s, -34 * s, 2 * s, 28 * s, '#e8eef8');
  pr(ctx, 11 * s, -8 * s, 9 * s, 4 * s, '#7d5f1f');
  if (hitFlash > 0) {
    ctx.globalAlpha = hitFlash;
    pr(ctx, -22 * s, -30 * s, 44 * s, 50 * s, '#ff5a5a');
    ctx.globalAlpha = 1;
  }
  ctx.restore();
}

// --- enemies: dispatch by sprite family --------------------------------
function drawCreature(ctx, sprite, x, y, s, color, time, hitFlash, dead) {
  ctx.save();
  ctx.translate(x, y);
  if (dead) ctx.globalAlpha = Math.max(0, dead);
  groundShadow(ctx, 0, 2, 28 * s);
  const fn = CREATURE[sprite] || CREATURE.humanoid;
  fn(ctx, s, color, time);
  if (hitFlash > 0 && !dead) {
    ctx.globalAlpha = hitFlash;
    pr(ctx, -26 * s, -38 * s, 52 * s, 56 * s, '#fff');
    ctx.globalAlpha = 1;
  }
  ctx.restore();
}

const CREATURE = {
  slime(ctx, s, c, t) {
    const sq = 1 + Math.sin(t * 3) * 0.08;
    const w = 30 * s * sq, h = 24 * s / sq;
    pr(ctx, -w / 2, -h, w, h, c);
    pr(ctx, -w / 2, -h, w, h * 0.35, shade(c, 40));
    pr(ctx, -w / 2 + 2, -h - 3 * s, w - 4 * s, 4 * s, shade(c, 60));
    // eyes
    pr(ctx, -8 * s, -h * 0.6, 6 * s, 7 * s, '#fff');
    pr(ctx, 4 * s, -h * 0.6, 6 * s, 7 * s, '#fff');
    pr(ctx, -6 * s, -h * 0.5, 3 * s, 4 * s, '#1a1422');
    pr(ctx, 6 * s, -h * 0.5, 3 * s, 4 * s, '#1a1422');
    pr(ctx, -5 * s, -h * 0.2, 12 * s, 2 * s, shade(c, -60));
  },
  bat(ctx, s, c, t) {
    const flap = Math.sin(t * 8) * 10 * s;
    pr(ctx, -34 * s, -20 * s - flap, 22 * s, 14 * s, shade(c, -30));
    pr(ctx, 12 * s, -20 * s - flap, 22 * s, 14 * s, shade(c, -30));
    pr(ctx, -34 * s, -20 * s - flap, 8 * s, 6 * s, shade(c, 30));
    pr(ctx, 26 * s, -20 * s - flap, 8 * s, 6 * s, shade(c, 30));
    pr(ctx, -12 * s, -24 * s, 24 * s, 20 * s, c);
    pr(ctx, -12 * s, -24 * s, 24 * s, 6 * s, shade(c, 35));
    pr(ctx, -10 * s, -28 * s, 5 * s, 6 * s, shade(c, -20)); // ears
    pr(ctx, 5 * s, -28 * s, 5 * s, 6 * s, shade(c, -20));
    pr(ctx, -8 * s, -18 * s, 5 * s, 5 * s, '#ffd34d');
    pr(ctx, 3 * s, -18 * s, 5 * s, 5 * s, '#ffd34d');
    pr(ctx, -6 * s, -16 * s, 2 * s, 2 * s, '#1a1422');
    pr(ctx, 5 * s, -16 * s, 2 * s, 2 * s, '#1a1422');
    pr(ctx, -4 * s, -9 * s, 8 * s, 3 * s, '#fff'); // fangs
  },
  humanoid(ctx, s, c, t) {
    const bob = Math.sin(t * 2) * 1.4 * s;
    ctx.translate(0, bob);
    pr(ctx, -8 * s, 4 * s, 6 * s, 14 * s, shade(c, -40));
    pr(ctx, 2 * s, 4 * s, 6 * s, 14 * s, shade(c, -40));
    pr(ctx, -11 * s, -14 * s, 22 * s, 20 * s, c); // robe/body
    pr(ctx, -11 * s, -14 * s, 22 * s, 5 * s, shade(c, 40));
    pr(ctx, -2 * s, -10 * s, 4 * s, 16 * s, shade(c, -30));
    pr(ctx, -16 * s, -12 * s, 6 * s, 16 * s, c); // arms
    pr(ctx, 10 * s, -12 * s, 6 * s, 16 * s, c);
    pr(ctx, -7 * s, -28 * s, 14 * s, 15 * s, '#d8b48a'); // head
    pr(ctx, -8 * s, -30 * s, 16 * s, 7 * s, shade(c, -10)); // hood
    pr(ctx, -5 * s, -22 * s, 3 * s, 4 * s, '#1a1422');
    pr(ctx, 3 * s, -22 * s, 3 * s, 4 * s, '#1a1422');
    pr(ctx, -3 * s, -15 * s, 7 * s, 2 * s, shade(c, -50));
  },
  beast(ctx, s, c, t) {
    const bob = Math.sin(t * 3.4) * 1.6 * s;
    ctx.translate(0, bob);
    pr(ctx, -14 * s, 2 * s, 6 * s, 14 * s, shade(c, -40));
    pr(ctx, 8 * s, 2 * s, 6 * s, 14 * s, shade(c, -40));
    pr(ctx, -18 * s, -16 * s, 36 * s, 22 * s, c); // body
    pr(ctx, -18 * s, -16 * s, 36 * s, 6 * s, shade(c, 38));
    pr(ctx, -24 * s, -22 * s, 16 * s, 16 * s, c); // head
    pr(ctx, -24 * s, -22 * s, 16 * s, 5 * s, shade(c, 38));
    pr(ctx, -24 * s, -28 * s, 5 * s, 7 * s, shade(c, -20)); // ear
    pr(ctx, -22 * s, -16 * s, 4 * s, 4 * s, '#ffd34d'); // eye
    pr(ctx, -28 * s, -10 * s, 6 * s, 5 * s, shade(c, -30)); // snout
    pr(ctx, -27 * s, -6 * s, 3 * s, 4 * s, '#fff'); // fang
    // spikes
    for (let i = 0; i < 4; i++) pr(ctx, -14 * s + i * 8 * s, -22 * s, 4 * s, 7 * s, shade(c, -50));
    pr(ctx, 16 * s, -10 * s, 8 * s, 4 * s, shade(c, -40)); // tail
  },
  knight(ctx, s, c, t) {
    const bob = Math.sin(t * 1.8) * 1.2 * s;
    ctx.translate(0, bob);
    pr(ctx, -8 * s, 6 * s, 7 * s, 12 * s, shade(c, -45));
    pr(ctx, 2 * s, 6 * s, 7 * s, 12 * s, shade(c, -45));
    pr(ctx, -12 * s, -12 * s, 24 * s, 20 * s, c);
    pr(ctx, -12 * s, -12 * s, 24 * s, 5 * s, shade(c, 45));
    pr(ctx, -8 * s, -26 * s, 16 * s, 15 * s, shade(c, 20)); // helm
    pr(ctx, -8 * s, -16 * s, 16 * s, 3 * s, shade(c, -40)); // visor slit
    pr(ctx, -1 * s, -28 * s, 3 * s, 6 * s, '#ff5a5a'); // plume
    // shield + blade
    pr(ctx, -22 * s, -10 * s, 10 * s, 18 * s, shade(c, -20));
    pr(ctx, -22 * s, -10 * s, 10 * s, 4 * s, shade(c, 30));
    pr(ctx, 13 * s, -30 * s, 5 * s, 24 * s, '#d2d8e4');
    pr(ctx, 11 * s, -8 * s, 9 * s, 4 * s, '#5a4626');
  },
  golem(ctx, s, c, t) {
    const bob = Math.sin(t * 1.4) * 1.4 * s;
    ctx.translate(0, bob);
    pr(ctx, -16 * s, 8 * s, 12 * s, 16 * s, shade(c, -45));
    pr(ctx, 4 * s, 8 * s, 12 * s, 16 * s, shade(c, -45));
    pr(ctx, -22 * s, -20 * s, 44 * s, 32 * s, c); // big body
    pr(ctx, -22 * s, -20 * s, 44 * s, 8 * s, shade(c, 40));
    pr(ctx, -28 * s, -14 * s, 9 * s, 24 * s, shade(c, -10)); // arms
    pr(ctx, 19 * s, -14 * s, 9 * s, 24 * s, shade(c, -10));
    pr(ctx, -10 * s, -34 * s, 20 * s, 16 * s, c); // head
    pr(ctx, -10 * s, -34 * s, 20 * s, 5 * s, shade(c, 40));
    pr(ctx, -6 * s, -28 * s, 5 * s, 5 * s, '#ff7a3a');
    pr(ctx, 2 * s, -28 * s, 5 * s, 5 * s, '#ff7a3a');
    // cracks
    pr(ctx, -4 * s, -16 * s, 3 * s, 18 * s, shade(c, -55));
    pr(ctx, 8 * s, -10 * s, 3 * s, 14 * s, shade(c, -55));
  },
  boss(ctx, s, c, t) {
    const bob = Math.sin(t * 1.2) * 2 * s;
    ctx.translate(0, bob);
    pr(ctx, -18 * s, 10 * s, 13 * s, 18 * s, shade(c, -45));
    pr(ctx, 5 * s, 10 * s, 13 * s, 18 * s, shade(c, -45));
    pr(ctx, -26 * s, -26 * s, 52 * s, 40 * s, c); // body
    pr(ctx, -26 * s, -26 * s, 52 * s, 9 * s, shade(c, 38));
    pr(ctx, -8 * s, -22 * s, 16 * s, 32 * s, shade(c, -25)); // chest plate
    pr(ctx, -34 * s, -18 * s, 11 * s, 28 * s, shade(c, -10)); // arms
    pr(ctx, 23 * s, -18 * s, 11 * s, 28 * s, shade(c, -10));
    pr(ctx, -14 * s, -46 * s, 28 * s, 22 * s, c); // head
    pr(ctx, -14 * s, -46 * s, 28 * s, 7 * s, shade(c, 38));
    // horns
    pr(ctx, -20 * s, -52 * s, 7 * s, 12 * s, '#e8e0d0');
    pr(ctx, 13 * s, -52 * s, 7 * s, 12 * s, '#e8e0d0');
    pr(ctx, -9 * s, -38 * s, 7 * s, 6 * s, '#ffd34d'); // eyes
    pr(ctx, 2 * s, -38 * s, 7 * s, 6 * s, '#ffd34d');
    pr(ctx, -7 * s, -36 * s, 3 * s, 3 * s, '#1a1422');
    pr(ctx, 4 * s, -36 * s, 3 * s, 3 * s, '#1a1422');
    pr(ctx, -8 * s, -28 * s, 16 * s, 3 * s, '#1a1422'); // mouth
    for (let i = 0; i < 5; i++) pr(ctx, -8 * s + i * 3.4 * s, -28 * s, 2 * s, 4 * s, '#fff');
  },
};

// --- intent / status / fx ----------------------------------------------
function drawIntentBadge(ctx, x, y, kind, value, time) {
  const pulse = 1 + Math.sin(time * 4) * 0.06;
  const w = 38 * pulse, h = 30 * pulse;
  ctx.fillStyle = 'rgba(20,16,30,0.92)';
  ctx.fillRect(x - w / 2, y - h / 2, w, h);
  const col = { attack: '#ff6a5a', block: '#7fb8ff', buff: '#ffd34d', debuff: '#c69bff', unknown: '#9a93b4' }[kind] || '#9a93b4';
  ctx.strokeStyle = col;
  ctx.lineWidth = 2;
  ctx.strokeRect(x - w / 2, y - h / 2, w, h);
  ctx.fillStyle = col;
  ctx.font = 'bold 15px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const icon = { attack: '⚔', block: '◈', buff: '▲', debuff: '✖', unknown: '?' }[kind] || '?';
  if (value != null && kind === 'attack') {
    ctx.fillText(icon, x - 8, y);
    ctx.fillStyle = '#fff';
    ctx.fillText('' + value, x + 9, y);
  } else {
    ctx.fillText(icon, x, y);
  }
  ctx.textBaseline = 'alphabetic';
}

function drawHpBar(ctx, x, y, w, ratio, block) {
  ctx.fillStyle = 'rgba(0,0,0,0.65)';
  ctx.fillRect(x - w / 2 - 2, y - 2, w + 4, 12);
  ctx.fillStyle = '#3a1320';
  ctx.fillRect(x - w / 2, y, w, 8);
  ctx.fillStyle = ratio > 0.5 ? '#5fe07a' : ratio > 0.25 ? '#f4c85a' : '#ff5a5a';
  ctx.fillRect(x - w / 2, y, w * Math.max(0, ratio), 8);
  if (block > 0) {
    ctx.fillStyle = '#7fb8ff';
    ctx.fillRect(x - w / 2 - 14, y - 1, 12, 12);
    ctx.fillStyle = '#11202a';
    ctx.font = 'bold 9px monospace';
    ctx.textAlign = 'center';
    ctx.fillText('' + block, x - w / 2 - 8, y + 8);
  }
}
