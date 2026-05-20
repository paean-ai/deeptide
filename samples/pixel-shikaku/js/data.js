// Pixel Shikaku - divide the grid into rectangles. Each rectangle contains
// exactly one number, and its area must equal that number.
//
// Each level seeds a random rectangle tiling, picks one cell of each rectangle
// as the clue, then a backtracking solver verifies the puzzle has exactly one
// solution.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Tile', '小格'],     seed: 19,  w: 5, h: 5 },
  { name: ['Plot', '田园'],     seed: 53,  w: 5, h: 6 },
  { name: ['Garden', '花园'],   seed: 112, w: 6, h: 6 },
  { name: ['Atrium', '中庭'],   seed: 188, w: 6, h: 7 },
  { name: ['Plaza', '广场'],    seed: 277, w: 7, h: 7 },
  { name: ['District', '街区'], seed: 384, w: 7, h: 8 },
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

const ix = (w, r, c) => r * w + c;

function shuffle(a, rng) {
  for (let k = a.length - 1; k > 0; k--) {
    const j = (rng() * (k + 1)) | 0;
    [a[k], a[j]] = [a[j], a[k]];
  }
}

// ---- tiling --------------------------------------------------------------
// Tile a w x h grid with random rectangles (areas roughly 1..6). May fail to
// cover; caller retries with a fresh rng.
function tile(w, h, rng) {
  const occ = new Int8Array(w * h);
  const rects = [];
  for (let r = 0; r < h; r++) {
    for (let c = 0; c < w; c++) {
      if (occ[ix(w, r, c)]) continue;
      const tried = [];
      // candidate sizes within bounds
      for (let rh = 1; rh <= Math.min(4, h - r); rh++) {
        for (let rw = 1; rw <= Math.min(5, w - c); rw++) {
          if (rh * rw > 6) continue;
          tried.push({ rh, rw });
        }
      }
      shuffle(tried, rng);
      // prefer non-1x1 by re-sorting non-trivial sizes to the front
      tried.sort((a, b) => (a.rh * a.rw === 1) - (b.rh * b.rw === 1));
      let placed = null;
      for (const s of tried) {
        let ok = true;
        for (let dr = 0; dr < s.rh && ok; dr++) {
          for (let dc = 0; dc < s.rw && ok; dc++) {
            if (occ[ix(w, r + dr, c + dc)]) ok = false;
          }
        }
        if (ok) { placed = s; break; }
      }
      const rec = { r, c, rh: placed.rh, rw: placed.rw };
      for (let dr = 0; dr < rec.rh; dr++)
        for (let dc = 0; dc < rec.rw; dc++) occ[ix(w, r + dr, c + dc)] = 1;
      rects.push(rec);
    }
  }
  return rects;
}

// ---- solver --------------------------------------------------------------
// All rectangles of area `A` whose bounding box contains (cr, cc) and fits in
// the w x h grid. Returns list of {r, c, rh, rw}.
function rectsContaining(w, h, cr, cc, A) {
  const out = [];
  for (let rh = 1; rh <= A; rh++) {
    if (A % rh !== 0) continue;
    const rw = A / rh;
    if (rh > h || rw > w) continue;
    const r0 = Math.max(0, cr - rh + 1), r1 = Math.min(h - rh, cr);
    const c0 = Math.max(0, cc - rw + 1), c1 = Math.min(w - rw, cc);
    for (let r = r0; r <= r1; r++)
      for (let c = c0; c <= c1; c++) out.push({ r, c, rh, rw });
  }
  return out;
}

function rectCovers(rec, cr, cc) {
  return cr >= rec.r && cr < rec.r + rec.rh && cc >= rec.c && cc < rec.c + rec.rw;
}

