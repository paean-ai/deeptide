// Pixel Foray - turn-based telegraphed-tactics dungeon. Pure battle logic.
//
// A 7x7 room. Each turn every enemy LOCKS its plan and shows it: a ghost on
// the cell a melee foe will step to, a red line for an archer's shot. Then
// the hero acts once - move to a neighbour, strike an adjacent foe, or wait -
// and the enemies execute exactly what they telegraphed. Striking a foe dead
// cancels its plan; only wounding it does not. Clear the room of foes.

const VW = 360, VH = 480;
const GRID = 7;
const HERO_HP = 8;
const HERO_DMG = 2;
const ARCHER_RANGE = 5;

const ETYPES = {
  grunt:  { hp: 2, melee: true,  slow: false, name: ['Grunt', '小卒'] },
  brute:  { hp: 4, melee: true,  slow: true,  name: ['Brute', '蛮兵'] },
  archer: { hp: 2, melee: false, slow: false, name: ['Archer', '弓手'] },
};

// Each room: wall cells, the hero's start cell, and the foes. Hand-authored
// and verified by a depth-search bot in the test - every room is clearable.
const ROOMS = [
  { name: ['Threshold', '门厅'], walls: [],
    hero: 45, enemies: [{ t: 'grunt', c: 9 }, { t: 'grunt', c: 11 }] },
  { name: ['Guardroom', '守卫室'], walls: [17, 24, 31],
    hero: 45, enemies: [{ t: 'grunt', c: 1 }, { t: 'grunt', c: 3 }, { t: 'grunt', c: 5 }] },
  { name: ['Crossway', '十字厅'], walls: [16, 18, 30, 32],
    hero: 45, enemies: [{ t: 'grunt', c: 0 }, { t: 'grunt', c: 6 }, { t: 'archer', c: 3 }] },
  { name: ['Stronghold', '堡垒'], walls: [15, 19, 29, 33],
    hero: 45, enemies: [{ t: 'brute', c: 24 }, { t: 'grunt', c: 1 }, { t: 'grunt', c: 5 }] },
  { name: ['Gallery', '回廊'], walls: [9, 11, 13, 35, 37, 39],
    hero: 45, enemies: [{ t: 'archer', c: 0 }, { t: 'archer', c: 6 }, { t: 'grunt', c: 23 }, { t: 'grunt', c: 25 }] },
  { name: ['Throne', '王座厅'], walls: [16, 18, 23, 25, 30, 32],
    hero: 45, enemies: [{ t: 'brute', c: 24 }, { t: 'archer', c: 0 }, { t: 'archer', c: 6 }, { t: 'grunt', c: 9 }, { t: 'grunt', c: 13 }] },
];
const ROOM_COUNT = ROOMS.length;

function rc(cell) { return [cell / GRID | 0, cell % GRID]; }
function cellOf(r, c) { return r * GRID + c; }
function inGrid(r, c) { return r >= 0 && c >= 0 && r < GRID && c < GRID; }
function manhattan(a, b) { const [ar, ac] = rc(a), [br, bc] = rc(b); return Math.abs(ar - br) + Math.abs(ac - bc); }
function adjacent(a, b) { return manhattan(a, b) === 1; }

const STEPS = [[-1, 0], [1, 0], [0, -1], [0, 1]];

function newRoom(roomIndex) {
  const room = ROOMS[roomIndex];
  const s = {
    roomIndex, room,
    walls: new Set(room.walls),
    hero: { cell: room.hero, hp: HERO_HP },
    enemies: room.enemies.map((e, i) => ({
      id: i, type: e.t, hp: ETYPES[e.t].hp, cell: e.c, plan: null,
    })),
    turn: 0,
    over: false, won: false,
  };
  planEnemies(s);
  return s;
}

function enemyAt(s, cell) {
  for (const e of s.enemies) if (e.cell === cell) return e;
  return null;
}
function blockedForPlan(s, cell, reserved) {
  if (s.walls.has(cell)) return true;
  if (cell === s.hero.cell) return true;
  if (enemyAt(s, cell)) return true;
  if (reserved.has(cell)) return true;
  return false;
}

// one step from `cell` toward `target`, skirting walls; null if stuck
function stepToward(s, cell, target, reserved) {
  const [r, c] = rc(cell), [tr, tc] = rc(target);
  const dr = Math.sign(tr - r), dc = Math.sign(tc - c);
  const order = [];
  if (Math.abs(tr - r) >= Math.abs(tc - c)) {
    if (dr) order.push([dr, 0]);
    if (dc) order.push([0, dc]);
  } else {
    if (dc) order.push([0, dc]);
    if (dr) order.push([dr, 0]);
  }
  for (const [sr, sc] of STEPS) {           // perpendicular skirting fallback
    if (!order.some(o => o[0] === sr && o[1] === sc)) order.push([sr, sc]);
  }
  let best = null, bestDist = manhattan(cell, target);
  for (const [sr, sc] of order) {
    if (!inGrid(r + sr, c + sc)) continue;
    const dest = cellOf(r + sr, c + sc);
    if (blockedForPlan(s, dest, reserved)) continue;
    const d = manhattan(dest, target);
    if (d < bestDist) return dest;          // a real advance - take it
    if (best === null && d <= bestDist) best = dest;
  }
  return best;
}

