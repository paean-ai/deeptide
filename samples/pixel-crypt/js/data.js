// Pixel Crypt - a Mamono-Sweeper-style RPG grid crawler.
//
// Every tile of the crypt hides a monster (level 1..5) or is empty. Reveal
// an empty tile and it shows the SUM of the monster levels around it.
// Reveal a monster and you fight it: stronger-or-equal foes fall for free,
// a foe above your level wounds you by the gap. Defeated foes grant XP;
// enough XP and you level up. Clear every monster to escape the crypt.

const VW = 360, VH = 480;

const MAX_HP = 20;
// Cumulative XP needed to reach hero level 1..6.
const XP_FOR_LEVEL = [0, 0, 3, 8, 15, 24, 36];

// Each level: grid size, monster mix (count per monster level 1..K), seed.
const LEVELS = [
  { name: ['Cellar',    '地窖'], n: 6, mix: [5, 2],            seed: 14 },
  { name: ['Catacomb',  '墓道'], n: 7, mix: [7, 3, 1],         seed: 53 },
  { name: ['Ossuary',   '藏骨堂'], n: 7, mix: [8, 4, 2],       seed: 97 },
  { name: ['Vault',     '密室'], n: 8, mix: [9, 5, 3, 1],      seed: 142 },
  { name: ['Abyss',     '深渊'], n: 8, mix: [10, 6, 4, 2],     seed: 208 },
  { name: ['Throne',    '王座'], n: 9, mix: [11, 7, 5, 3, 1],  seed: 271 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

function heroLevel(xp) {
  let lv = 1;
  for (let k = 2; k < XP_FOR_LEVEL.length; k++) if (xp >= XP_FOR_LEVEL[k]) lv = k;
  return lv;
}

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const n = cfg.n, N = n * n;
  const rng = seededRandom(cfg.seed);
  // monster[cell] = monster level (1..K) or 0 for an empty tile.
  const monster = new Array(N).fill(0);
  const cells = [];
  for (let i = 0; i < N; i++) cells.push(i);
  for (let i = cells.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    const t = cells[i]; cells[i] = cells[j]; cells[j] = t;
  }
  let placed = 0, total = 0;
  for (const c of cfg.mix) total += c;
  for (let lv = 1; lv <= cfg.mix.length; lv++) {
    for (let k = 0; k < cfg.mix[lv - 1]; k++) monster[cells[placed++]] = lv;
  }
  // clue[cell] = sum of neighbouring monster levels (for empty tiles).
  const clue = new Array(N).fill(0);
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      let sum = 0;
      for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++) {
        if (!dr && !dc) continue;
        const nr = r + dr, nc = c + dc;
        if (nr >= 0 && nr < n && nc >= 0 && nc < n) sum += monster[nr * n + nc];
      }
      clue[r * n + c] = sum;
    }
  }
  return {
    levelIndex, cfg, n, monster, clue,
    monsterTotal: total,
    revealed: new Array(N).fill(false),
    flagged: new Array(N).fill(false),
    hp: MAX_HP, xp: 0, level: 1,
    slain: 0,
    flagMode: false,
    over: false, won: false,
    lastHit: -1,                 // cell of the most recent wounding fight
  };
}

// ---- play --------------------------------------------------------------
function toggleFlag(s, cell) {
  if (s.over || s.revealed[cell]) return;
  s.flagged[cell] = !s.flagged[cell];
}

function reveal(s, cell) {
  if (s.over || s.revealed[cell] || s.flagged[cell]) return;
  const m = s.monster[cell];
  if (m > 0) {
    // Fight the monster - it always falls, but a higher level wounds you.
    s.revealed[cell] = true;
    s.slain++;
    s.xp += m;
    if (m > s.level) { s.hp -= (m - s.level); s.lastHit = cell; }
    s.level = heroLevel(s.xp);
    if (s.hp <= 0) { s.hp = 0; s.over = true; s.won = false; return; }
  } else {
    // Empty tile - flood out through tiles with no monsters around them.
    floodReveal(s, cell);
  }
  if (s.slain >= s.monsterTotal) { s.over = true; s.won = true; }
}

function floodReveal(s, start) {
  const n = s.n, stack = [start];
  while (stack.length) {
    const cell = stack.pop();
    if (s.revealed[cell] || s.monster[cell] > 0) continue;
    s.revealed[cell] = true;
    s.flagged[cell] = false;
    if (s.clue[cell] !== 0) continue;          // only a 0-clue spreads
    const r = (cell / n) | 0, c = cell % n;
    for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++) {
      if (!dr && !dc) continue;
      const nr = r + dr, nc = c + dc;
      if (nr >= 0 && nr < n && nc >= 0 && nc < n) {
        const ni = nr * n + nc;
        if (!s.revealed[ni] && s.monster[ni] === 0) stack.push(ni);
      }
    }
  }
}

function finalScore(s) {
  return s.won ? s.hp * 5 + s.xp : 0;
}
