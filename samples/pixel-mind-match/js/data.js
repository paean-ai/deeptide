// Pixel Mind Match - content data: creatures and level grids.

// 24 creatures = 6 shapes x 4 hue variants, every pair visually distinct.
// (Was 18 = 6 x 3 — bumped to 4 hues to support the new 6 x 7 / 6 x 8
// endgame boards without repeating sprites within a board.)
const SHAPES = ['slime', 'bird', 'bug', 'fish', 'cat', 'mush'];
const CREATURES = [];
for (let s = 0; s < SHAPES.length; s++) {
  for (let v = 0; v < 4; v++) {
    CREATURES.push({ id: CREATURES.length, shape: SHAPES[s], hue: (s * 60 + v * 95 + 20) % 360 });
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
  { c: 6, r: 7, starMoves: [48, 70] },
  { c: 6, r: 8, starMoves: [56, 82] },
];
const LEVEL_COUNT = LEVELS.length;

function pairsFor(level) { return (level.c * level.r) / 2; }