// the straight line of cells an archer threatens toward the hero
function fireLine(s, archerCell, heroCell) {
  const [ar, ac] = rc(archerCell), [hr, hc] = rc(heroCell);
  if (ar !== hr && ac !== hc) return null;
  if (manhattan(archerCell, heroCell) > ARCHER_RANGE) return null;
  const dr = Math.sign(hr - ar), dc = Math.sign(hc - ac);
  const line = [];
  let r = ar + dr, c = ac + dc, steps = 0;
  while (inGrid(r, c) && steps < ARCHER_RANGE) {
    const cell = cellOf(r, c);
    if (s.walls.has(cell)) break;
    line.push(cell);
    if (cell === heroCell) return line;     // LOS reaches the hero
    r += dr; c += dc; steps++;
  }
  return null;                              // blocked before reaching the hero
}

// lock every enemy's plan for the upcoming hero turn
function planEnemies(s) {
  const reserved = new Set();
  for (const e of s.enemies) {
    const def = ETYPES[e.type];
    if (!def.melee) {
      const line = fireLine(s, e.cell, s.hero.cell);
      if (line) { e.plan = { fire: line }; continue; }
    }
    if (def.slow && (s.turn % 2 === 1)) { e.plan = { move: e.cell }; continue; }
    const dest = stepToward(s, e.cell, s.hero.cell, reserved);
    const move = dest === null ? e.cell : dest;
    e.plan = { move };
    reserved.add(move);
  }
}

// run the locked enemy phase; if `heroCellOverride` is given, treat the hero
// as standing there and DO NOT mutate `s` - used for danger prediction.
function resolveEnemies(s, heroCellOverride) {
  const sim = heroCellOverride !== undefined;
  const heroCell = sim ? heroCellOverride : s.hero.cell;
  const pos = {};
  for (const e of s.enemies) pos[e.id] = e.cell;
  // melee enemies move to their locked destination
  for (const e of s.enemies) {
    if (!e.plan || !e.plan.move) continue;
    let dest = e.plan.move;
    if (dest === heroCell) dest = e.cell;            // hero blocks the cell
    else {
      for (const o of s.enemies) {
        if (o.id !== e.id && pos[o.id] === dest) { dest = e.cell; break; }
      }
    }
    pos[e.id] = dest;
  }
  // resolve damage to the hero
  let dmg = 0;
  for (const e of s.enemies) {
    if (e.plan && e.plan.fire) {
      if (e.plan.fire.indexOf(heroCell) >= 0) dmg++;
    } else if (adjacent(pos[e.id], heroCell)) {
      dmg++;
    }
  }
  if (sim) return dmg;
  for (const e of s.enemies) e.cell = pos[e.id];
  s.hero.hp -= dmg;
  s.turn++;
  if (s.hero.hp <= 0) { s.hero.hp = 0; s.over = true; s.won = false; }
  else if (s.enemies.length === 0) { s.over = true; s.won = true; }
  else planEnemies(s);
  return dmg;
}

// would the hero take damage ending this turn on `cell`?
function wouldTakeDamageAt(s, cell) { return resolveEnemies(s, cell) > 0; }

function dangerCells(s) {
  const set = new Set();
  for (let c = 0; c < GRID * GRID; c++) {
    if (s.walls.has(c)) continue;
    if (wouldTakeDamageAt(s, c)) set.add(c);
  }
  return set;
}

// the hero acts on a tapped cell. returns true if a turn was spent.
function heroTap(s, cell) {
  if (s.over) return false;
  if (cell === s.hero.cell) { resolveEnemies(s); return true; }   // wait
  if (!adjacent(cell, s.hero.cell)) return false;
  const foe = enemyAt(s, cell);
  if (foe) {
    foe.hp -= HERO_DMG;
    if (foe.hp <= 0) s.enemies = s.enemies.filter(e => e.id !== foe.id);
    if (s.enemies.length === 0) { s.over = true; s.won = true; return true; }
  } else {
    if (s.walls.has(cell)) return false;
    s.hero.cell = cell;
  }
  resolveEnemies(s);
  return true;
}

function legalMoves(s) {
  const out = [s.hero.cell];                 // wait
  const [r, c] = rc(s.hero.cell);
  for (const [sr, sc] of STEPS) {
    if (!inGrid(r + sr, c + sc)) continue;
    const cell = cellOf(r + sr, c + sc);
    if (enemyAt(s, cell)) out.push(cell);     // attack
    else if (!s.walls.has(cell)) out.push(cell); // move
  }
  return out;
}

function stars(s) {
  if (s.hero.hp >= 7) return 3;
  if (s.hero.hp >= 4) return 2;
  return 1;
}
