const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');

const N = 8;
const TILE = 80;
const GEMS = [
  { name: 'Aqua', color: '#67d8ff', dark: '#246a9a' },
  { name: 'Leaf', color: '#72df89', dark: '#2f7c4a' },
  { name: 'Sun', color: '#f4c85a', dark: '#9a6e20' },
  { name: 'Ruby', color: '#ff7d7d', dark: '#9b363d' },
  { name: 'Violet', color: '#b7a7ff', dark: '#5c52a0' },
  { name: 'Bloom', color: '#ff9cd6', dark: '#a94778' },
];
const SPECIALS = {
  row: { label: 'H', color: '#fff1a6' },
  col: { label: 'V', color: '#fff1a6' },
  bomb: { label: 'B', color: '#ff8f6b' },
  prism: { label: '*', color: '#ffffff' },
};

const state = {
  board: [],
  selected: null,
  score: 0,
  moves: 0,
  combo: 1,
  level: 1,
  busy: false,
  particles: [],
  flashes: [],
  target: null,
  message: '',
  messageTimer: 0,
};

function randGem() {
  return Math.floor(Math.random() * GEMS.length);
}

function cell(type = randGem(), special = null) {
  return { type, special };
}

function levelMoves() {
  return Math.max(18, 30 - Math.floor(state.level / 3));
}

function makeTarget() {
  return {
    type: state.level % GEMS.length,
    needed: 16 + state.level * 4,
    collected: 0,
  };
}

function makeBoard() {
  state.board = Array.from({ length: N }, () => Array(N).fill(null));
  for (let y = 0; y < N; y++) {
    for (let x = 0; x < N; x++) {
      do {
        state.board[y][x] = cell();
      } while (
        (x >= 2 && state.board[y][x].type === state.board[y][x - 1].type && state.board[y][x].type === state.board[y][x - 2].type) ||
        (y >= 2 && state.board[y][x].type === state.board[y - 1][x].type && state.board[y][x].type === state.board[y - 2][x].type)
      );
    }
  }
}

function restartLevel(resetScore = false) {
  if (resetScore) {
    state.score = 0;
    state.level = 1;
  }
  state.moves = levelMoves();
  state.combo = 1;
  state.selected = null;
  state.target = makeTarget();
  state.message = `LEVEL ${state.level}`;
  state.messageTimer = 90;
  makeBoard();
  ensurePlayable();
  updateHud();
}

function findGroups() {
  const groups = [];
  for (let y = 0; y < N; y++) {
    let run = [0];
    for (let x = 1; x <= N; x++) {
      if (x < N && state.board[y][x] && state.board[y][x - 1] && state.board[y][x].type === state.board[y][x - 1].type) run.push(x);
      else {
        if (run.length >= 3) groups.push({ cells: run.map(rx => ({ x: rx, y })), dir: 'row' });
        run = [x];
      }
    }
  }
  for (let x = 0; x < N; x++) {
    let run = [0];
    for (let y = 1; y <= N; y++) {
      if (y < N && state.board[y][x] && state.board[y - 1][x] && state.board[y][x].type === state.board[y - 1][x].type) run.push(y);
      else {
        if (run.length >= 3) groups.push({ cells: run.map(ry => ({ x, y: ry })), dir: 'col' });
        run = [y];
      }
    }
  }
  return groups;
}

function uniquePositions(groups) {
  const map = new Map();
  for (const g of groups) for (const p of g.cells) map.set(`${p.x},${p.y}`, p);
  return [...map.values()];
}

function pickSpecial(groups, origin) {
  const originGroups = groups.filter(g => g.cells.some(p => p.x === origin.x && p.y === origin.y));
  if (!originGroups.length) return null;
  if (originGroups.length > 1) return 'bomb';
  const g = originGroups[0];
  if (g.cells.length >= 5) return 'prism';
  if (g.cells.length >= 4) return g.dir === 'row' ? 'row' : 'col';
  return null;
}

function expandSpecials(marks) {
  const queue = [...marks];
  const key = p => `${p.x},${p.y}`;
  const seen = new Set(marks.map(key));
  for (let i = 0; i < queue.length; i++) {
    const p = queue[i];
    const c = state.board[p.y][p.x];
    if (!c || !c.special) continue;
    const add = [];
    if (c.special === 'row') for (let x = 0; x < N; x++) add.push({ x, y: p.y });
    if (c.special === 'col') for (let y = 0; y < N; y++) add.push({ x: p.x, y });
    if (c.special === 'bomb') {
      for (let yy = p.y - 1; yy <= p.y + 1; yy++) for (let xx = p.x - 1; xx <= p.x + 1; xx++) {
        if (xx >= 0 && xx < N && yy >= 0 && yy < N) add.push({ x: xx, y: yy });
      }
    }
    if (c.special === 'prism') {
      const type = c.type;
      for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
        if (state.board[y][x] && state.board[y][x].type === type) add.push({ x, y });
      }
    }
    for (const n of add) if (!seen.has(key(n))) {
      seen.add(key(n));
      queue.push(n);
    }
  }
  return queue;
}

