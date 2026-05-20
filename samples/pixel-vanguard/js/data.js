// Pixel Vanguard - turn-based mech tactics: levels and combat rules.
//
// Each enemy telegraphs the tile it will strike next turn. On your turn you
// move and attack two mechs - and every attack PUSHES, so knocking a foe aside
// redirects its telegraphed strike off your buildings (or into another foe).

const VW = 360, VH = 480;
const GRID = 6;
const DIRS = [{ dr: -1, dc: 0 }, { dr: 1, dc: 0 }, { dr: 0, dc: -1 }, { dr: 0, dc: 1 }];

// Each level: walls, buildings, the shared core HP, two mechs, and the enemies.
const LEVELS = [
  { name: ['Outset', '序战'], core: 6,
    walls: [], buildings: [[5, 1], [5, 4]],
    heroes: [[4, 2], [4, 3]],
    enemies: [[0, 2, 3], [0, 3, 3]] },
  { name: ['Breach', '突破'], core: 6,
    walls: [[2, 2]], buildings: [[5, 2], [5, 3]],
    heroes: [[4, 1], [4, 4]],
    enemies: [[0, 1, 3], [0, 4, 3], [1, 3, 3]] },
  { name: ['Skyline', '天际'], core: 7,
    walls: [[2, 1], [2, 4]], buildings: [[5, 0], [5, 2], [5, 5]],
    heroes: [[4, 2], [4, 3]],
    enemies: [[0, 1, 3], [0, 3, 3], [0, 5, 3]] },
  { name: ['Onslaught', '猛攻'], core: 7,
    walls: [[2, 2], [2, 3]], buildings: [[5, 1], [5, 3], [5, 5]],
    heroes: [[3, 2], [3, 3]],
    enemies: [[0, 0, 3], [0, 2, 4], [0, 4, 3], [1, 5, 4]] },
  { name: ['Bastion', '壁垒'], core: 8,
    walls: [[2, 0], [2, 3], [3, 5]], buildings: [[5, 1], [5, 3], [5, 5]],
    heroes: [[4, 1], [4, 4]],
    enemies: [[0, 1, 4], [0, 3, 4], [0, 5, 3], [1, 2, 3]] },
  { name: ['Last Stand', '死守'], core: 12,
    walls: [[1, 1], [2, 4], [3, 2]], buildings: [[5, 0], [5, 2], [5, 4]],
    heroes: [[4, 2], [4, 3]],
    enemies: [[0, 0, 4], [0, 2, 3], [0, 4, 4], [0, 5, 3], [1, 3, 3]] },
  { name: ['Crucible', '熔炉'], core: 12,
    walls: [[1, 2], [2, 0], [2, 5], [3, 3]], buildings: [[5, 1], [5, 2], [5, 4]],
    heroes: [[4, 1], [4, 4]],
    enemies: [[0, 0, 4], [0, 1, 3], [0, 3, 5], [0, 5, 4], [1, 4, 3]] },
  { name: ['Rampart', '城墙'], core: 14,
    walls: [[2, 0], [2, 5], [3, 0], [3, 5]],
    buildings: [[5, 0], [5, 2], [5, 3], [5, 5]],
    heroes: [[4, 2], [4, 3]],
    enemies: [[0, 0, 4], [0, 2, 4], [0, 3, 4], [0, 5, 4], [1, 1, 3], [1, 4, 3]] },
];
const LEVEL_COUNT = LEVELS.length;

const HERO_HP = 5, HERO_ATK = 3, HERO_MOVE = 3, ENEMY_ATK = 2;

function buildState(index) {
  const L = LEVELS[index];
  let id = 0;
  const s = {
    index, w: GRID, h: GRID,
    walls: L.walls.map(w => w.slice()),
    buildings: L.buildings.map(([r, c]) => ({ r, c })),
    core: L.core, coreMax: L.core,
    heroes: L.heroes.map(([r, c]) => ({ id: id++, r, c, hp: HERO_HP, maxhp: HERO_HP, atk: HERO_ATK, move: HERO_MOVE, moved: false, acted: false })),
    enemies: L.enemies.map(([r, c, hp]) => ({ id: id++, r, c, hp, maxhp: hp, atk: ENEMY_ATK, facing: null })),
    turn: 'player', over: false, won: false, log: '',
  };
  for (const e of s.enemies) e.facing = enemyAimFacing(s, e);
  return s;
}

// ---- board queries -------------------------------------------------------
function inBounds(s, r, c) { return r >= 0 && c >= 0 && r < s.h && c < s.w; }
function wallAt(s, r, c) { return s.walls.some(w => w[0] === r && w[1] === c); }
function buildingAt(s, r, c) { return s.buildings.find(b => b.r === r && b.c === c); }
function heroAt(s, r, c) { return s.heroes.find(h => h.r === r && h.c === c); }
function enemyAt(s, r, c) { return s.enemies.find(e => e.r === r && e.c === c); }
function solid(s, r, c) {
  return !inBounds(s, r, c) || wallAt(s, r, c) || !!buildingAt(s, r, c) ||
    !!heroAt(s, r, c) || !!enemyAt(s, r, c);
}

