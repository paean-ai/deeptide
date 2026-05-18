// Pixel Match Quest - match-3 engine, levels, UI
(() => {
'use strict';

const SAVE_KEY = 'pixel-match-save';
const $ = id => document.getElementById(id);
const canvas = $('board');
const ctx = canvas.getContext('2d');
const CELL = 72;
const BOARD_PX = GRID * CELL;
canvas.width = BOARD_PX; canvas.height = BOARD_PX;
ctx.imageSmoothingEnabled = false;

// ---- save --------------------------------------------------------------
function loadSave() {
  try { return JSON.parse(localStorage.getItem(SAVE_KEY)) || null; } catch (e) { return null; }
}
let save = loadSave() || { coins: 200, levels: {}, boosters: { hammer: 1, shuffle: 1, moves: 1 } };
function persist() { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); }

// ---- screens -----------------------------------------------------------
function showScreen(id) {
  ['title', 'map', 'game'].forEach(s => $('screen-' + s).classList.toggle('hidden', s !== id));
}

// ---- state -------------------------------------------------------------
let idSeq = 0;
let board = [];      // board[r][c]
let iceGrid = [];    // iceGrid[r][c] = layers
let L = null;        // current level runtime
let phase = 'idle';
let phaseT = 0, phaseDur = 0;
let pending = null;  // pending clear info
let swapPair = null;
let combo = 0;
let particles = [], floats = [];
let hammerMode = false;
let rafId = 0, lastT = 0;

const inb = (r, c) => r >= 0 && c >= 0 && r < GRID && c < GRID;

function makeGem(color) {
  return { type: 'gem', color: color != null ? color : (Math.random() * COLORS) | 0,
    special: null, id: ++idSeq, oy: 0, pop: 1, app: 1 };
}

// ---- board build -------------------------------------------------------
function buildLevel(idx) {
  const def = LEVELS[idx];
  board = []; iceGrid = [];
  for (let r = 0; r < GRID; r++) {
    board.push(new Array(GRID).fill(null));
    iceGrid.push(new Array(GRID).fill(0));
  }
  const layout = def.layout;
  let iceTotal = 0, crateTotal = 0, dropTotal = 0;
  for (let r = 0; r < GRID; r++) {
    for (let c = 0; c < GRID; c++) {
      const ch = layout ? layout[r][c] : '.';
      if (ch === 'x') { board[r][c] = { type: 'crate', layers: 1, id: ++idSeq, oy: 0, pop: 1, app: 1 }; crateTotal++; }
      else if (ch === 'C') { board[r][c] = { type: 'ingredient', id: ++idSeq, oy: 0, pop: 1, app: 1 }; dropTotal++; }
      else {
        board[r][c] = makeGem();
        if (ch === 'i') { iceGrid[r][c] = 1; iceTotal += 1; }
        else if (ch === 'I') { iceGrid[r][c] = 2; iceTotal += 2; }
      }
    }
  }
  // clear any starting matches
  for (let safety = 0; safety < 30; safety++) {
    const runs = scanRuns();
    if (!runs.length) break;
    for (const run of runs) {
      const m = run.cells[(run.cells.length / 2) | 0];
      if (board[m.r][m.c].type === 'gem') board[m.r][m.c].color = (board[m.r][m.c].color + 1 + ((Math.random() * 4) | 0)) % COLORS;
    }
  }
  if (!hasValidMove()) shuffleBoard(true);

  const obj = def.objective;
  L = {
    idx, def, moves: def.moves, score: 0,
    objType: obj.type,
    objNeed: obj.type === 'ice' ? iceTotal : obj.type === 'crate' ? crateTotal
           : obj.type === 'drop' ? (obj.n || dropTotal) : obj.n,
    objHave: 0,
    over: false, won: false,
  };
  if (obj.type === 'score') L.objNeed = obj.n;
}

// ---- match scanning ----------------------------------------------------
function scanRuns() {
  const runs = [];
  for (let r = 0; r < GRID; r++) {
    let c = 0;
    while (c < GRID) {
      const cell = board[r][c];
      if (cell && cell.type === 'gem') {
        let len = 1;
        while (c + len < GRID) {
          const n = board[r][c + len];
          if (n && n.type === 'gem' && n.color === cell.color) len++;
          else break;
        }
        if (len >= 3) {
          const cells = [];
          for (let k = 0; k < len; k++) cells.push({ r, c: c + k });
          runs.push({ cells, color: cell.color, dir: 'h', len });
        }
        c += len;
      } else c++;
    }
  }
  for (let c = 0; c < GRID; c++) {
    let r = 0;
    while (r < GRID) {
      const cell = board[r][c];
      if (cell && cell.type === 'gem') {
        let len = 1;
        while (r + len < GRID) {
          const n = board[r + len][c];
          if (n && n.type === 'gem' && n.color === cell.color) len++;
          else break;
        }
        if (len >= 3) {
          const cells = [];
          for (let k = 0; k < len; k++) cells.push({ r: r + k, c });
          runs.push({ cells, color: cell.color, dir: 'v', len });
        }
        r += len;
      } else r++;
    }
  }
  return runs;
}

function hasValidMove() {
  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++) {
    for (const [dr, dc] of [[0, 1], [1, 0]]) {
      const nr = r + dr, nc = c + dc;
      if (!inb(nr, nc)) continue;
      if (!swappable(r, c) || !swappable(nr, nc)) continue;
      swapCells(r, c, nr, nc);
      const ok = scanRuns().length > 0 ||
        (board[r][c].special) || (board[nr][nc].special);
      swapCells(r, c, nr, nc);
      if (ok) return true;
    }
  }
  return false;
}
function swappable(r, c) {
  const cell = board[r][c];
  return cell && cell.type === 'gem';
}
function swapCells(r1, c1, r2, c2) {
  const tmp = board[r1][c1];
  board[r1][c1] = board[r2][c2];
  board[r2][c2] = tmp;
}
function shuffleBoard(silent) {
  const gems = [];
  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++)
    if (board[r][c] && board[r][c].type === 'gem') gems.push(board[r][c]);
  for (let safety = 0; safety < 60; safety++) {
    for (let i = gems.length - 1; i > 0; i--) {
      const j = (Math.random() * (i + 1)) | 0;
      [gems[i], gems[j]] = [gems[j], gems[i]];
    }
    let k = 0;
    for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++)
      if (board[r][c] && board[r][c].type === 'gem') board[r][c] = gems[k++];
    if (scanRuns().length === 0 && hasValidMove()) break;
  }
  if (!silent) toast(t('shuffled'));
}