function collapse() {
  for (let x = 0; x < N; x++) {
    const col = [];
    for (let y = N - 1; y >= 0; y--) if (state.board[y][x]) col.push(state.board[y][x]);
    while (col.length < N) col.push(cell());
    for (let y = N - 1; y >= 0; y--) state.board[y][x] = col[N - 1 - y];
  }
}

async function resolveMatches(origin = null) {
  state.busy = true;
  let combo = 1;
  while (true) {
    const groups = findGroups();
    const directSpecial = !groups.length && origin && state.board[origin.y][origin.x]?.special;
    if (!groups.length && !directSpecial) break;
    let marks = directSpecial ? [origin] : expandSpecials(uniquePositions(groups));
    let created = null;
    if (origin) {
      const special = pickSpecial(groups, origin);
      if (special) created = { ...origin, special, type: state.board[origin.y][origin.x]?.type ?? randGem() };
    }

    for (const p of marks) {
      const c = state.board[p.y][p.x];
      if (!c) continue;
      if (created && p.x === created.x && p.y === created.y) continue;
      if (c.type === state.target.type) state.target.collected++;
      burst(p.x, p.y, GEMS[c.type].color, c.special ? 12 : 7);
      state.board[p.y][p.x] = null;
    }
    if (created) {
      state.board[created.y][created.x] = cell(created.type, created.special);
      state.flashes.push({ x: created.x, y: created.y, life: 24, color: SPECIALS[created.special].color });
    }
    state.score += marks.length * 15 * combo + Math.max(0, combo - 1) * 80;
    state.combo = combo;
    updateHud();
    await delay(170);
    collapse();
    origin = null;
    combo++;
    await delay(100);
  }
  state.combo = 1;
  state.busy = false;
  if (state.target.collected >= state.target.needed) {
    state.level++;
    state.score += 500 + state.moves * 25;
    restartLevel(false);
  } else if (state.moves <= 0) {
    state.message = 'OUT OF MOVES';
    state.messageTimer = 180;
  } else {
    ensurePlayable();
  }
  updateHud();
}

