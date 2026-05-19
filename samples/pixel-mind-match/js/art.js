// Pixel Mind Match - card back and creature face rendering.

function mmRect(ctx, px, py, x, y, w, h, u, c) {
  ctx.fillStyle = c;
  ctx.fillRect(Math.round(px + x * u), Math.round(py + y * u), Math.ceil(w * u), Math.ceil(h * u));
}

// Face-down tile.
function drawCardBack(ctx, px, py, s) {
  const u = s / 16;
  mmRect(ctx, px, py, 0, 0, 16, 16, u, '#3a3266');
  mmRect(ctx, px, py, 1, 1, 14, 14, u, '#2a2450');
  mmRect(ctx, px, py, 1, 1, 14, 2, u, '#4a4078');
  mmRect(ctx, px, py, 1, 13, 14, 2, u, '#1c1838');
  // emblem star
  mmRect(ctx, px, py, 7, 3, 2, 10, u, '#5a4f9a');
  mmRect(ctx, px, py, 3, 7, 10, 2, u, '#5a4f9a');
  mmRect(ctx, px, py, 5, 5, 6, 6, u, '#6a5ec0');
  mmRect(ctx, px, py, 7, 7, 2, 2, u, '#c9c0ff');
}

// Creature face on a lit tile.
function drawCreature(ctx, px, py, s, creature, matched, time) {
  const u = s / 16;
  const h = creature.hue;
  const body = `hsl(${h}, 62%, 56%)`;
  const dark = `hsl(${h}, 50%, 34%)`;
  const lite = `hsl(${h}, 72%, 72%)`;
  // tile face
  mmRect(ctx, px, py, 0, 0, 16, 16, u, matched ? '#23502f' : '#1d2740');
  mmRect(ctx, px, py, 1, 1, 14, 14, u, matched ? '#2d6b3e' : '#26314f');
  mmRect(ctx, px, py, 1, 1, 14, 2, u, matched ? '#3f8a52' : '#34406a');

  const R = (x, y, w, hh, c) => mmRect(ctx, px, py, x, y, w, hh, u, c);
  const sh = creature.shape;
  if (sh === 'slime') {
    R(4, 7, 8, 6, body); R(3, 9, 10, 4, body); R(4, 7, 8, 2, lite);
    R(3, 12, 10, 1, dark);
    R(6, 9, 2, 2, '#15121f'); R(9, 9, 2, 2, '#15121f');
  } else if (sh === 'bird') {
    R(6, 3, 5, 5, body); R(5, 7, 7, 5, body); R(5, 7, 7, 2, lite);
    R(3, 8, 3, 3, dark); R(11, 5, 3, 2, '#f4c85a');           // wing, beak
    R(8, 4, 2, 2, '#15121f');
    R(6, 12, 2, 2, dark); R(9, 12, 2, 2, dark);
  } else if (sh === 'bug') {
    R(5, 4, 6, 9, body); R(5, 4, 6, 3, lite); R(5, 8, 6, 1, dark);
    R(3, 5, 2, 2, dark); R(11, 5, 2, 2, dark);
    R(6, 6, 1, 1, '#15121f'); R(9, 6, 1, 1, '#15121f');
    R(2, 7, 3, 1, dark); R(11, 7, 3, 1, dark); R(2, 10, 3, 1, dark); R(11, 10, 3, 1, dark);
  } else if (sh === 'fish') {
    R(4, 6, 8, 5, body); R(4, 6, 8, 2, lite);
    R(11, 5, 4, 7, body); R(2, 7, 3, 3, dark);                // tail, fin
    R(5, 7, 2, 2, '#15121f');
    R(4, 11, 8, 1, dark);
  } else if (sh === 'cat') {
    R(5, 4, 6, 6, body); R(4, 2, 3, 3, body); R(9, 2, 3, 3, body);  // head + ears
    R(4, 10, 8, 3, body); R(5, 4, 6, 2, lite);
    R(6, 6, 1, 2, '#15121f'); R(9, 6, 1, 2, '#15121f');
    R(7, 8, 2, 1, dark); R(4, 12, 8, 1, dark);
  } else { // mush
    R(3, 4, 10, 5, body); R(4, 3, 8, 2, body); R(3, 4, 10, 2, lite);
    R(5, 6, 2, 2, '#fff'); R(9, 5, 2, 2, '#fff');             // spots
    R(6, 9, 4, 4, '#e8dcc0'); R(6, 12, 4, 1, dark);          // stem
  }
  if (matched) {
    ctx.globalAlpha = 0.35 + Math.abs(Math.sin(time * 4)) * 0.3;
    mmRect(ctx, px, py, 0, 0, 16, 16, u, '#7dff9f');
    ctx.globalAlpha = 1;
  }
}