// ---- collect clears (special expansion) --------------------------------
function collectClears(seeds) {
  const cleared = new Set();
  const queue = seeds.slice();
  while (queue.length) {
    const { r, c } = queue.pop();
    const key = r + ',' + c;
    if (cleared.has(key)) continue;
    const cell = board[r] && board[r][c];
    if (!cell) continue;
    cleared.add(key);
    if (cell.type === 'gem' && cell.special) {
      const sp = cell.special;
      if (sp === 'rowH') for (let cc = 0; cc < GRID; cc++) queue.push({ r, c: cc });
      else if (sp === 'rowV') for (let rr = 0; rr < GRID; rr++) queue.push({ r: rr, c });
      else if (sp === 'bomb') {
        for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++)
          if (inb(r + dr, c + dc)) queue.push({ r: r + dr, c: c + dc });
      } else if (sp === 'color') {
        const col = cell.colorTarget != null ? cell.colorTarget : cell.color;
        for (let rr = 0; rr < GRID; rr++) for (let cc = 0; cc < GRID; cc++) {
          const g = board[rr][cc];
          if (g && g.type === 'gem' && g.color === col) queue.push({ r: rr, c: cc });
        }
      }
    }
  }
  return cleared;
}

// ---- resolve / cascade -------------------------------------------------
function step() {
  const runs = scanRuns();
  if (runs.length) {
    combo++;
    startClear(runs, null);
    return;
  }
  // collect dropped ingredients at bottom
  let dropped = false;
  for (let c = 0; c < GRID; c++) {
    const cell = board[GRID - 1][c];
    if (cell && cell.type === 'ingredient') {
      board[GRID - 1][c] = null;
      dropped = true;
      if (L.objType === 'drop') L.objHave++;
      floats.push({ x: c * CELL + CELL / 2, y: (GRID - 1) * CELL, str: '🍒', life: 1, color: '#fff' });
      burst(c * CELL + CELL / 2, (GRID - 1) * CELL + CELL / 2, 12, '#ff5a6e');
    }
  }
  if (dropped) {
    applyGravity();
    enterPhase('fall', 0.28);
    return;
  }
  // settled
  combo = 0;
  checkObjective();
  if (!L.over) {
    if (L.moves <= 0) finishLevel();
    else if (!hasValidMove()) { shuffleBoard(false); }
  }
  phase = 'idle';
}