function delay(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function swap(a, b) {
  const tmp = state.board[a.y][a.x];
  state.board[a.y][a.x] = state.board[b.y][b.x];
  state.board[b.y][b.x] = tmp;
}

async function clickCell(x, y) {
  if (state.busy || state.moves <= 0) return;
  if (!state.selected) {
    state.selected = { x, y };
  } else {
    const a = state.selected;
    const b = { x, y };
    const adjacent = Math.abs(a.x - b.x) + Math.abs(a.y - b.y) === 1;
    if (adjacent) {
      swap(a, b);
      state.selected = null;
      if (findGroups().length || state.board[b.y][b.x].special || state.board[a.y][a.x].special) {
        state.moves--;
        await resolveMatches(b);
      } else {
        swap(a, b);
        state.message = 'NO MATCH';
        state.messageTimer = 45;
      }
    } else {
      state.selected = { x, y };
    }
  }
  updateHud();
}

function hasMove() {
  for (let y = 0; y < N; y++) {
    for (let x = 0; x < N; x++) {
      const a = { x, y };
      for (const b of [{ x: x + 1, y }, { x, y: y + 1 }]) {
        if (b.x >= N || b.y >= N) continue;
        swap(a, b);
        const ok = findGroups().length > 0;
        swap(a, b);
        if (ok) return true;
      }
    }
  }
  return false;
}

function ensurePlayable() {
  let guard = 0;
  while (!hasMove() && guard++ < 20) makeBoard();
}

function burst(x, y, color, count) {
  for (let i = 0; i < count; i++) {
    state.particles.push({
      x: x * TILE + TILE / 2,
      y: y * TILE + TILE / 2,
      vx: (Math.random() - 0.5) * 5,
      vy: (Math.random() - 0.5) * 5,
      life: 24 + Math.random() * 16,
      color,
    });
  }
}

function drawGem(x, y, c) {
  const px = x * TILE;
  const py = y * TILE;
  ctx.fillStyle = (x + y) % 2 ? '#172237' : '#141d31';
  ctx.fillRect(px + 3, py + 3, TILE - 6, TILE - 6);
  ctx.fillStyle = '#243552';
  ctx.fillRect(px + 8, py + 8, TILE - 16, TILE - 16);
  if (!c) return;

  const g = GEMS[c.type];
  ctx.fillStyle = g.dark;
  ctx.fillRect(px + 22, py + 16, 36, 48);
  ctx.fillStyle = g.color;
  ctx.beginPath();
  ctx.moveTo(px + 40, py + 11);
  ctx.lineTo(px + 64, py + 31);
  ctx.lineTo(px + 54, py + 63);
  ctx.lineTo(px + 26, py + 63);
  ctx.lineTo(px + 16, py + 31);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = 'rgba(255,255,255,0.42)';
  ctx.fillRect(px + 29, py + 22, 18, 7);
  ctx.fillRect(px + 24, py + 32, 8, 5);
  if (c.special) {
    ctx.fillStyle = SPECIALS[c.special].color;
    ctx.fillRect(px + 26, py + 50, 28, 14);
    ctx.fillStyle = '#111827';
    ctx.font = '900 13px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(SPECIALS[c.special].label, px + 40, py + 62);
  }
}

function draw() {
  ctx.fillStyle = '#0f1522';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = '#243552';
  for (let i = 0; i <= N; i++) {
    ctx.fillRect(i * TILE - 1, 0, 2, canvas.height);
    ctx.fillRect(0, i * TILE - 1, canvas.width, 2);
  }
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) drawGem(x, y, state.board[y][x]);

  if (state.selected) {
    ctx.strokeStyle = '#f4c85a';
    ctx.lineWidth = 5;
    ctx.strokeRect(state.selected.x * TILE + 7, state.selected.y * TILE + 7, TILE - 14, TILE - 14);
  }

  for (const f of state.flashes) {
    ctx.globalAlpha = Math.max(0, f.life / 24);
    ctx.strokeStyle = f.color;
    ctx.lineWidth = 4;
    ctx.strokeRect(f.x * TILE + 10 - (24 - f.life), f.y * TILE + 10 - (24 - f.life), 60 + (24 - f.life) * 2, 60 + (24 - f.life) * 2);
    ctx.globalAlpha = 1;
  }
  for (const p of state.particles) {
    ctx.globalAlpha = Math.max(0, p.life / 30);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x, p.y, 5, 5);
    ctx.globalAlpha = 1;
  }

  if (state.messageTimer > 0) {
    ctx.fillStyle = 'rgba(8, 12, 19, 0.62)';
    ctx.fillRect(0, 260, canvas.width, 118);
    ctx.fillStyle = state.moves <= 0 && state.message === 'OUT OF MOVES' ? '#ff7d7d' : '#f4c85a';
    ctx.font = '900 42px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(state.message, canvas.width / 2, 333);
  }
}

function tick() {
  for (const p of state.particles) {
    p.x += p.vx;
    p.y += p.vy;
    p.life--;
  }
  state.particles = state.particles.filter(p => p.life > 0);
  for (const f of state.flashes) f.life--;
  state.flashes = state.flashes.filter(f => f.life > 0);
  if (state.messageTimer > 0) state.messageTimer--;
  draw();
  requestAnimationFrame(tick);
}

function updateHud() {
  document.getElementById('level').textContent = state.level;
  document.getElementById('score').textContent = state.score;
  document.getElementById('moves').textContent = state.moves;
  document.getElementById('combo').textContent = `x${state.combo}`;
  const g = GEMS[state.target?.type || 0];
  document.getElementById('target').textContent = `${g.name} ${Math.min(state.target.collected, state.target.needed)}/${state.target.needed}`;
}

function boardPos(e) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: Math.floor(((e.clientX - rect.left) / rect.width) * N),
    y: Math.floor(((e.clientY - rect.top) / rect.height) * N),
  };
}

canvas.addEventListener('pointerdown', e => {
  const p = boardPos(e);
  if (p.x >= 0 && p.x < N && p.y >= 0 && p.y < N) clickCell(p.x, p.y);
});

document.getElementById('shuffle').onclick = () => {
  if (state.busy || state.moves <= 0) return;
  makeBoard();
  state.moves = Math.max(0, state.moves - 1);
  state.message = 'SHUFFLE';
  state.messageTimer = 45;
  updateHud();
};
document.getElementById('restart').onclick = () => restartLevel(true);

restartLevel(true);
tick();
