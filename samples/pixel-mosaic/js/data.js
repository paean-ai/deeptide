// Pixel Mosaic - a Fill-a-Pix puzzle. Every cell shows how many cells in its
// 3x3 neighbourhood (including itself) are filled. Shade the right cells.
//
// Each puzzle is a hand-drawn pixel bitmap. The clue grid is derived from it
// at load time; a win is when every cell's clue matches the player's shading.

const VW = 360, VH = 480;

const PUZZLES = [
  { name: ['Hourglass', '沙漏'], color: '#e8c984', grid: [
    '0111111110',
    '0011111100',
    '0001111000',
    '0000110000',
    '0000110000',
    '0000110000',
    '0000110000',
    '0001111000',
    '0011111100',
    '0111111110',
  ] },
  { name: ['Diamond', '宝石'], color: '#5fd6e8', grid: [
    '0000110000',
    '0001111000',
    '0011111100',
    '0111111110',
    '1111111111',
    '0111111110',
    '0011111100',
    '0001111000',
    '0000110000',
    '0000000000',
  ] },
  { name: ['Mug', '陶杯'], color: '#c98a4a', grid: [
    '0000000000',
    '0011111110',
    '0011111111',
    '0011000011',
    '0011111101',
    '0011111101',
    '0011000011',
    '0011111111',
    '0011111110',
    '0000000000',
  ] },
  { name: ['Lighthouse', '灯塔'], color: '#ffd86b', grid: [
    '0000100000',
    '0001110000',
    '0001110000',
    '0011111000',
    '0001110000',
    '0001110000',
    '0001110000',
    '0011111000',
    '0111111100',
    '1111111110',
  ] },
  { name: ['Fox', '狐狸'], color: '#ff8a3a', grid: [
    '1100000011',
    '1110000111',
    '1111111111',
    '0111111110',
    '0111111110',
    '0110110110',
    '0111111110',
    '0011111100',
    '0011001100',
    '0011001100',
  ] },
  { name: ['Mountain', '山峰'], color: '#9aa6be', grid: [
    '0000000000',
    '0000100000',
    '0000110000',
    '0001111000',
    '0011111100',
    '0011111100',
    '0111111110',
    '0111111110',
    '1111111111',
    '1111111111',
  ] },
];
const PUZZLE_COUNT = PUZZLES.length;
const N = 10;       // every puzzle is 10x10

// player cell states
const UNKNOWN = 0, FILLED = 1, EMPTY = 2;

// 3x3 count of filled cells around (r,c), clipped to grid
function clueAt(grid, r, c) {
  let n = 0;
  for (let dr = -1; dr <= 1; dr++) {
    for (let dc = -1; dc <= 1; dc++) {
      const rr = r + dr, cc = c + dc;
      if (rr < 0 || cc < 0 || rr >= N || cc >= N) continue;
      if (grid[rr][cc] === '1') n++;
    }
  }
  return n;
}

// Derive the clue grid (numbers 0..9) for a puzzle.
function deriveClues(p) {
  const c = [];
  for (let r = 0; r < N; r++) {
    const row = new Uint8Array(N);
    for (let cc = 0; cc < N; cc++) row[cc] = clueAt(p.grid, r, cc);
    c.push(row);
  }
  return c;
}

// Cycle a cell state: UNKNOWN -> FILLED -> EMPTY -> UNKNOWN.
function cycleState(v) { return (v + 1) % 3; }

// Evaluate the player's cell states against the puzzle clues.
// Returns { bad: Set of "r,c" cells whose clue is impossible / wrong, solved }
function evaluate(p, cells) {
  const clues = p._clues || (p._clues = deriveClues(p));
  const bad = new Set();
  let solved = true;
  for (let r = 0; r < N; r++) {
    for (let c = 0; c < N; c++) {
      let fl = 0, unk = 0, total = 0;
      for (let dr = -1; dr <= 1; dr++) {
        for (let dc = -1; dc <= 1; dc++) {
          const rr = r + dr, cc = c + dc;
          if (rr < 0 || cc < 0 || rr >= N || cc >= N) continue;
          total++;
          const v = cells[rr * N + cc];
          if (v === FILLED) fl++;
          else if (v === UNKNOWN) unk++;
        }
      }
      const target = clues[r][c];
      if (fl > target || fl + unk < target) bad.add(r + ',' + c);
      if (fl !== target) solved = false;
    }
  }
  return { clues, bad, solved };
}