// startClear: matched runs OR forced seeds (specials / hammer)
function startClear(runs, forcedSeeds) {
  // determine special spawns
  const spawns = {}; // key -> {type, color}
  const hSet = new Set(), vSet = new Set();
  if (runs) {
    runs.forEach(run => run.cells.forEach(({ r, c }) =>
      (run.dir === 'h' ? hSet : vSet).add(r + ',' + c)));
    // intersections -> bomb
    for (const key of hSet) {
      if (vSet.has(key)) {
        const [r, c] = key.split(',').map(Number);
        spawns[key] = { type: 'bomb', color: board[r][c].color };
      }
    }
    // 5-runs -> color
    runs.filter(r => r.len >= 5).forEach(run => {
      const cell = pickSpawnCell(run, spawns);
      if (cell) spawns[cell] = { type: 'color', color: run.color };
    });
    // 4-runs -> line
    runs.filter(r => r.len === 4).forEach(run => {
      const cell = pickSpawnCell(run, spawns);
      if (cell) spawns[cell] = { type: run.dir === 'h' ? 'rowH' : 'rowV', color: run.color };
    });
  }
  // seeds = all run cells + forced
  const seeds = [];
  if (runs) runs.forEach(run => run.cells.forEach(p => seeds.push(p)));
  if (forcedSeeds) forcedSeeds.forEach(p => seeds.push(p));
  const cleared = collectClears(seeds);
  // spawn cells survive
  for (const key in spawns) cleared.delete(key);

  pending = { cleared, spawns };
  // mark animation
  for (const key of cleared) {
    const [r, c] = key.split(',').map(Number);
    const cell = board[r][c];
    if (cell) cell.popping = true;
  }
  enterPhase('clear', 0.2);
}
function pickSpawnCell(run, spawns) {
  // prefer the player's swapped cell, else middle
  if (swapPair) {
    for (const p of run.cells) {
      const key = p.r + ',' + p.c;
      if (!spawns[key] && (sameCell(p, swapPair.a) || sameCell(p, swapPair.b))) return key;
    }
  }
  for (let off = 0; off < run.cells.length; off++) {
    const idx = (((run.cells.length / 2) | 0) + off) % run.cells.length;
    const p = run.cells[idx];
    const key = p.r + ',' + p.c;
    if (!spawns[key]) return key;
  }
  return null;
}
function sameCell(a, b) { return a && b && a.r === b.r && a.c === b.c; }

function applyClear() {
  const { cleared, spawns } = pending;
  const mul = 1 + combo * 0.35;
  let clearedGems = 0;
  const crateHits = new Set();
  for (const key of cleared) {
    const [r, c] = key.split(',').map(Number);
    const cell = board[r][c];
    if (!cell) continue;
    if (cell.type === 'crate') { crateHits.add(key); continue; }
    if (cell.type === 'ingredient') continue; // ingredients not cleared
    // a gem
    clearedGems++;
    L.score += Math.round(30 * mul);
    if (L.objType === 'color' && cell.color === L.def.objective.color) L.objHave++;
    // crack ice here
    if (iceGrid[r][c] > 0) iceGrid[r][c]--;
    // damage adjacent crates
    for (const [dr, dc] of [[1,0],[-1,0],[0,1],[0,-1]]) {
      if (inb(r + dr, c + dc) && board[r + dr][c + dc] && board[r + dr][c + dc].type === 'crate')
        crateHits.add((r + dr) + ',' + (dc + c));
    }
    burst(c * CELL + CELL / 2, r * CELL + CELL / 2, 5, GEM_COLORS[cell.color].main);
    board[r][c] = null;
  }
  // damage crates
  for (const key of crateHits) {
    const [r, c] = key.split(',').map(Number);
    const cr = board[r][c];
    if (cr && cr.type === 'crate') {
      cr.layers--;
      burst(c * CELL + CELL / 2, r * CELL + CELL / 2, 8, '#a8804a');
      if (cr.layers <= 0) board[r][c] = null;
    }
  }
  // transform spawns
  for (const key in spawns) {
    const [r, c] = key.split(',').map(Number);
    const sp = spawns[key];
    const cell = board[r][c];
    if (cell && cell.type === 'gem') {
      cell.special = sp.type;
      cell.color = sp.color;
      cell.pop = 1.25;
    }
  }
  if (L.score > 0 && combo >= 3) {
    floats.push({ x: BOARD_PX / 2, y: BOARD_PX / 2 - 30,
      str: combo >= 6 ? t('greatWord') : t('sweetWord'), color: '#ffd34d', life: 1.1 });
  }
  applyGravity();
  enterPhase('fall', 0.28);
}

