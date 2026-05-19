// Pixel Auto Arena - pixel art for the 9 unit archetypes and the arena.

function aaShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// Draws a unit centred at (cx, cy). `s` is the sprite size; sprites are
// authored on a 16-wide grid with feet near the bottom. facing 1 = right.
function drawUnit(ctx, cx, cy, s, unit, color, facing, flash, t) {
  const u = s / 16;
  ctx.save();
  ctx.translate(Math.round(cx), Math.round(cy));
  ctx.scale(facing, 1);
  const dark = aaShade(color, -62), lite = aaShade(color, 46);
  const ink = '#14101c', eye = '#fff2b0';
  // P draws on a grid where x in [-8,8], y in [-14,2]
  const P = (gx, gy, gw, gh, c) => {
    ctx.fillStyle = flash > 0 ? '#ffffff' : c;
    ctx.fillRect(Math.round(gx * u), Math.round(gy * u), Math.ceil(gw * u), Math.ceil(gh * u));
  };
  // ground shadow
  ctx.fillStyle = 'rgba(0,0,0,0.32)';
  ctx.fillRect(-7 * u, 0, 14 * u, 2.4 * u);
  const bob = Math.round(Math.sin(t * 3 + cx * 0.1));
  const g = unit.glyph;

  if (g === 'wolf') {
    P(-6, -3, 3, 3, ink); P(0, -3, 3, 3, ink); P(3, -3, 3, 3, ink);   // legs
    P(-6, -9, 11, 6, color); P(-6, -9, 11, 2, lite);
    P(4, -12, 5, 5, color); P(7, -15, 2, 3, color);                   // head + ear
    P(8, -10, 3, 2, dark); P(6, -11, 2, 2, eye);                      // snout, eye
    P(-8, -8, 3, 2, color);                                           // tail
  } else if (g === 'boar') {
    P(-6, -3, 3, 3, ink); P(-1, -3, 3, 3, ink); P(3, -3, 3, 3, ink);
    P(-7, -10, 12, 7, color); P(-7, -10, 12, 2, lite);
    P(5, -11, 5, 6, color); P(9, -9, 2, 2, dark);                     // head
    P(8, -6, 3, 1, '#f4f0e0'); P(10, -7, 1, 1, '#f4f0e0');            // tusk
    P(7, -10, 2, 2, eye);
  } else if (g === 'bear') {
    P(-6, -3, 4, 3, ink); P(2, -3, 4, 3, ink);
    P(-7, -11, 13, 8, color); P(-7, -11, 13, 3, lite); P(-7, -5, 13, 2, dark);
    P(4, -14, 6, 6, color); P(3, -15, 2, 2, color); P(9, -15, 2, 2, color);  // head + ears
    P(8, -12, 2, 2, eye); P(8, -9, 3, 2, dark);
  } else if (g === 'bolt') {
    P(-5, -11 + bob, 10, 8, color); P(-5, -11 + bob, 10, 3, lite);
    P(-5, -4 + bob, 10, 2, dark);
    P(-3, -9 + bob, 6, 3, ink); P(-2, -8 + bob, 2, 2, '#5fd9ff');     // eye visor
    P(-7, -8 + bob, 2, 4, dark); P(5, -8 + bob, 2, 4, dark);          // arms
    P(-2, -13 + bob, 1, 2, color); P(2, -1 + bob, 2, 2, '#5fd9ff');
  } else if (g === 'turret') {
    P(-7, -4, 14, 4, dark); P(-7, -4, 14, 1, lite);                   // base
    P(-4, -10, 8, 6, color); P(-4, -10, 8, 2, lite);
    P(-2, -9, 4, 3, ink); P(-1, -8, 2, 2, '#ff6b6b');                 // core
    P(3, -9, 8, 3, '#9aa6b8'); P(10, -10, 2, 5, '#9aa6b8');           // barrel
  } else if (g === 'titan') {
    P(-7, -4, 5, 4, ink); P(2, -4, 5, 4, ink);                        // feet
    P(-7, -12, 14, 8, color); P(-7, -12, 14, 3, lite); P(-7, -6, 14, 2, dark);
    P(-3, -16, 8, 5, color); P(-1, -15, 4, 2, '#ff6b6b');             // head + visor
    P(-10, -12, 3, 7, color); P(7, -12, 3, 7, color);                 // shoulders
    P(8, -11, 4, 9, '#9aa6b8');                                       // cannon arm
  } else if (g === 'imp') {
    P(-9, -10 + bob, 4, 6, dark); P(5, -10 + bob, 4, 6, dark);        // wings
    P(-4, -10 + bob, 8, 7, color); P(-4, -10 + bob, 8, 2, lite);
    P(-2, -8 + bob, 2, 2, eye); P(1, -8 + bob, 2, 2, eye);
    P(-3, -13 + bob, 2, 3, color); P(2, -13 + bob, 2, 3, color);      // horns
    P(-2, -3 + bob, 4, 3, ink);                                       // tail/leg
  } else if (g === 'sage') {
    P(-6, -7, 12, 7, color); P(-6, -7, 12, 2, lite); P(-6, -2, 12, 2, dark);
    P(-3, -13, 6, 6, '#e8c79a'); P(-3, -13, 6, 2, dark);              // hood/head
    P(-1, -11, 3, 2, ink);                                            // eyes shadow
    P(6, -16, 2, 14, '#7a5230'); P(5, -18, 4, 3, color);              // staff + orb
  } else if (g === 'archon') {
    ctx.globalAlpha = 0.35;
    P(-9, -16, 18, 18, lite);                                         // aura
    ctx.globalAlpha = 1;
    P(-6, -9, 12, 9, color); P(-6, -9, 12, 3, lite); P(-6, -2, 12, 2, dark);
    P(-3, -16, 6, 7, color); P(-2, -14, 4, 2, eye);                   // head
    P(-1, -20, 2, 4, lite);                                           // crest
    P(-8, -10, 3, 7, color); P(5, -10, 3, 7, color);                  // sleeves
  }
  ctx.restore();
}

// Simple stage backdrop for the arena.
function drawArena(ctx, w, h, t) {
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, '#1a1430');
  g.addColorStop(1, '#0c0a18');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  // floor band
  ctx.fillStyle = '#241c3c';
  ctx.fillRect(0, h * 0.52, w, h * 0.48);
  ctx.fillStyle = '#2f2550';
  for (let x = 0; x < w; x += 32) ctx.fillRect(x, h * 0.52, 16, 4);
  // centre divider
  ctx.fillStyle = 'rgba(255,255,255,0.07)';
  ctx.fillRect(w / 2 - 1, h * 0.2, 2, h * 0.6);
  // ambient sparks
  ctx.fillStyle = 'rgba(180,140,255,0.5)';
  for (let i = 0; i < 14; i++) {
    const sx = (i * 73 + t * 18) % w;
    const sy = (i * 41) % (h * 0.5) + 12;
    ctx.fillRect(sx | 0, sy | 0, 2, 2);
  }
}
