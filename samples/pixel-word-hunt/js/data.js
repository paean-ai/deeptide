// Pixel Word Hunt - themed word-search puzzles + grid generation.

const VW = 360, VH = 480;

// Each puzzle: a bilingual theme label, grid size n, a seed, and the word
// list (uppercase A-Z, each word's length <= n).
const PUZZLES = [
  { theme: ['Animals', '动物'], n: 9, seed: 1471,
    words: ['CAT', 'LION', 'BEAR', 'WOLF', 'FROG', 'TIGER', 'PANDA'] },
  { theme: ['Fruit', '水果'], n: 9, seed: 2638,
    words: ['PEAR', 'PLUM', 'KIWI', 'MANGO', 'MELON', 'GRAPE', 'LEMON', 'APPLE'] },
  { theme: ['Space', '太空'], n: 10, seed: 3852,
    words: ['STAR', 'MARS', 'MOON', 'COMET', 'ORBIT', 'PLANET', 'ROCKET', 'GALAXY'] },
  { theme: ['Ocean', '海洋'], n: 10, seed: 4109,
    words: ['FISH', 'CRAB', 'REEF', 'WHALE', 'SHARK', 'CORAL', 'SQUID', 'PEARL'] },
  { theme: ['Colours', '颜色'], n: 10, seed: 5274,
    words: ['BLUE', 'GOLD', 'PINK', 'GREEN', 'AMBER', 'CORAL', 'IVORY', 'OLIVE'] },
  { theme: ['Weather', '天气'], n: 11, seed: 6630,
    words: ['RAIN', 'SNOW', 'WIND', 'STORM', 'CLOUD', 'FROST', 'SUNNY', 'THUNDER'] },
  { theme: ['Music', '音乐'], n: 11, seed: 7918,
    words: ['DRUM', 'BASS', 'TUNE', 'CHORD', 'PIANO', 'TEMPO', 'MELODY', 'RHYTHM'] },
  { theme: ['Castle', '城堡'], n: 11, seed: 9043,
    words: ['KING', 'MOAT', 'TOWER', 'KNIGHT', 'CASTLE', 'THRONE', 'DRAGON', 'BANNER'] },
];
const PUZZLE_COUNT = PUZZLES.length;

// The 8 placement directions.
const WH_DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [1, -1], [-1, 1], [-1, -1]];

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}
function whShuffle(arr, rng) {
  const a = arr.slice();
  for (let i = a.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

// Build a letter grid with every word placed (overlaps allowed) and the
// remaining cells filled with random letters. Deterministic per seed.
function genGrid(puzzle) {
  const AZ = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  for (let attempt = 0; attempt < 40; attempt++) {
    const rng = seededRandom(puzzle.seed + attempt * 104729);
    const n = puzzle.n;
    const grid = Array.from({ length: n }, () => new Array(n).fill(0));
    const placements = {};
    let ok = true;
    for (const word of whShuffle(puzzle.words, rng)) {
      const spots = [];
      for (let r = 0; r < n; r++) {
        for (let c = 0; c < n; c++) {
          for (const d of WH_DIRS) {
            const er = r + d[0] * (word.length - 1);
            const ec = c + d[1] * (word.length - 1);
            if (er < 0 || ec < 0 || er >= n || ec >= n) continue;
            let fits = true;
            for (let i = 0; i < word.length; i++) {
              const cell = grid[r + d[0] * i][c + d[1] * i];
              if (cell !== 0 && cell !== word[i]) { fits = false; break; }
            }
            if (fits) spots.push([r, c, d]);
          }
        }
      }
      if (!spots.length) { ok = false; break; }
      const [r, c, d] = spots[(rng() * spots.length) | 0];
      const cells = [];
      for (let i = 0; i < word.length; i++) {
        grid[r + d[0] * i][c + d[1] * i] = word[i];
        cells.push([r + d[0] * i, c + d[1] * i]);
      }
      placements[word] = cells;
    }
    if (!ok) continue;
    for (let r = 0; r < n; r++) {
      for (let c = 0; c < n; c++) {
        if (grid[r][c] === 0) grid[r][c] = AZ[(rng() * 26) | 0];
      }
    }
    return { grid, placements };
  }
  return null;
}
