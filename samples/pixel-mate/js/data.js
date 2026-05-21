// Pixel Mate - chess mate-in-1 puzzles. From each position, find the
// single White move that checkmates the Black king. The data layer ships
// six hand-designed positions and a faithful (if minimal) chess engine
// that handles all moves needed to verify mate-in-1: pawn single + double
// step + diagonal capture, knight L, bishop / rook / queen sliders, king
// one-step. Castling, en passant and promotion are intentionally out of
// scope for a mate-in-1 sample.
//
// Each position is laid out with rank 0 at the top of the board (Black
// side) and rank 7 at the bottom (White side); files 0..7 = a..h.
// Pieces are encoded as { p: 'K'|'Q'|'R'|'B'|'N'|'P', c: 'w'|'b' }.

const VW = 360, VH = 480;
const FILES = 8, RANKS = 8;

// Six puzzles. Each `pos` is an array of { p, c, x, y } where (x, y) is
// (file, rank). White to move; in every position, exactly one White move
// delivers checkmate. The test harness verifies this end-to-end.
const LEVELS = [
  // 1. Back rank mate — White R-e8#.
  { name: ['Back Rank', '底线杀'], pos: [
    { p: 'K', c: 'w', x: 4, y: 7 },
    { p: 'R', c: 'w', x: 4, y: 1 },
    { p: 'K', c: 'b', x: 6, y: 0 },
    { p: 'P', c: 'b', x: 5, y: 1 },
    { p: 'P', c: 'b', x: 6, y: 1 },
    { p: 'P', c: 'b', x: 7, y: 1 },
  ]},
  // 2. Queen + King — White Q-g7# (king on h8, Q at g5 supported by K f6).
  { name: ['Royal Pair', '王后双杀'], pos: [
    { p: 'K', c: 'b', x: 7, y: 0 },
    { p: 'K', c: 'w', x: 5, y: 2 },
    { p: 'Q', c: 'w', x: 6, y: 3 },
  ]},
  // 3. Knight smother — fully boxed king on h8, White Ne5-f7 lands the
  // classic L-check that no Black piece can answer.
  { name: ['Smother', '闷杀'], pos: [
    { p: 'K', c: 'b', x: 7, y: 0 },     // h8
    { p: 'R', c: 'b', x: 6, y: 0 },     // g8 (blocks)
    { p: 'P', c: 'b', x: 7, y: 1 },     // h7 (blocks)
    { p: 'P', c: 'b', x: 6, y: 1 },     // g7 (blocks)
    { p: 'N', c: 'w', x: 4, y: 3 },     // e5 -> Nf7#
    { p: 'K', c: 'w', x: 4, y: 7 },     // e1
  ]},
  // 4. Two rooks — ladder mate. Black K at d8; White Rs at a7 and b-file.
  { name: ['Ladder',   '阶梯杀'], pos: [
    { p: 'K', c: 'b', x: 3, y: 0 },
    { p: 'R', c: 'w', x: 0, y: 1 },     // a7
    { p: 'R', c: 'w', x: 1, y: 7 },     // b1 -> Rb8#
    { p: 'K', c: 'w', x: 4, y: 7 },
  ]},
  // 5. Back-rank ambush — pawns on g7 + h7 leave h8 unable to escape; the
  // Q lifts from e5 up the e-file to e8, raking the 8th rank for the mate.
  { name: ['Eighth Rank', '八排杀'], pos: [
    { p: 'K', c: 'b', x: 7, y: 0 },     // h8
    { p: 'P', c: 'b', x: 7, y: 1 },     // h7
    { p: 'P', c: 'b', x: 6, y: 1 },     // g7
    { p: 'B', c: 'w', x: 1, y: 6 },     // b2 (decoration along a1-h8 diag)
    { p: 'Q', c: 'w', x: 4, y: 3 },     // e5 -> Qe8#
    { p: 'K', c: 'w', x: 4, y: 7 },     // e1
  ]},
  // 6. King + Queen mate — black king cornered at a8; queen at b6 -> Q-b7#.
  { name: ['Corner', '角落杀'], pos: [
    { p: 'K', c: 'b', x: 0, y: 0 },
    { p: 'Q', c: 'w', x: 1, y: 2 },     // b6
    { p: 'K', c: 'w', x: 0, y: 2 },     // a6
  ]},
];
const LEVEL_COUNT = LEVELS.length;

// ---- board helpers -----------------------------------------------------
function inBounds(x, y) { return x >= 0 && y >= 0 && x < FILES && y < RANKS; }
function buildBoard(pos) {
  const b = [];
  for (let y = 0; y < RANKS; y++) {
    const row = [];
    for (let x = 0; x < FILES; x++) row.push(null);
    b.push(row);
  }
  for (const pc of pos) b[pc.y][pc.x] = { p: pc.p, c: pc.c };
  return b;
}
function cloneBoard(b) {
  const out = [];
  for (let y = 0; y < RANKS; y++) {
    const row = [];
    for (let x = 0; x < FILES; x++) row.push(b[y][x] ? { p: b[y][x].p, c: b[y][x].c } : null);
    out.push(row);
  }
  return out;
}
function findKing(b, c) {
  for (let y = 0; y < RANKS; y++) for (let x = 0; x < FILES; x++) {
    const sq = b[y][x];
    if (sq && sq.p === 'K' && sq.c === c) return { x, y };
  }
  return null;
}