// tiles a mech can reach (BFS through empty tiles), as a map "r,c" -> distance
function reachable(s, hero) {
  const seen = { [hero.r + ',' + hero.c]: 0 };
  let frontier = [[hero.r, hero.c]];
  for (let step = 0; step < hero.move; step++) {
    const next = [];
    for (const [r, c] of frontier) {
      for (const d of DIRS) {
        const nr = r + d.dr, nc = c + d.dc, k = nr + ',' + nc;
        if (k in seen || solid(s, nr, nc)) continue;
        seen[k] = step + 1;
        next.push([nr, nc]);
      }
    }
    frontier = next;
  }
  return seen;
}

// enemies orthogonally adjacent to a mech
function attackTargets(s, hero) {
  return s.enemies.filter(e => Math.abs(e.r - hero.r) + Math.abs(e.c - hero.c) === 1);
}

// ---- player actions ------------------------------------------------------
function moveHero(s, hero, r, c) {
  hero.r = r; hero.c = c;
  hero.moved = true;
}

// push a unit one tile in dir d; blocked -> collision damage to both sides
function pushUnit(s, unit, d) {
  const tr = unit.r + d.dr, tc = unit.c + d.dc;
  if (!inBounds(s, tr, tc) || wallAt(s, tr, tc) || buildingAt(s, tr, tc)) {
    unit.hp -= 1;
    return;
  }
  const other = enemyAt(s, tr, tc) || heroAt(s, tr, tc);
  if (other) { unit.hp -= 1; other.hp -= 1; return; }
  unit.r = tr; unit.c = tc;
}

function heroAttack(s, hero, enemy) {
  enemy.hp -= hero.atk;
  if (enemy.hp > 0) {
    const d = { dr: Math.sign(enemy.r - hero.r), dc: Math.sign(enemy.c - hero.c) };
    pushUnit(s, enemy, d);
  }
  hero.acted = true;
  cleanup(s);
  checkOver(s);
}

function cleanup(s) {
  s.enemies = s.enemies.filter(e => e.hp > 0);
  s.heroes = s.heroes.filter(h => h.hp > 0);
}

// ---- enemy turn ----------------------------------------------------------
function nearestTarget(s, e) {
  let best = null, bd = 1e9;
  const cands = s.buildings.map(b => [b.r, b.c]).concat(s.heroes.map(h => [h.r, h.c]));
  for (const [r, c] of cands) {
    const d = Math.abs(r - e.r) + Math.abs(c - e.c);
    if (d < bd) { bd = d; best = [r, c]; }
  }
  return { pos: best, dist: bd };
}
function enemyAimFacing(s, e) {
  const { pos } = nearestTarget(s, e);
  if (!pos) return DIRS[1];
  let best = DIRS[0], bd = 1e9;
  for (const d of DIRS) {
    const dd = Math.abs(e.r + d.dr - pos[0]) + Math.abs(e.c + d.dc - pos[1]);
    if (dd < bd) { bd = dd; best = d; }
  }
  return best;
}
function enemyStep(s, e) {
  const { pos, dist } = nearestTarget(s, e);
  if (!pos || dist <= 1) return;
  const opts = DIRS
    .map(d => ({ nr: e.r + d.dr, nc: e.c + d.dc }))
    .filter(o => !solid(s, o.nr, o.nc))
    .map(o => ({ ...o, nd: Math.abs(o.nr - pos[0]) + Math.abs(o.nc - pos[1]) }))
    .sort((a, b) => a.nd - b.nd);
  if (opts.length && opts[0].nd < dist) { e.r = opts[0].nr; e.c = opts[0].nc; }
}

// resolve telegraphed strikes, then enemies advance and re-aim
function endPlayerTurn(s) {
  if (s.over) return;
  for (const e of s.enemies) {
    if (!e.facing) continue;
    const tr = e.r + e.facing.dr, tc = e.c + e.facing.dc;
    const h = heroAt(s, tr, tc);
    if (h) h.hp -= e.atk;
    else if (buildingAt(s, tr, tc)) s.core -= e.atk;
  }
  cleanup(s);
  if (checkOver(s)) return;
  for (const e of s.enemies) enemyStep(s, e);
  for (const e of s.enemies) e.facing = enemyAimFacing(s, e);
  for (const h of s.heroes) { h.moved = false; h.acted = false; }
  s.turn = 'player';
  checkOver(s);
}

function checkOver(s) {
  if (s.enemies.length === 0) { s.over = true; s.won = true; return true; }
  if (s.core <= 0 || s.heroes.length === 0) { s.over = true; s.won = false; return true; }
  return false;
}
