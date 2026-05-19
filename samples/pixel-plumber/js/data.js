// Pixel Plumber - pipe pieces, the upcoming-piece bag, and the level campaign.

const VW = 360, VH = 480;

// Each pipe piece's open sides. Water entering one open side leaves by another.
const PIECES = {
  h:  ['e', 'w'],
  v:  ['n', 's'],
  ne: ['n', 'e'],
  nw: ['n', 'w'],
  se: ['s', 'e'],
  sw: ['s', 'w'],
  x:  ['n', 'e', 's', 'w'],
};
// Weighted draw bag for the upcoming-piece queue (crosses are rare).
const PIECE_BAG = ['h', 'h', 'h', 'v', 'v', 'v', 'ne', 'nw', 'se', 'sw',
                   'ne', 'nw', 'se', 'sw', 'x'];

// source.dir is the direction water leaves the source into the grid.
const LEVELS = [
  { cols: 7,  rows: 6, source: { r: 2, c: 0, dir: 'e' }, goal: { r: 3, c: 6 }, delay: 6.0, interval: 1.15, seed: 1207 },
  { cols: 8,  rows: 6, source: { r: 1, c: 0, dir: 'e' }, goal: { r: 4, c: 7 }, delay: 5.5, interval: 1.05, seed: 2391 },
  { cols: 8,  rows: 7, source: { r: 3, c: 0, dir: 'e' }, goal: { r: 0, c: 7 }, delay: 5.0, interval: 1.00, seed: 3518 },
  { cols: 9,  rows: 7, source: { r: 6, c: 4, dir: 'n' }, goal: { r: 0, c: 4 }, delay: 5.0, interval: 0.94, seed: 4602 },
  { cols: 9,  rows: 8, source: { r: 0, c: 1, dir: 's' }, goal: { r: 7, c: 7 }, delay: 4.6, interval: 0.88, seed: 5774 },
  { cols: 10, rows: 8, source: { r: 4, c: 0, dir: 'e' }, goal: { r: 4, c: 9 }, delay: 4.6, interval: 0.83, seed: 6855 },
  { cols: 10, rows: 9, source: { r: 8, c: 1, dir: 'n' }, goal: { r: 0, c: 8 }, delay: 4.2, interval: 0.78, seed: 7931 },
  { cols: 10, rows: 9, source: { r: 0, c: 5, dir: 's' }, goal: { r: 8, c: 2 }, delay: 4.2, interval: 0.72, seed: 9048 },
];
const LEVEL_COUNT = LEVELS.length;

const OPP = { n: 's', s: 'n', e: 'w', w: 'e' };
const STEP = { n: [-1, 0], s: [1, 0], e: [0, 1], w: [0, -1] };

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}
