// Pixel Match Quest - level definitions

const GRID = 8;
const COLORS = 6;

// layout chars: '.' normal  'i' ice(1)  'I' ice(2)  'x' crate  'C' ingredient
// objective.type: score | color | ice | crate | drop
// color index for `color` objectives: 0..5
const LEVELS = [
  { // 1 - intro
    moves: 22, objective: { type: 'score', n: 1800 },
    stars: [1800, 3200, 5000],
  },
  { // 2 - color
    moves: 24, objective: { type: 'color', color: 0, n: 28 },
    stars: [2000, 3600, 5500],
  },
  { // 3 - ice intro
    moves: 20, objective: { type: 'ice' },
    stars: [2200, 3800, 5800],
    layout: [
      '........',
      '........',
      '..iiii..',
      '..iiii..',
      '..iiii..',
      '..iiii..',
      '........',
      '........',
    ],
  },
  { // 4 - score push
    moves: 22, objective: { type: 'score', n: 5200 },
    stars: [5200, 8000, 12000],
  },
  { // 5 - crates
    moves: 24, objective: { type: 'crate' },
    stars: [2600, 4400, 6800],
    layout: [
      '........',
      '...xx...',
      '..x..x..',
      '.x....x.',
      '.x....x.',
      '..x..x..',
      '...xx...',
      '........',
    ],
  },
  { // 6 - drop
    moves: 26, objective: { type: 'drop', n: 4 },
    stars: [3000, 5000, 7800],
    layout: [
      'C..C..C.',
      '........',
      '........',
      '........',
      '........',
      '........',
      '........',
      '...C....',
    ],
  },
  { // 7 - color, harder
    moves: 24, objective: { type: 'color', color: 2, n: 36 },
    stars: [3600, 6000, 9000],
  },
  { // 8 - thick ice
    moves: 22, objective: { type: 'ice' },
    stars: [3400, 5600, 8600],
    layout: [
      'I......I',
      '.I.ii.I.',
      '..IiiI..',
      '...II...',
      '...II...',
      '..IiiI..',
      '.I.ii.I.',
      'I......I',
    ],
  },
  { // 9 - big score
    moves: 20, objective: { type: 'score', n: 9000 },
    stars: [9000, 14000, 20000],
  },
  { // 10 - crate + ice
    moves: 26, objective: { type: 'crate' },
    stars: [4000, 6800, 10000],
    layout: [
      '..x..x..',
      '.iI..Ii.',
      'x.x..x.x',
      '........',
      '........',
      'x.x..x.x',
      '.iI..Ii.',
      '..x..x..',
    ],
  },
  { // 11 - big drop
    moves: 28, objective: { type: 'drop', n: 6 },
    stars: [4400, 7200, 11000],
    layout: [
      'C.C...C.',
      '........',
      '..C..C..',
      '........',
      '........',
      '...C....',
      '........',
      '........',
    ],
  },
  { // 12 - finale
    moves: 26, objective: { type: 'score', n: 12000 },
    stars: [12000, 18000, 26000],
    layout: [
      'I..xx..I',
      '.i.II.i.',
      '..i..i..',
      'x.i..i.x',
      'x.i..i.x',
      '..i..i..',
      '.i.II.i.',
      'I..xx..I',
    ],
  },
];

// boosters
const BOOSTERS = {
  hammer:  { icon: '🔨', cost: 120 },
  shuffle: { icon: '🔀', cost: 90 },
  moves:   { icon: '➕', cost: 160 },
};