// Count solutions for a given clue list, stopping at `limit`.
function solveCount(w, h, clues, limit) {
  const owner = new Int8Array(w * h).fill(-1);
  for (let i = 0; i < owner.length; i++) owner[i] = -1;
  let found = 0;
  // pre-compute candidates per clue + order clues by fewest options
  const order = clues.map((_, i) => i)
    .sort((a, b) => clues[a].n - clues[b].n);
  function bt(k) {
    if (found >= limit) return;
    if (k === order.length) {
      for (let i = 0; i < owner.length; i++) if (owner[i] === -1) return;
      found++;
      return;
    }
    const ci = order[k];
    const clue = clues[ci];
    const opts = rectsContaining(w, h, clue.r, clue.c, clue.n);
    for (const rec of opts) {
      // check no cell already owned, and no OTHER clue inside
      let ok = true;
      for (let dr = 0; dr < rec.rh && ok; dr++) {
        for (let dc = 0; dc < rec.rw && ok; dc++) {
          const cell = ix(w, rec.r + dr, rec.c + dc);
          if (owner[cell] !== -1) ok = false;
        }
      }
      if (!ok) continue;
      let extraClue = false;
      for (let i = 0; i < clues.length && !extraClue; i++) {
        if (i === ci) continue;
        if (rectCovers(rec, clues[i].r, clues[i].c)) extraClue = true;
      }
      if (extraClue) continue;
      // place
      for (let dr = 0; dr < rec.rh; dr++)
        for (let dc = 0; dc < rec.rw; dc++) owner[ix(w, rec.r + dr, rec.c + dc)] = ci;
      bt(k + 1);
      for (let dr = 0; dr < rec.rh; dr++)
        for (let dc = 0; dc < rec.rw; dc++) owner[ix(w, rec.r + dr, rec.c + dc)] = -1;
      if (found >= limit) return;
    }
  }
  bt(0);
  return found;
}

// Build a uniquely-solvable Shikaku.
function buildPuzzle(level) {
  const rng = seededRandom(level.seed);
  for (let attempt = 0; attempt < 200; attempt++) {
    const rects = tile(level.w, level.h, rng);
    // pick a clue cell inside each rectangle
    const clues = rects.map(r => {
      const dr = (rng() * r.rh) | 0, dc = (rng() * r.rw) | 0;
      return { r: r.r + dr, c: r.c + dc, n: r.rh * r.rw };
    });
    if (solveCount(level.w, level.h, clues, 2) === 1) {
      return { w: level.w, h: level.h, clues, solution: rects };
    }
  }
  return null;
}

// ---- evaluation (game side) ----------------------------------------------
// Validate a player rectangle: must contain exactly one clue cell, area must
// match that clue's number, and (excluding overwriting that clue's existing
// rectangle) not overlap any other clue's rectangle.
function validateRect(pz, rects, rect) {
  let clueIdx = -1, clueCount = 0;
  for (let i = 0; i < pz.clues.length; i++) {
    if (rectCovers(rect, pz.clues[i].r, pz.clues[i].c)) { clueIdx = i; clueCount++; }
  }
  if (clueCount !== 1) return { ok: false };
  const clue = pz.clues[clueIdx];
  const area = rect.rh * rect.rw;
  if (area !== clue.n) return { ok: false };
  // overlap check vs other rectangles (allow replacing rects[clueIdx])
  for (let i = 0; i < rects.length; i++) {
    if (!rects[i] || i === clueIdx) continue;
    if (rectsOverlap(rects[i], rect)) return { ok: false };
  }
  return { ok: true, clueIdx };
}
function rectsOverlap(a, b) {
  return !(a.c + a.rw <= b.c || b.c + b.rw <= a.c ||
           a.r + a.rh <= b.r || b.r + b.rh <= a.r);
}

function evaluate(pz, rects) {
  const cov = new Int8Array(pz.w * pz.h);
  let ok = true;
  for (let i = 0; i < rects.length; i++) {
    const rec = rects[i];
    if (!rec) { ok = false; continue; }
    for (let dr = 0; dr < rec.rh; dr++)
      for (let dc = 0; dc < rec.rw; dc++) {
        const cell = ix(pz.w, rec.r + dr, rec.c + dc);
        if (cov[cell]) ok = false;
        cov[cell] = 1;
      }
  }
  for (let i = 0; i < cov.length && ok; i++) if (!cov[i]) ok = false;
  return { solved: ok };
}
