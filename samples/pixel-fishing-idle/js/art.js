// Pixel Fishing Idle - pixel art: fish species sprites, boat, lure shadow.
// Fish are authored on a small grid and scaled, so each species reads as a
// distinct silhouette rather than a generic rectangle.

function fShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// Draw a fish centred on (cx, cy). `s` is the pixel scale; `t` animates fins.
function drawFishSprite(ctx, cx, cy, fish, s, t, faceRight) {
  const body = fish.color;
  const dark = fShade(body, -54);
  const light = fShade(body, 46);
  const fin = RARITY[fish.rarity].color;
  const wob = Math.sin((t || 0) * 6) * s;

  ctx.save();
  ctx.translate(cx, cy);
  if (!faceRight) ctx.scale(-1, 1);
  const R = (x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(Math.round(x * s), Math.round(y * s), Math.ceil(w * s), Math.ceil(h * s)); };

  const shape = fish.shape;
  if (shape === 'small') {
    R(-9, -2, 16, 7, body); R(-9, -2, 16, 2, light); R(-9, 3, 16, 2, dark);
    R(7, -3, 5, 9, body); R(-11, -3 + wob / s, 4, 6, fin); // tail
    R(3, -1, 3, 3, '#0a141c'); R(3, -1, 1, 1, '#fff');
  } else if (shape === 'round') {
    R(-9, -7, 17, 14, body); R(-9, -7, 17, 3, light); R(-9, 4, 17, 3, dark);
    R(-12, -4, 5, 8, body); R(8, -6 + wob / s, 6, 12, fin);  // tail
    R(-2, -11, 8, 4, fin); R(0, 9, 7, 3, fin);                // fins
    R(-6, -3, 3, 3, '#0a141c'); R(-6, -3, 1, 1, '#fff');
  } else if (shape === 'long') {
    R(-16, -3, 30, 8, body); R(-16, -3, 30, 2, light); R(-16, 3, 30, 2, dark);
    R(14, -4 + wob / s, 5, 10, fin);
    R(-4, -7, 9, 4, fin);
    R(-13, -1, 3, 3, '#0a141c');
  } else if (shape === 'flat') {
    R(-13, -4, 26, 9, body); R(-13, -4, 26, 2, light);
    R(-9, -8, 18, 4, body); R(-9, 5, 18, 4, body);
    R(11, -2 + wob / s, 7, 5, fin); R(-15, -7, 4, 14, fin);
    R(-9, -2, 3, 3, '#0a141c'); R(-9, -2, 1, 1, '#fff');
  } else if (shape === 'jelly') {
    R(-9, -8, 18, 9, body); R(-9, -8, 18, 3, light); R(-7, 1, 14, 2, fish.color);
    for (let i = 0; i < 5; i++) R(-7 + i * 4, 3, 2, 7 + Math.sin(t * 4 + i) * 2, fish.color);
    ctx.globalAlpha = 0.5; R(-9, -8, 18, 9, light); ctx.globalAlpha = 1;
  } else if (shape === 'crab') {
    R(-9, -3, 18, 9, body); R(-9, -3, 18, 3, light); R(-9, 4, 18, 2, dark);
    R(-13, -1 + wob / s, 5, 5, body); R(8, -1 - wob / s, 5, 5, body); // claws
    for (let i = 0; i < 3; i++) { R(-8 + i * 5, 6, 2, 4, dark); R(2 + i * 4, 6, 2, 4, dark); }
    R(-5, -1, 3, 3, '#0a141c'); R(3, -1, 3, 3, '#0a141c');
  } else if (shape === 'serpent') {
    for (let i = 0; i < 7; i++) {
      const sy = Math.sin(t * 3 + i * 0.7) * 4;
      R(-22 + i * 6, sy - 4, 8, 9, i % 2 ? body : light);
      R(-22 + i * 6, sy - 7, 5, 3, fin); // crest
    }
    R(16, Math.sin(t * 3 + 5) * 4 - 5, 8, 11, body);
    R(20, Math.sin(t * 3 + 5) * 4 - 2, 3, 3, '#0a141c'); R(20, Math.sin(t * 3 + 5) * 4 - 2, 1, 1, '#fff');
    R(-26, Math.sin(t * 3) * 4 - 5, 5, 11, fin); // tail
  }
  ctx.restore();
}

// Lure shadow that circles below the bobber while waiting for a bite.
function drawLureShadow(ctx, x, y, t, accent) {
  ctx.globalAlpha = 0.4;
  ctx.fillStyle = accent;
  const sw = Math.sin(t * 2) * 14;
  ctx.fillRect(x - 14 + sw, y, 26, 6);
  ctx.fillRect(x - 6 + sw, y - 4, 10, 5);
  ctx.globalAlpha = 1;
}

// The fishing boat with a deckhand per crew level.
function drawBoat(ctx, t, crew) {
  const bob = Math.sin(t * 1.6) * 4;
  ctx.fillStyle = '#5a3f2c';
  ctx.fillRect(326, 276 + bob, 196, 40);
  ctx.fillStyle = '#3c2a1d';
  ctx.fillRect(326, 308 + bob, 196, 8);
  ctx.fillStyle = '#7b5738';
  ctx.fillRect(356, 244 + bob, 122, 36);
  ctx.fillStyle = '#8f6747';
  ctx.fillRect(356, 244 + bob, 122, 5);
  ctx.fillStyle = '#263243';
  ctx.fillRect(394, 238 + bob, 48, 42);
  ctx.fillStyle = '#f0bf8f';
  ctx.fillRect(404, 212 + bob, 28, 26);
  ctx.fillStyle = '#d8554f';
  ctx.fillRect(404, 212 + bob, 28, 6);
  for (let i = 0; i < Math.min(4, crew); i++) {
    const cx = 350 + i * 30;
    ctx.fillStyle = '#65d9ff';
    ctx.fillRect(cx, 232 + bob, 14, 18);
    ctx.fillStyle = '#e7b88a';
    ctx.fillRect(cx + 2, 224 + bob, 10, 9);
  }
  ctx.strokeStyle = '#d7b46a';
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.moveTo(430, 230 + bob);
  ctx.lineTo(595, 286);
  ctx.stroke();
  return bob;
}