// ---- gravity -----------------------------------------------------------
function applyGravity() {
  for (let c = 0; c < GRID; c++) {
    let segTop = 0;
    for (let r = 0; r <= GRID; r++) {
      if (r === GRID || (board[r][c] && board[r][c].type === 'crate')) {
        compactSegment(c, segTop, r - 1);
        segTop = r + 1;
      }
    }
  }
}
function compactSegment(c, top, bot) {
  if (bot < top) return;
  const items = [];
  for (let r = bot; r >= top; r--) {
    const cell = board[r][c];
    if (cell && (cell.type === 'gem' || cell.type === 'ingredient')) items.push({ cell, from: r });
    board[r][c] = null;
  }
  let r = bot;
  for (const it of items) {
    board[r][c] = it.cell;
    it.cell.oy = it.from - r;   // negative (came from higher row)
    r--;
  }
  let spawnK = 1;
  while (r >= top) {
    const g = makeGem();
    board[r][c] = g;
    g.oy = -(spawnK + (bot - r));
    g.app = 1;
    r--; spawnK++;
  }
}

// ---- input / moves -----------------------------------------------------
let selected = null;
function tryMove(r1, c1, r2, c2) {
  if (phase !== 'idle' || L.over) return;
  if (!swappable(r1, c1) || !swappable(r2, c2)) return;
  swapPair = { a: { r: r1, c: c1 }, b: { r: r2, c: c2 } };
  swapCells(r1, c1, r2, c2);
  const a = board[r1][c1], b = board[r2][c2];
  const hasMatch = scanRuns().length > 0;
  const special = a.special || b.special;
  if (!hasMatch && !special) {
    // invalid - swap back with animation
    swapCells(r1, c1, r2, c2);
    swapPair = { a: { r: r1, c: c1 }, b: { r: r2, c: c2 }, back: true };
    enterPhase('swapback', 0.18);
    return;
  }
  // valid
  L.moves--;
  combo = 0;
  enterPhase('swap', 0.16);
}
function afterSwap() {
  // special-swap activation without a match
  const { a, b } = swapPair;
  const ca = board[a.r][a.c], cb = board[b.r][b.c];
  const seeds = [];
  let colorClear = -1;
  if (ca && ca.type === 'gem' && ca.special === 'color') { colorClear = cb && cb.type === 'gem' ? cb.color : ca.color; }
  if (cb && cb.type === 'gem' && cb.special === 'color') { colorClear = ca && ca.type === 'gem' ? ca.color : cb.color; }
  if (colorClear >= 0) {
    if (ca && ca.special === 'color') ca.colorTarget = colorClear;
    if (cb && cb.special === 'color') cb.colorTarget = colorClear;
    seeds.push(a, b);
  } else if (((ca && ca.special) || (cb && cb.special)) && scanRuns().length === 0) {
    seeds.push(a, b);
  }
  if (seeds.length) { combo++; startClear(null, seeds); }
  else step();
}

// ---- boosters ----------------------------------------------------------
function useBooster(id) {
  if (phase !== 'idle' || !L || L.over) return;
  if ((save.boosters[id] || 0) <= 0) {
    const cost = BOOSTERS[id].cost;
    if (save.coins < cost) { toast(t('notEnough')); return; }
    save.coins -= cost;
    save.boosters[id] = (save.boosters[id] || 0) + 1;
    persist();
  }
  save.boosters[id]--;
  persist();
  if (id === 'hammer') { hammerMode = true; toast(t('useHammer')); }
  else if (id === 'shuffle') { shuffleBoard(false); }
  else if (id === 'moves') { L.moves += 5; toast(t('movesAdded')); }
  renderHud();
}
function hammerHit(r, c) {
  hammerMode = false;
  if (!swappable(r, c)) { renderHud(); return; }
  combo = 1;
  startClear(null, [{ r, c }]);
}

