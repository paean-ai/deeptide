// Pixel Color Cascade - flood-fill puzzle data.
//
// Each level is a seed + grid size (n) + colour count (k). The board is
// generated deterministically, and the move limit is derived at load time
// from a greedy solver so every level is guaranteed winnable.

const VW = 360, VH = 480;

// Six pixel-friendly hues; a level with k colours uses the first k.
const PALETTE = ['#e8554f', '#ef9b3e', '#f2cf3f', '#5fc06e', '#4a9be8', '#9a6cd8'];

const LEVELS = [
  { n: 7,  k: 4, seed: 1207 },
  { n: 8,  k: 4, seed: 2418 },
  { n: 9,  k: 4, seed: 3631 },
  { n: 9,  k: 5, seed: 4842 },
  { n: 10, k: 5, seed: 5093 },
  { n: 11, k: 5, seed: 6274 },
  { n: 11, k: 6, seed: 7385 },
  { n: 12, k: 5, seed: 8596 },
  { n: 12, k: 6, seed: 9607 },
  { n: 13, k: 6, seed: 10718 },
  { n: 13, k: 6, seed: 11829 },
  { n: 14, k: 6, seed: 12940 },
  { n: 14, k: 6, seed: 14051 },
  { n: 15, k: 6, seed: 15162 },
  { n: 15, k: 6, seed: 16273 },
  { n: 16, k: 6, seed: 17384 },
  { n: 16, k: 6, seed: 18506 },
  { n: 16, k: 6, seed: 19618 },
  { n: 17, k: 6, seed: 20729 },
  { n: 17, k: 6, seed: 21840 },
  { n: 17, k: 6, seed: 22951 },
  { n: 18, k: 6, seed: 24063 },
  { n: 18, k: 6, seed: 25174 },
  { n: 18, k: 6, seed: 26285 },
];
const LEVEL_COUNT = LEVELS.length;

// Extra moves granted above the greedy-solver count — generous early, tight late.
function levelSlack(index) {
  return Math.max(2, 7 - Math.floor(index / 2));
}

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// Build the n*n colour-index grid for a level.
function buildBoard(level) {
  const rng = seededRandom(level.seed);
  const g = [];
  for (let y = 0; y < level.n; y++) {
    const row = [];
    for (let x = 0; x < level.n; x++) row.push(Math.floor(rng() * level.k));
    g.push(row);
  }
  return g;
}
