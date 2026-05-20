// Pixel Hanoi - classic Tower of Hanoi. Three pegs; a stack of disks
// starts on the left peg, decreasing in size from bottom to top. Move the
// whole stack to the right peg under the rules:
//   - only one disk moves at a time (the top disk of any peg),
//   - never place a larger disk on top of a smaller one.
//
// Solve in the optimal 2^N - 1 moves to earn three stars.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Spire',     '塔尖'], disks: 3 },     // par = 7
  { name: ['Tower',     '塔身'], disks: 4 },     // par = 15
  { name: ['Citadel',   '城堡'], disks: 5 },     // par = 31
  { name: ['Pagoda',    '宝塔'], disks: 6 },     // par = 63
  { name: ['Ziggurat',  '塔庙'], disks: 7 },     // par = 127
  { name: ['Colossus',  '巨擎'], disks: 8 },     // par = 255
];
const LEVEL_COUNT = LEVELS.length;

function par(disks) { return (1 << disks) - 1; }

// ---- runtime state -----------------------------------------------------
// pegs: [array, array, array]. Each peg is a bottom-up stack of disk
// sizes (e.g. [4, 3, 2, 1] for a four-disk stack with disk 1 on top).
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const initial = [];
  for (let i = lv.disks; i >= 1; i--) initial.push(i);   // largest on bottom
  return {
    levelIndex, lv,
    disks: lv.disks,
    pegs: [initial.slice(), [], []],
    selected: -1,         // peg index whose top disk is "picked up"
    moves: 0,
    history: [],
    solved: false,
    over: false,
  };
}

// Return the size of the top disk on a peg, or 0 if empty.
function topOf(peg) { return peg.length ? peg[peg.length - 1] : 0; }

// Attempt to move the top disk of `from` to `to`. Returns true on success.
function moveDisk(s, from, to) {
  if (s.solved || from === to) return false;
  if (from < 0 || from > 2 || to < 0 || to > 2) return false;
  const src = s.pegs[from];
  const dst = s.pegs[to];
  if (src.length === 0) return false;
  const disk = src[src.length - 1];
  const dstTop = topOf(dst);
  if (dstTop && dstTop < disk) return false;     // larger onto smaller is illegal
  src.pop();
  dst.push(disk);
  s.history.push({ from, to });
  s.moves++;
  checkSolved(s);
  return true;
}

function checkSolved(s) {
  if (s.pegs[2].length === s.disks) {
    // Right peg holds every disk: solved.
    s.solved = true;
    s.over = true;
  }
}

// Tap-then-tap input: first tap selects a non-empty peg, second tap moves
// the picked disk to the target peg (or cancels by re-tapping the same).
// Returns true if the tap caused a state change (selection or move).
function tapPeg(s, p) {
  if (s.over) return false;
  if (p < 0 || p > 2) return false;
  if (s.selected === -1) {
    if (s.pegs[p].length === 0) return false;
    s.selected = p;
    return true;
  }
  if (s.selected === p) { s.selected = -1; return true; }
  const moved = moveDisk(s, s.selected, p);
  s.selected = -1;
  return moved;
}

function undo(s) {
  const last = s.history.pop();
  if (!last) return false;
  // Reverse the recorded move; bypass the legality check by hand to allow
  // restoring even into "wrong" intermediates.
  const disk = s.pegs[last.to].pop();
  s.pegs[last.from].push(disk);
  s.moves = Math.max(0, s.moves - 1);
  s.selected = -1;
  s.solved = false;
  s.over = false;
  return true;
}

function restart(s) {
  const lv = LEVELS[s.levelIndex];
  s.pegs = [[], [], []];
  for (let i = lv.disks; i >= 1; i--) s.pegs[0].push(i);
  s.selected = -1;
  s.moves = 0;
  s.history.length = 0;
  s.solved = false;
  s.over = false;
}

// 3 stars at optimal par; 2 within +50%; 1 otherwise.
function stars(moves, disks) {
  const p = par(disks);
  if (moves <= p) return 3;
  if (moves <= Math.floor(p * 1.5)) return 2;
  return 1;
}

// Compute the optimal solution as a flat array of {from, to} moves. Useful
// for tests; also for an optional "hint next move" in the UI later.
function optimalMoves(n, from = 0, to = 2, via = 1, out = []) {
  if (n === 0) return out;
  optimalMoves(n - 1, from, via, to, out);
  out.push({ from, to });
  optimalMoves(n - 1, via, to, from, out);
  return out;
}