// ---- objective / end ---------------------------------------------------
function checkObjective() {
  if (L.over) return;
  let met = false;
  if (L.objType === 'ice') met = sumIce() === 0;
  else if (L.objType === 'crate') met = countCrates() === 0;
  else if (L.objType === 'color') met = L.objHave >= L.objNeed;
  else if (L.objType === 'drop') met = L.objHave >= L.objNeed;
  if (met) finishLevel();
}
function sumIce() {
  let s = 0;
  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++) s += iceGrid[r][c];
  return s;
}
function countCrates() {
  let n = 0;
  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++)
    if (board[r][c] && board[r][c].type === 'crate') n++;
  return n;
}
function objComplete() {
  if (L.objType === 'score') return L.score >= L.objNeed;
  if (L.objType === 'ice') return sumIce() === 0;
  if (L.objType === 'crate') return countCrates() === 0;
  return L.objHave >= L.objNeed;
}
function finishLevel() {
  if (L.over) return;
  L.over = true;
  const won = objComplete();
  L.won = won;
  let stars = 0;
  if (won) {
    const th = L.def.stars;
    stars = L.score >= th[2] ? 3 : L.score >= th[1] ? 2 : 1;
    const rec = save.levels[L.idx] || { stars: 0 };
    const coinGain = 40 + stars * 30 + Math.floor(L.score / 200);
    save.coins += coinGain;
    if (stars > rec.stars) rec.stars = stars;
    save.levels[L.idx] = rec;
    L.coinGain = coinGain;
    persist();
  }
  L.stars = stars;
  setTimeout(showResult, 600);
}

// ---- phases ------------------------------------------------------------
function enterPhase(p, dur) { phase = p; phaseT = 0; phaseDur = dur; }
function advancePhase(dt) {
  if (phase === 'idle') return;
  phaseT += dt;
  if (phaseT < phaseDur) return;
  if (phase === 'swap') { afterSwap(); }
  else if (phase === 'swapback') { phase = 'idle'; swapPair = null; }
  else if (phase === 'clear') { applyClear(); }
  else if (phase === 'fall') {
    // settle offsets
    for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++)
      if (board[r][c]) { board[r][c].oy = 0; board[r][c].pop = 1; board[r][c].popping = false; }
    step();
  }
}

// ---- effects -----------------------------------------------------------
function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * 6.28, sp = 60 + Math.random() * 150;
    particles.push({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 40,
      life: 0.45, max: 0.45, size: 3 + Math.random() * 3, color });
  }
}
let toastT = 0;
function toast(msg) { $('toast').textContent = msg; $('toast').classList.remove('hidden'); toastT = 2; }

// ---- rendering ---------------------------------------------------------
function render() {
  ctx.fillStyle = '#241f38';
  ctx.fillRect(0, 0, BOARD_PX, BOARD_PX);
  // cell backdrops
  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++) {
    ctx.fillStyle = (r + c) & 1 ? '#2e2848' : '#332c52';
    ctx.fillRect(c * CELL + 2, r * CELL + 2, CELL - 4, CELL - 4);
  }
  const ease = phase === 'fall' ? 1 - phaseT / phaseDur : phase === 'clear' ? 1 : 0;
  const swapP = (phase === 'swap' || phase === 'swapback') ? phaseT / phaseDur : 0;

  for (let r = 0; r < GRID; r++) for (let c = 0; c < GRID; c++) {
    const cell = board[r][c];
    if (!cell) continue;
    let x = c * CELL, y = r * CELL;
    // falling offset
    if (cell.oy) y += cell.oy * CELL * ease;
    // swap interpolation
    if (swapPair && (phase === 'swap' || phase === 'swapback')) {
      const a = swapPair.a, b = swapPair.b;
      if (r === a.r && c === a.c) { x = lerp(a.c, b.c, swapP) * CELL; y = lerp(a.r, b.r, swapP) * CELL; }
      else if (r === b.r && c === b.c) { x = lerp(b.c, a.c, swapP) * CELL; y = lerp(b.r, a.r, swapP) * CELL; }
    }
    let scale = cell.app < 1 ? cell.app : 1;
    if (cell.popping && phase === 'clear') scale = 1 - phaseT / phaseDur;
    else if (cell.pop > 1) { scale = cell.pop; }
    drawCell(ctx, cell, x, y, CELL, animClock, scale);
    if (iceGrid[r][c] > 0) drawIce(ctx, x, y, CELL, iceGrid[r][c]);
  }
  // selection
  if (selected) {
    ctx.strokeStyle = '#ffd34d'; ctx.lineWidth = 4;
    ctx.strokeRect(selected.c * CELL + 3, selected.r * CELL + 3, CELL - 6, CELL - 6);
  }
  if (hammerMode) {
    ctx.fillStyle = 'rgba(255,90,90,0.12)';
    ctx.fillRect(0, 0, BOARD_PX, BOARD_PX);
  }
  // particles
  for (const p of particles) {
    ctx.globalAlpha = Math.max(0, p.life / p.max);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x - p.size / 2, p.y - p.size / 2, p.size, p.size);
  }
  ctx.globalAlpha = 1;
  // floats
  ctx.textAlign = 'center';
  for (const f of floats) {
    ctx.globalAlpha = Math.min(1, f.life * 1.4);
    ctx.font = 'bold 26px monospace';
    ctx.fillStyle = '#000'; ctx.fillText(f.str, f.x + 2, f.y + 2);
    ctx.fillStyle = f.color; ctx.fillText(f.str, f.x, f.y);
  }
  ctx.globalAlpha = 1;
}
function lerp(a, b, t) { return a + (b - a) * t; }
let animClock = 0;

