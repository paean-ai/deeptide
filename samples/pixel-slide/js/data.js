// Pixel Slide - sliding-tile puzzle: levels, scramble, slide rules.

const VW = 360, VH = 480;

// Each level: grid size n (n*n-1 tiles + one gap), a scramble seed, and a
// par move count for the 3-star rating.
const LEVELS = [
  { n: 3, seed: 1502, par: 26 },
  { n: 3, seed: 2841, par: 30 },
  { n: 4, seed: 3677, par: 80 },
  { n: 4, seed: 4920, par: 92 },
  { n: 4, seed: 6133, par: 104 },
  { n: 5, seed: 7488, par: 170 },
  { n: 5, seed: 8615, par: 190 },
  { n: 5, seed: 9902, par: 210 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// orthogonal neighbours of a flat index on an n*n grid
function neighbors(idx, n) {
  const r = (idx / n) | 0, c = idx % n, out = [];
  if (r > 0) out.push(idx - n);
  if (r < n - 1) out.push(idx + n);
  if (c > 0) out.push(idx - 1);
  if (c < n - 1) out.push(idx + 1);
  return out;
}

function solvedBoard(n) {
  const b = [];
  for (let i = 1; i < n * n; i++) b.push(i);
  b.push(0);
  return b;
}
function isSolved(board) {
  for (let i = 0; i < board.length - 1; i++) if (board[i] !== i + 1) return false;
  return board[board.length - 1] === 0;
}

// Scramble a solved board with random legal slides (so it is always solvable).
function scramble(n, seed) {
  const board = solvedBoard(n);
  const rng = seededRandom(seed);
  let empty = n * n - 1, last = -1;
  const slides = n * n * 26;
  for (let k = 0; k < slides; k++) {
    const opts = neighbors(empty, n).filter(i => i !== last);
    const pick = opts[(rng() * opts.length) | 0];
    board[empty] = board[pick];
    board[pick] = 0;
    last = empty;
    empty = pick;
  }
  return board;
}
