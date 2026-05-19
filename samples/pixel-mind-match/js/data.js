// Pixel Mind Match - content data: creatures and level grids.

// 18 creatures = 6 shapes x 3 hue variants, every pair visually distinct.
const SHAPES = ['slime', 'bird', 'bug', 'fish', 'cat', 'mush'];
const CREATURES = [];
for (let s = 0; s < SHAPES.length; s++) {
  for (let v = 0; v < 3; v++) {
    CREATURES.push({ id: CREATURES.length, shape: SHAPES[s], hue: (s * 60 + v * 130 + 20) % 360 });
  }
}

// Level grids (cols x rows, product even). starMoves: [3-star, 2-star] limits;
// anything more than the 2-star limit clears at 1 star.
const LEVELS = [
  { c: 3, r: 4, starMoves: [9, 14] },
  { c: 4, r: 4, starMoves: [13, 20] },
  { c: 4, r: 5, starMoves: [17, 26] },
  { c: 5, r: 4, starMoves: [17, 26] },
  { c: 4, r: 6, starMoves: [22, 34] },
  { c: 5, r: 6, starMoves: [30, 44] },
  { c: 6, r: 6, starMoves: [40, 58] },
];
const LEVEL_COUNT = LEVELS.length;

function pairsFor(level) { return (level.c * level.r) / 2; }