// ---- HUD ---------------------------------------------------------------
function renderHud() {
  if (!L) return;
  $('hud-moves').textContent = L.moves;
  $('hud-score').textContent = L.score;
  $('hud-coins').textContent = '◆ ' + save.coins;
  // objective
  const obj = L.def.objective;
  let txt, prog;
  if (L.objType === 'score') { txt = t('objScore', L.objNeed); prog = Math.min(100, L.score / L.objNeed * 100); }
  else if (L.objType === 'color') { txt = t('objColor', L.objNeed, colorName(obj.color)); prog = L.objHave / L.objNeed * 100; }
  else if (L.objType === 'ice') { txt = t('objIce'); prog = (1 - sumIce() / Math.max(1, L.objNeed)) * 100; }
  else if (L.objType === 'crate') { txt = t('objCrate'); prog = (1 - countCrates() / Math.max(1, L.objNeed)) * 100; }
  else { txt = t('objDrop', L.objNeed); prog = L.objHave / L.objNeed * 100; }
  $('obj-text').textContent = txt;
  $('obj-fill').style.width = Math.min(100, Math.max(0, prog)) + '%';
  // boosters
  ['hammer', 'shuffle', 'moves'].forEach(id => {
    const el = $('booster-' + id);
    const n = save.boosters[id] || 0;
    el.querySelector('.b-count').textContent = n > 0 ? n : '◆' + BOOSTERS[id].cost;
    el.classList.toggle('buy', n <= 0);
  });
}

// ---- result ------------------------------------------------------------
function showResult() {
  $('result-title').textContent = L.won ? t('levelClear') : t('failed');
  $('result-title').className = L.won ? 'win' : 'lose';
  const sc = $('result-stars');
  sc.innerHTML = '';
  for (let i = 0; i < 3; i++) {
    const s = document.createElement('span');
    s.className = 'rstar' + (i < L.stars ? ' on' : '');
    s.textContent = '★';
    sc.appendChild(s);
  }
  $('result-info').textContent = t('score') + ': ' + L.score +
    (L.won ? '   ' + t('coinsEarned', L.coinGain) : '');
  const nextBtn = $('btn-result-next');
  nextBtn.classList.toggle('hidden', !L.won || L.idx + 1 >= LEVELS.length);
  $('overlay-result').classList.remove('hidden');
}

// ---- map ---------------------------------------------------------------
function renderMap() {
  $('map-coins').textContent = '◆ ' + save.coins;
  const list = $('level-list');
  list.innerHTML = '';
  LEVELS.forEach((lv, i) => {
    const unlocked = i === 0 || (save.levels[i - 1] && save.levels[i - 1].stars > 0);
    const rec = save.levels[i];
    const card = document.createElement('button');
    card.className = 'level-card' + (unlocked ? '' : ' locked');
    let stars = '';
    for (let s = 0; s < 3; s++) stars += `<span class="${rec && s < rec.stars ? 'on' : ''}">★</span>`;
    card.innerHTML = `<span class="lc-num">${i + 1}</span>` +
      `<span class="lc-obj">${objShort(lv)}</span>` +
      `<span class="lc-stars">${unlocked ? stars : '🔒'}</span>`;
    if (unlocked) card.onclick = () => startLevel(i);
    list.appendChild(card);
  });
}
function objShort(lv) {
  const o = lv.objective;
  if (o.type === 'score') return t('objScore', o.n);
  if (o.type === 'color') return t('objColor', o.n, colorName(o.color));
  if (o.type === 'ice') return t('objIce');
  if (o.type === 'crate') return t('objCrate');
  return t('objDrop', o.n);
}

