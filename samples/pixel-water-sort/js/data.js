// Pixel Water Sort - colours, level campaign, puzzle generation + solver.

const VW = 360, VH = 480;
const TUBE_CAP = 4;

const COLORS = [
  '#e8554f', '#4a9be8', '#5fc06e', '#f2cf3f', '#9a6cd8',
  '#ef9b3e', '#4fd6d6', '#ff7db0', '#a8d84a', '#c8804a',
];

// Each level: a colour count. Tubes = colours + 2 empty. Seeded for repeats.
const LEVELS = [
  { colors: 3, seed: 1411 },
  { colors: 3, seed: 2933 },
  { colors: 4, seed: 3517 },
  { colors: 4, seed: 4806 },
  { colors: 5, seed: 5290 },
  { colors: 5, seed: 6744 },
  { colors: 6, seed: 7188 },
  { colors: 6, seed: 8620 },
  { colors: 7, seed: 9035 },
  { colors: 7, seed: 10471 },
  { colors: 8, seed: 11859 },
  { colors: 8, seed: 13002 },
  { colors: 9, seed: 14488 },
  { colors: 9, seed: 15971 },
  { colors: 10, seed: 17350 },
  { colors: 10, seed: 18866 },
  { colors: 10, seed: 20415 },
  { colors: 10, seed: 22043 },
  { colors: 10, seed: 23718 },
  { colors: 10, seed: 25391 },
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

// ---- pour rules ----------------------------------------------------------
function topRun(tube) {
  if (!tube.length) return null;
  const c = tube[tube.length - 1];
  let n = 0;
  for (let i = tube.length - 1; i >= 0 && tube[i] === c; i--) n++;
  return { color: c, count: n };
}
function tubeSorted(tube) {
  return tube.length === 0 ||
    (tube.length === TUBE_CAP && tube.every(c => c === tube[0]));
}
function canPour(src, dst) {
  if (!src.length || dst.length >= TUBE_CAP) return false;
  if (src.length === TUBE_CAP && src.every(c => c === src[0])) return false;
  if (!dst.length) return true;
  return dst[dst.length - 1] === src[src.length - 1];
}
function doPour(tubes, si, di) {
  const t = tubes.map(x => x.slice());
  const run = topRun(t[si]);
  const n = Math.min(run.count, TUBE_CAP - t[di].length);
  for (let k = 0; k < n; k++) t[di].push(t[si].pop());
  return t;
}
function isSolved(tubes) {
  return tubes.every(tubeSorted);
}

// ---- solver (used to confirm a generated puzzle is winnable) -------------
function tubesKey(tubes) {
  return tubes.map(t => t.join(',')).sort().join('|');
}
function solvable(tubes) {
  const visited = new Set([tubesKey(tubes)]);
  const stack = [tubes];
  let nodes = 0;
  while (stack.length) {
    if (++nodes > 160000) return false;
    const cur = stack.pop();
    if (isSolved(cur)) return true;
    for (let si = 0; si < cur.length; si++) {
      for (let di = 0; di < cur.length; di++) {
        if (si === di || !canPour(cur[si], cur[di])) continue;
        if (!cur[di].length && topRun(cur[si]).count === cur[si].length) continue;
        const ng = doPour(cur, si, di);
        const k = tubesKey(ng);
        if (!visited.has(k)) { visited.add(k); stack.push(ng); }
      }
    }
  }
  return false;
}

// Reverse-scramble a solved board, then confirm it is still solvable.
function genPuzzle(colorCount, seed) {
  for (let attempt = 0; attempt < 30; attempt++) {
    const rng = seededRandom(seed + attempt * 7919);
    const tubes = [];
    for (let c = 0; c < colorCount; c++) tubes.push([c, c, c, c]);
    tubes.push([]);
    tubes.push([]);
    const moves = 70 + colorCount * 24;
    for (let k = 0; k < moves; k++) {
      const srcs = [];
      for (let i = 0; i < tubes.length; i++) if (tubes[i].length) srcs.push(i);
      const si = srcs[(rng() * srcs.length) | 0];
      const run = topRun(tubes[si]);
      const dsts = [];
      for (let i = 0; i < tubes.length; i++) {
        if (i !== si && tubes[i].length < TUBE_CAP) dsts.push(i);
      }
      if (!dsts.length) continue;
      const di = dsts[(rng() * dsts.length) | 0];
      const space = TUBE_CAP - tubes[di].length;
      const n = Math.min(run.count, space, 1 + ((rng() * run.count) | 0));
      for (let m = 0; m < n; m++) tubes[di].push(tubes[si].pop());
    }
    if (!isSolved(tubes) && solvable(tubes)) return tubes;
  }
  // fallback: a trivially solvable arrangement
  const tubes = [];
  for (let c = 0; c < colorCount; c++) tubes.push([c, c, c, c]);
  tubes.push([]);
  tubes.push([]);
  return tubes;
}