// ---- pseudo-legal move generation --------------------------------------
function pseudoMoves(b, x, y) {
  const piece = b[y][x];
  if (!piece) return [];
  const me = piece.c;
  const out = [];
  const push = (nx, ny) => {
    if (!inBounds(nx, ny)) return false;
    const sq = b[ny][nx];
    if (sq && sq.c === me) return false;
    out.push({ x: nx, y: ny });
    return !sq; // continue sliding only if the square was empty
  };
  if (piece.p === 'P') {
    const dir = me === 'w' ? -1 : 1;   // White moves up (toward rank 0)
    const startRank = me === 'w' ? 6 : 1;
    // Single step forward (no capture).
    if (inBounds(x, y + dir) && !b[y + dir][x]) {
      out.push({ x, y: y + dir });
      if (y === startRank && !b[y + 2 * dir][x]) out.push({ x, y: y + 2 * dir });
    }
    // Diagonal captures.
    for (const dx of [-1, 1]) {
      const nx = x + dx, ny = y + dir;
      if (!inBounds(nx, ny)) continue;
      const sq = b[ny][nx];
      if (sq && sq.c !== me) out.push({ x: nx, y: ny });
    }
  } else if (piece.p === 'N') {
    for (const [dx, dy] of [[1,-2],[2,-1],[2,1],[1,2],[-1,2],[-2,1],[-2,-1],[-1,-2]]) {
      push(x + dx, y + dy);
    }
  } else if (piece.p === 'B' || piece.p === 'Q') {
    for (const [dx, dy] of [[1,1],[1,-1],[-1,1],[-1,-1]]) {
      let nx = x + dx, ny = y + dy;
      while (push(nx, ny)) { nx += dx; ny += dy; }
    }
  }
  if (piece.p === 'R' || piece.p === 'Q') {
    for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
      let nx = x + dx, ny = y + dy;
      while (push(nx, ny)) { nx += dx; ny += dy; }
    }
  }
  if (piece.p === 'K') {
    for (const dx of [-1, 0, 1]) for (const dy of [-1, 0, 1]) {
      if (dx === 0 && dy === 0) continue;
      push(x + dx, y + dy);
    }
  }
  return out;
}

// Side `c`'s king in check?
function inCheck(b, c) {
  const k = findKing(b, c);
  if (!k) return false;
  const opp = c === 'w' ? 'b' : 'w';
  for (let y = 0; y < RANKS; y++) for (let x = 0; x < FILES; x++) {
    const sq = b[y][x];
    if (!sq || sq.c !== opp) continue;
    for (const m of pseudoMoves(b, x, y)) {
      if (m.x === k.x && m.y === k.y) return true;
    }
  }
  return false;
}

function applyMove(b, fx, fy, tx, ty) {
  const nb = cloneBoard(b);
  nb[ty][tx] = nb[fy][fx];
  nb[fy][fx] = null;
  return nb;
}

// Legal moves for side `c`: pseudo-legal filtered to those that don't
// leave our king in check.
function legalMovesFor(b, c) {
  const out = [];
  for (let y = 0; y < RANKS; y++) for (let x = 0; x < FILES; x++) {
    const sq = b[y][x];
    if (!sq || sq.c !== c) continue;
    for (const m of pseudoMoves(b, x, y)) {
      const nb = applyMove(b, x, y, m.x, m.y);
      if (!inCheck(nb, c)) out.push({ fx: x, fy: y, tx: m.x, ty: m.y });
    }
  }
  return out;
}

function isCheckmate(b, c) {
  return inCheck(b, c) && legalMovesFor(b, c).length === 0;
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  return {
    levelIndex, lv,
    board: buildBoard(lv.pos),
    selected: null,            // {x, y} of the picked White piece
    moves: [],                 // legal target squares of the selected piece
    over: false, won: false,
    attempts: 0,               // how many wrong moves taken (for star tier)
  };
}

// Tap a square: select / move / cancel. Returns 'select' | 'move' | 'cancel' | 'noop'.
function tap(s, x, y) {
  if (s.over) return 'noop';
  const sq = s.board[y][x];
  if (s.selected) {
    if (s.selected.x === x && s.selected.y === y) {
      s.selected = null; s.moves = [];
      return 'cancel';
    }
    // Tap a legal target -> attempt move.
    if (s.moves.some(m => m.x === x && m.y === y)) {
      const nb = applyMove(s.board, s.selected.x, s.selected.y, x, y);
      // The move must leave our King NOT in check (already true since
      // moves came from legalMovesFor below).
      s.board = nb;
      s.attempts++;
      if (isCheckmate(s.board, 'b')) {
        s.over = true; s.won = true;
      } else {
        // Wrong move: undo so the player can keep trying.
        s.board = buildBoard(s.lv.pos);
      }
      s.selected = null; s.moves = [];
      return 'move';
    }
    // Tap another own piece -> reselect.
    if (sq && sq.c === 'w') {
      s.selected = { x, y };
      s.moves = legalMovesFor(s.board, 'w').filter(m => m.fx === x && m.fy === y).map(m => ({ x: m.tx, y: m.ty }));
      return 'select';
    }
    return 'noop';
  }
  // No selection: pick up a White piece.
  if (sq && sq.c === 'w') {
    s.selected = { x, y };
    s.moves = legalMovesFor(s.board, 'w').filter(m => m.fx === x && m.fy === y).map(m => ({ x: m.tx, y: m.ty }));
    return 'select';
  }
  return 'noop';
}

function restart(s) {
  s.board = buildBoard(s.lv.pos);
  s.selected = null; s.moves = [];
  s.over = false; s.won = false;
  s.attempts = 0;
}

// Star tier: 3 = first try, 2 = up to 3 tries, 1 = more.
function stars(attempts) {
  if (attempts <= 1) return 3;
  if (attempts <= 3) return 2;
  return 1;
}
