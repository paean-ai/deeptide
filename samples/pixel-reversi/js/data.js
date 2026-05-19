// Pixel Reversi - Othello board rules and the minimax AI.

const VW = 360, VH = 480;
const N = 8;
const EMPTY = 0, DARK = 1, LIGHT = 2;   // player is DARK, AI is LIGHT

const DIFFICULTIES = [
  { name: ['Easy', '简单'],   depth: 1 },
  { name: ['Medium', '普通'], depth: 3 },
  { name: ['Hard', '困难'],   depth: 4 },
];

const DIRS = [[-1, -1], [-1, 0], [-1, 1], [0, -1], [0, 1], [1, -1], [1, 0], [1, 1]];

// classic positional weights — corners prized, the cells beside them poison
const WEIGHTS = [
  [120, -20, 20, 5, 5, 20, -20, 120],
  [-20, -40, -5, -5, -5, -5, -40, -20],
  [20, -5, 15, 3, 3, 15, -5, 20],
  [5, -5, 3, 3, 3, 3, -5, 5],
  [5, -5, 3, 3, 3, 3, -5, 5],
  [20, -5, 15, 3, 3, 15, -5, 20],
  [-20, -40, -5, -5, -5, -5, -40, -20],
  [120, -20, 20, 5, 5, 20, -20, 120],
];

function initialBoard() {
  const b = Array.from({ length: N }, () => new Array(N).fill(EMPTY));
  b[3][3] = LIGHT; b[4][4] = LIGHT;
  b[3][4] = DARK; b[4][3] = DARK;
  return b;
}
function opponent(p) { return p === DARK ? LIGHT : DARK; }

// cells that flip if player p drops a disc at (r,c); empty array = illegal.
function flipsFor(board, p, r, c) {
  if (board[r][c] !== EMPTY) return [];
  const opp = opponent(p);
  const flips = [];
  for (const [dr, dc] of DIRS) {
    const line = [];
    let rr = r + dr, cc = c + dc;
    while (rr >= 0 && cc >= 0 && rr < N && cc < N && board[rr][cc] === opp) {
      line.push([rr, cc]);
      rr += dr; cc += dc;
    }
    if (line.length && rr >= 0 && cc >= 0 && rr < N && cc < N && board[rr][cc] === p) {
      for (const cell of line) flips.push(cell);
    }
  }
  return flips;
}
function legalMoves(board, p) {
  const moves = [];
  for (let r = 0; r < N; r++) {
    for (let c = 0; c < N; c++) {
      if (board[r][c] !== EMPTY) continue;
      const f = flipsFor(board, p, r, c);
      if (f.length) moves.push({ r, c, flips: f });
    }
  }
  return moves;
}
function applyMove(board, p, r, c) {
  const nb = board.map(row => row.slice());
  const flips = flipsFor(board, p, r, c);
  nb[r][c] = p;
  for (const [fr, fc] of flips) nb[fr][fc] = p;
  return nb;
}
function countDiscs(board) {
  let dark = 0, light = 0;
  for (let r = 0; r < N; r++) {
    for (let c = 0; c < N; c++) {
      if (board[r][c] === DARK) dark++;
      else if (board[r][c] === LIGHT) light++;
    }
  }
  return { dark, light };
}

// evaluation from the AI's (LIGHT) point of view
function evaluate(board) {
  let pos = 0;
  for (let r = 0; r < N; r++) {
    for (let c = 0; c < N; c++) {
      if (board[r][c] === LIGHT) pos += WEIGHTS[r][c];
      else if (board[r][c] === DARK) pos -= WEIGHTS[r][c];
    }
  }
  const mob = legalMoves(board, LIGHT).length - legalMoves(board, DARK).length;
  return pos + mob * 10;
}

function minimax(board, depth, alpha, beta, toMove) {
  const moves = legalMoves(board, toMove);
  if (!moves.length) {
    const other = legalMoves(board, opponent(toMove));
    if (!other.length) {
      const { dark, light } = countDiscs(board);
      return (light - dark) * 10000;
    }
    return minimax(board, depth - 1, alpha, beta, opponent(toMove));
  }
  if (depth === 0) return evaluate(board);
  if (toMove === LIGHT) {
    let best = -Infinity;
    for (const m of moves) {
      const s = minimax(applyMove(board, LIGHT, m.r, m.c), depth - 1, alpha, beta, DARK);
      best = Math.max(best, s);
      alpha = Math.max(alpha, s);
      if (alpha >= beta) break;
    }
    return best;
  }
  let best = Infinity;
  for (const m of moves) {
    const s = minimax(applyMove(board, DARK, m.r, m.c), depth - 1, alpha, beta, LIGHT);
    best = Math.min(best, s);
    beta = Math.min(beta, s);
    if (alpha >= beta) break;
  }
  return best;
}

// pick the AI's move (random tie-break among equal-scored moves)
function aiMove(board, depth) {
  const moves = legalMoves(board, LIGHT);
  if (!moves.length) return null;
  let bestScore = -Infinity, best = [];
  for (const m of moves) {
    const s = minimax(applyMove(board, LIGHT, m.r, m.c), depth - 1, -Infinity, Infinity, DARK);
    if (s > bestScore) { bestScore = s; best = [m]; }
    else if (s === bestScore) best.push(m);
  }
  return best[(Math.random() * best.length) | 0];
}