// ---- input -------------------------------------------------------------
let rect = null;
function updateRect() { rect = canvas.getBoundingClientRect(); }
function toCell(cx, cy) {
  if (!rect) updateRect();
  const x = (cx - rect.left) / rect.width * BOARD_PX;
  const y = (cy - rect.top) / rect.height * BOARD_PX;
  return { r: Math.floor(y / CELL), c: Math.floor(x / CELL) };
}
let dragStart = null;
canvas.addEventListener('pointerdown', e => {
  e.preventDefault();
  if (!L || L.over) return;
  const p = toCell(e.clientX, e.clientY);
  if (!inb(p.r, p.c)) return;
  if (hammerMode) { hammerHit(p.r, p.c); return; }
  if (phase !== 'idle') return;
  dragStart = p;
  if (selected && (Math.abs(selected.r - p.r) + Math.abs(selected.c - p.c)) === 1) {
    tryMove(selected.r, selected.c, p.r, p.c);
    selected = null;
  } else {
    selected = swappable(p.r, p.c) ? p : null;
  }
});
canvas.addEventListener('pointerup', e => {
  if (!L || L.over || phase !== 'idle' || !dragStart) return;
  const p = toCell(e.clientX, e.clientY);
  if (inb(p.r, p.c) && (Math.abs(dragStart.r - p.r) + Math.abs(dragStart.c - p.c)) === 1) {
    tryMove(dragStart.r, dragStart.c, p.r, p.c);
    selected = null;
  }
  dragStart = null;
});

// ---- loop --------------------------------------------------------------
function loop(now) {
  rafId = requestAnimationFrame(loop);
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.05) dt = 0.05;
  animClock += dt;
  if (toastT > 0) { toastT -= dt; if (toastT <= 0) $('toast').classList.add('hidden'); }
  if (L && !$('screen-game').classList.contains('hidden')) {
    advancePhase(dt);
    for (const p of particles) { p.life -= dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 300 * dt; }
    particles = particles.filter(p => p.life > 0);
    for (const f of floats) { f.life -= dt; f.y -= 30 * dt; }
    floats = floats.filter(f => f.life > 0);
    render();
    renderHud();
  }
}

// ---- resize ------------------------------------------------------------
function resize() {
  const stage = $('board-stage');
  const m = Math.min(stage.clientWidth, stage.clientHeight);
  canvas.style.width = m + 'px';
  canvas.style.height = m + 'px';
  updateRect();
}
window.addEventListener('resize', resize);
window.addEventListener('orientationchange', () => setTimeout(resize, 200));

// ---- screen flow -------------------------------------------------------
function startLevel(idx) {
  buildLevel(idx);
  phase = 'idle'; selected = null; swapPair = null; combo = 0;
  particles = []; floats = []; hammerMode = false;
  $('overlay-result').classList.add('hidden');
  showScreen('game');
  resize();
  renderHud();
}
function bindUI() {
  $('btn-play').onclick = () => { renderMap(); showScreen('map'); };
  $('btn-map-back').onclick = () => showScreen('title');
  $('btn-result-retry').onclick = () => { $('overlay-result').classList.add('hidden'); startLevel(L.idx); };
  $('btn-result-next').onclick = () => { $('overlay-result').classList.add('hidden'); startLevel(L.idx + 1); };
  $('btn-result-map').onclick = () => { $('overlay-result').classList.add('hidden'); renderMap(); showScreen('map'); };
  $('btn-game-back').onclick = () => { renderMap(); showScreen('map'); };
  ['hammer', 'shuffle', 'moves'].forEach(id => {
    $('booster-' + id).onclick = () => useBooster(id);
  });
  setupLanguageToggle(() => {
    if (!$('screen-map').classList.contains('hidden')) renderMap();
    if (L && !$('screen-game').classList.contains('hidden')) renderHud();
  });
}

bindUI();
applyStaticText();
showScreen('title');
lastT = performance.now();
rafId = requestAnimationFrame(loop);



})();
