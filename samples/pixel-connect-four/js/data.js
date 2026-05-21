// Pixel Connect Four - board rules and the minimax AI.

const VW = 360, VH = 480;
const COLS = 7, ROWS = 6;
const PLAYER = 1, AI = 2;

const DIFFICULTIES = [
  { name: ['Easy', '简单'],   depth: 2 },
  { name: ['Medium', '普通'], depth: 4 },
  { name: ['Hard', '困难'],   depth: 6 },
  { name: ['Expert', '专家'], depth: 8 },
];

// search columns centre-out so alpha-beta prunes hard
const COL_ORDER = [3, 2, 4, 1, 5, 0, 6];

function emptyBoard() {
  return Array.from({ length: ROWS }, () => new Array(COLS).fill(0));
}
// lowest empty row in a column, or -1 if full
function dropRow(board, col) {
  for (let r = ROWS - 1; r >= 0; r--) if (board[r][col] === 0) return r;
  return -1;
}
function boardFull(board) {
  for (let c = 0; c < COLS; c++) if (board[0][c] === 0) return false;
  return true;
}

// does placing `p` at (r,c) complete a line of four through that cell?
function winsAt(board, p, r, c) {
  for (const [dr, dc] of [[0, 1], [1, 0], [1, 1], [1, -1]]) {
    let count = 1;
    for (const s of [1, -1]) {
      let rr = r + dr * s, cc = c + dc * s;
      while (rr >= 0 && cc >= 0 && rr < ROWS && cc < COLS && board[rr][cc] === p) {
        count++; rr += dr * s; cc += dc * s;
      }
    }
    if (count >= 4) return true;
  }
  return false;
}
// full-board win scan — used to highlight a finished game
function findWin(board, p) {
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      if (board[r][c] !== p) continue;
      for (const [dr, dc] of [[0, 1], [1, 0], [1, 1], [1, -1]]) {
        const line = [[r, c]];
        for (let k = 1; k < 4; k++) {
          const rr = r + dr * k, cc = c + dc * k;
          if (rr < 0 || cc < 0 || rr >= ROWS || cc >= COLS || board[rr][cc] !== p) break;
          line.push([rr, cc]);
        }
        if (line.length === 4) return line;
      }
    }
  }
  return null;
}

// heuristic: positive favours the AI
function scoreWindow(a, b, c, d) {
  let ai = 0, pl = 0;
  for (const v of [a, b, c, d]) { if (v === AI) ai++; else if (v === PLAYER) pl++; }
  if (ai && pl) return 0;
  if (ai === 4) return 100000;
  if (ai === 3) return 60;
  if (ai === 2) return 8;
  if (pl === 4) return -100000;
  if (pl === 3) return -75;
  if (pl === 2) return -8;
  return 0;
}
function evaluate(board) {
  let s = 0;
  for (let r = 0; r < ROWS; r++) s += board[r][3] === AI ? 6 : board[r][3] === PLAYER ? -6 : 0;
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      if (c + 3 < COLS) s += scoreWindow(board[r][c], board[r][c + 1], board[r][c + 2], board[r][c + 3]);
      if (r + 3 < ROWS) s += scoreWindow(board[r][c], board[r + 1][c], board[r + 2][c], board[r + 3][c]);
      if (r + 3 < ROWS && c + 3 < COLS)
        s += scoreWindow(board[r][c], board[r + 1][c + 1], board[r + 2][c + 2], board[r + 3][c + 3]);
      if (r + 3 < ROWS && c - 3 >= 0)
        s += scoreWindow(board[r][c], board[r + 1][c - 1], board[r + 2][c - 2], board[r + 3][c - 3]);
    }
  }
  return s;
}

function minimax(board, depth, alpha, beta, toMove) {
  if (depth === 0) return evaluate(board);
  const cols = COL_ORDER.filter(c => dropRow(board, c) >= 0);
  if (!cols.length) return 0;
  if (toMove === AI) {
    let best = -Infinity;
    for (const c of cols) {
      const r = dropRow(board, c);
      board[r][c] = AI;
      const s = winsAt(board, AI, r, c) ? 100000 + depth
        : minimax(board, depth - 1, alpha, beta, PLAYER);
      board[r][c] = 0;
      best = Math.max(best, s);
      alpha = Math.max(alpha, s);
      if (alpha >= beta) break;
    }
    return best;
  }
  let best = Infinity;
  for (const c of cols) {
    const r = dropRow(board, c);
    board[r][c] = PLAYER;
    const s = winsAt(board, PLAYER, r, c) ? -100000 - depth
      : minimax(board, depth - 1, alpha, beta, AI);
    board[r][c] = 0;
    best = Math.min(best, s);
    beta = Math.min(beta, s);
    if (alpha >= beta) break;
  }
  return best;
}

// choose the AI's column (random tie-break among equally-good moves)
function aiMove(board, depth) {
  let bestScore = -Infinity;
  let bestCols = [];
  for (const c of COL_ORDER) {
    const r = dropRow(board, c);
    if (r < 0) continue;
    board[r][c] = AI;
    const s = winsAt(board, AI, r, c) ? 1000000
      : minimax(board, depth - 1, -Infinity, Infinity, PLAYER);
    board[r][c] = 0;
    if (s > bestScore) { bestScore = s; bestCols = [c]; }
    else if (s === bestScore) bestCols.push(c);
  }
  return bestCols[(Math.random() * bestCols.length) | 0];
}
