// Pixel Arcade - shell + five mini-games
(() => {
'use strict';

const SAVE_KEY = 'pixel-arcade-save';
const $ = id => document.getElementById(id);
const canvas = $('arcade-canvas');
const ctx = canvas.getContext('2d');
const W = 480, H = 720;
canvas.width = W; canvas.height = H;
ctx.imageSmoothingEnabled = false;

const GROUND = H - 70;
const GAME_IDS = ['flap', 'catch', 'reflex', 'stack', 'dash', 'squash', 'memory'];
const MEDALS = {
  flap:   [6, 15, 28],
  catch:  [14, 32, 55],
  reflex: [14, 28, 46],
  stack:  [7, 15, 26],
  dash:   [260, 620, 1150],
  squash: [10, 22, 40],
  memory: [4, 9, 14],
};

// ---- save --------------------------------------------------------------
function loadSave() {
  try { return JSON.parse(localStorage.getItem(SAVE_KEY)) || { best: {} }; }
  catch (e) { return { best: {} }; }
}
let save = loadSave();
function persist() { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); }
function medalOf(id, score) {
  const m = MEDALS[id];
  return score >= m[2] ? 3 : score >= m[1] ? 2 : score >= m[0] ? 1 : 0;
}
const MEDAL_ICON = ['', '🥉', '🥈', '🥇'];

// ---- shell state -------------------------------------------------------
let G = null;
let rafId = 0, lastT = 0;
let paused = false;
let particles = [];

function showScreen(id) {
  ['hub', 'play'].forEach(s => $('screen-' + s).classList.toggle('hidden', s !== id));
}

function startGame(id) {
  G = { id, s: {}, score: 0, lives: -1, over: false, started: false, time: 0, shake: 0 };
  particles = [];
  paused = false;
  GAMES[id].init(G);
  $('overlay-over').classList.add('hidden');
  $('overlay-pause').classList.add('hidden');
  showScreen('play');
  resize();
}

function endGame() {
  if (G.over) return;
  G.over = true;
  G.shake = 0.4;
  const prev = save.best[G.id] || 0;
  const isBest = G.score > prev;
  if (isBest) { save.best[G.id] = G.score; persist(); }
  setTimeout(() => {
    $('over-title').textContent = isBest ? t('newBest') : t('gameOver');
    $('over-title').className = isBest ? 'win' : 'lose';
    $('over-score').textContent = G.score + ' ' + gameUnit(G.id);
    $('over-best').textContent = t('best') + ': ' + (save.best[G.id] || 0);
    const med = medalOf(G.id, G.score);
    $('over-medal').textContent = med ? MEDAL_ICON[med] : '';
    $('overlay-over').classList.remove('hidden');
  }, 700);
}

// ---- particles ---------------------------------------------------------
function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * 6.28, sp = 50 + Math.random() * 170;
    particles.push({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 40,
      life: 0.5, max: 0.5, size: 3 + Math.random() * 4, color });
  }
}

// ========================================================================
// GAME 1: SKY FLAP
// ========================================================================
const GAMES = {};
GAMES.flap = {
  init(G) {
    G.s = { y: H * 0.42, vy: 0, pipes: [], spawnT: 0.6, gapH: 190 };
  },
  flap(G) { G.s.vy = -430; burst(110, G.s.y + 16, 4, '#fff'); },
  down(G) { if (!G.over) this.flap(G); },
  update(G, dt) {
    const s = G.s;
    s.vy += 1500 * dt;
    s.y += s.vy * dt;
    s.spawnT -= dt;
    if (s.spawnT <= 0) {
      const gap = s.gapH;
      const gy = 90 + Math.random() * (GROUND - 180 - gap);
      s.pipes.push({ x: W + 40, gapY: gy, passed: false });
      s.spawnT = 1.65 - Math.min(0.6, G.score * 0.03);
      s.gapH = Math.max(140, 190 - G.score * 2);
    }
    const speed = 175 + G.score * 3;
    for (const p of s.pipes) {
      p.x -= speed * dt;
      if (!p.passed && p.x + 64 < 110) { p.passed = true; G.score++; }
    }
    s.pipes = s.pipes.filter(p => p.x > -80);
    // collision
    const bx = 110;
    if (s.y - 11 < 0) { s.y = 11; s.vy = 0; }
    if (s.y + 11 > GROUND) { endGame(); }
    for (const p of s.pipes) {
      if (bx + 11 > p.x && bx - 11 < p.x + 64) {
        if (s.y - 10 < p.gapY || s.y + 10 > p.gapY + s.gapH) { endGame(); }
      }
    }
  },
  render(G, ctx) {
    skyGradient(ctx, W, H, '#5fb8e8', '#bfe8f5');
    drawCloud(ctx, (W - (G.time * 18) % (W + 80)), 110, 2);
    drawCloud(ctx, (W - (G.time * 12 + 220) % (W + 120)), 220, 2.6);
    for (const p of G.s.pipes) drawPipe(ctx, p.x, p.gapY, G.s.gapH, 64, GROUND);
    drawGround(ctx, GROUND, W, H, G.time, '#caa14a');
    drawBird(ctx, 110, G.s.y, G.time, G.s.vy);
  },
};

// ========================================================================
// GAME 2: FRUIT CATCH
// ========================================================================
GAMES.catch = {
  init(G) {
    G.lives = 3;
    G.s = { bx: W / 2, tx: W / 2, items: [], spawnT: 0.8, bw: 92 };
  },
  down(G, x) { G.s.tx = x; },
  move(G, x) { G.s.tx = x; },
  update(G, dt) {
    const s = G.s;
    s.bx += (s.tx - s.bx) * Math.min(1, dt * 14);
    s.bx = Math.max(s.bw / 2, Math.min(W - s.bw / 2, s.bx));
    s.spawnT -= dt;
    if (s.spawnT <= 0) {
      const bomb = Math.random() < 0.26;
      s.items.push({ x: 40 + Math.random() * (W - 80), y: -20,
        vy: 150 + Math.random() * 60 + G.time * 5,
        kind: bomb ? 'bomb' : 'fruit', sub: (Math.random() * 4) | 0 });
      s.spawnT = Math.max(0.42, 0.95 - G.time * 0.02);
    }
    const by = GROUND - 90;
    for (const it of s.items) {
      it.y += it.vy * dt;
      if (it.y > by - 6 && it.y < by + 26 && Math.abs(it.x - s.bx) < s.bw / 2) {
        it.gone = true;
        if (it.kind === 'fruit') { G.score++; burst(it.x, it.y, 8, '#5fd06a'); }
        else { G.lives--; G.shake = 0.35; burst(it.x, it.y, 14, '#ff5a3a'); }
      } else if (it.y > H + 20) {
        it.gone = true;
        if (it.kind === 'fruit') { G.lives--; G.shake = 0.25; }
      }
    }
    s.items = s.items.filter(i => !i.gone);
    if (G.lives <= 0) endGame();
  },
  render(G, ctx) {
    skyGradient(ctx, W, H, '#8a6fc0', '#d8c4ec');
    drawGround(ctx, GROUND, W, H, G.time, '#4a7a3a');
    for (const it of G.s.items) {
      if (it.kind === 'fruit') drawFruit(ctx, it.x, it.y, it.sub, G.time);
      else drawBomb(ctx, it.x, it.y, G.time);
    }
    drawBasket(ctx, G.s.bx, GROUND - 90, G.s.bw);
  },
};

// ========================================================================
// GAME 3: REFLEX TAP
// ========================================================================
GAMES.reflex = {
  init(G) {
    G.lives = 3;
    G.s = { targets: [], spawnT: 0.5, spawnInt: 1.15 };
  },
  down(G, x, y) {
    let hit = null, hd = 1e9;
    for (const tg of G.s.targets) {
      const d = Math.hypot(tg.x - x, tg.y - y);
      if (d < tg.r + 8 && d < hd) { hd = d; hit = tg; }
    }
    if (hit) {
      hit.gone = true; G.score++;
      burst(hit.x, hit.y, 12, '#ffd34d');
    }
  },
  update(G, dt) {
    const s = G.s;
    s.spawnT -= dt;
    if (s.spawnT <= 0) {
      const r = 30 + Math.random() * 14;
      s.targets.push({
        x: 50 + Math.random() * (W - 100),
        y: 130 + Math.random() * (GROUND - 200),
        r, ttl: 1, life: Math.max(0.7, 1.6 - G.time * 0.03),
      });
      s.spawnInt = Math.max(0.42, 1.15 - G.time * 0.02);
      s.spawnT = s.spawnInt;
    }
    for (const tg of s.targets) {
      tg.ttl -= dt / tg.life;
      if (tg.ttl <= 0 && !tg.gone) { tg.gone = true; G.lives--; G.shake = 0.3; }
    }
    s.targets = s.targets.filter(tg => !tg.gone);
    if (G.lives <= 0) endGame();
  },
  render(G, ctx) {
    skyGradient(ctx, W, H, '#2a3a5a', '#16203a');
    for (let i = 0; i < 30; i++) {
      const sx = (i * 137) % W, sy = (i * 89) % GROUND;
      pr(ctx, sx, sy, 2, 2, 'rgba(255,255,255,0.3)');
    }
    for (const tg of G.s.targets) drawTarget(ctx, tg.x, tg.y, tg.r, Math.max(0, tg.ttl));
    drawGround(ctx, GROUND, W, H, G.time, '#2a2f4a');
  },
};

// ========================================================================
// GAME 4: TOWER STACK
// ========================================================================
GAMES.stack = {
  init(G) {
    const baseW = 180;
    G.s = {
      blocks: [{ x: W / 2 - baseW / 2, w: baseW, y: GROUND - 40 }],
      cur: { x: 0, w: baseW, dir: 1, y: GROUND - 40 - 40 },
      speed: 220, camY: 0, hue: 200,
    };
  },
  down(G) {
    if (G.over) return;
    const s = G.s;
    const top = s.blocks[s.blocks.length - 1];
    const cur = s.cur;
    const left = Math.max(cur.x, top.x);
    const right = Math.min(cur.x + cur.w, top.x + top.w);
    const overlap = right - left;
    if (overlap <= 0) { endGame(); return; }
    // trim particles
    if (cur.x < top.x) burst(cur.x + 6, cur.y + 20, 10, `hsl(${s.hue},58%,58%)`);
    if (cur.x + cur.w > top.x + top.w) burst(cur.x + cur.w - 6, cur.y + 20, 10, `hsl(${s.hue},58%,58%)`);
    s.blocks.push({ x: left, w: overlap, y: cur.y });
    G.score++;
    s.hue = (s.hue + 28) % 360;
    s.speed = Math.min(430, s.speed + 12);
    const newY = cur.y - 40;
    const fromLeft = Math.random() < 0.5;
    s.cur = { x: fromLeft ? -overlap : W, w: overlap, dir: fromLeft ? 1 : -1, y: newY };
    // scroll camera so the top stays around 1/3 height
    const targetCam = Math.max(0, (GROUND - 40 - newY) - H * 0.42);
    s.camYTarget = targetCam;
  },
  update(G, dt) {
    const s = G.s;
    s.cur.x += s.cur.dir * s.speed * dt;
    if (s.cur.x < -s.cur.w) s.cur.x = -s.cur.w, s.cur.dir = 1;
    if (s.cur.x > W) s.cur.x = W, s.cur.dir = -1;
    s.camY += ((s.camYTarget || 0) - s.camY) * Math.min(1, dt * 6);
  },
  render(G, ctx) {
    skyGradient(ctx, W, H, '#1d2b4a', '#3a5a8a');
    const s = G.s, oy = s.camY;
    for (let i = 0; i < s.blocks.length; i++) {
      const b = s.blocks[i];
      drawBlock(ctx, b.x, b.y + oy, b.w, 40, (200 + i * 28) % 360);
    }
    drawBlock(ctx, s.cur.x, s.cur.y + oy, s.cur.w, 40, s.hue);
    drawGround(ctx, GROUND, W, H, G.time, '#2a2f4a');
  },
};

// ========================================================================
// GAME 5: PIXEL DASH
// ========================================================================
GAMES.dash = {
  init(G) {
    G.s = {
      y: GROUND - 28, vy: 0, onGround: true, jumps: 0,
      obs: [], spawnT: 1.0, speed: 280, dist: 0,
    };
  },
  down(G) {
    const s = G.s;
    if (G.over) return;
    if (s.onGround || s.jumps < 2) {
      s.vy = s.onGround ? -620 : -540;
      s.onGround = false;
      s.jumps++;
      burst(110, s.y + 10, 5, '#fff');
    }
  },
  update(G, dt) {
    const s = G.s;
    s.vy += 1900 * dt;
    s.y += s.vy * dt;
    if (s.y >= GROUND - 28) { s.y = GROUND - 28; s.vy = 0; s.onGround = true; s.jumps = 0; }
    s.speed = 280 + G.time * 9;
    s.dist += s.speed * dt;
    G.score = Math.floor(s.dist / 10);
    s.spawnT -= dt;
    if (s.spawnT <= 0) {
      const tall = Math.random() < 0.4;
      s.obs.push({ x: W + 30, type: tall ? 'crate' : 'spike',
        w: tall ? 34 : 30 + Math.random() * 28, h: tall ? 46 : 22 });
      s.spawnT = Math.max(0.5, (0.95 + Math.random() * 0.6) - G.time * 0.012);
    }
    for (const o of s.obs) o.x -= s.speed * dt;
    s.obs = s.obs.filter(o => o.x > -60);
    // collision: runner box ~ (110±13, y-24 .. y+18)
    for (const o of s.obs) {
      const oy = GROUND - o.h;
      if (110 + 11 > o.x && 110 - 11 < o.x + o.w && s.y + 18 > oy) { endGame(); }
    }
  },
  render(G, ctx) {
    skyGradient(ctx, W, H, '#e88a4a', '#f5d8a8');
    drawCloud(ctx, (W - (G.time * 22) % (W + 80)), 120, 2.4);
    for (const o of G.s.obs) {
      if (o.type === 'spike') drawSpike(ctx, o.x, GROUND, o.w);
      else drawCrateObs(ctx, o.x, GROUND - o.h, o.w, o.h);
    }
    drawGround(ctx, GROUND, W, H, G.time, '#4a3a2a');
    drawRunner(ctx, 110, G.s.y, G.time, !G.s.onGround);
  },
};

// ========================================================================
// GAME 6: BUG SQUASH
// ========================================================================
GAMES.squash = {
  init(G) {
    const holes = [];
    for (let r = 0; r < 3; r++) for (let c = 0; c < 3; c++) {
      holes.push({ cx: 110 + c * 130, cy: 250 + r * 168, occ: null });
    }
    G.s = { holes, spawnT: 0.7, escapes: 0 };
  },
  update(G, dt) {
    const s = G.s;
    s.spawnT -= dt;
    if (s.spawnT <= 0) {
      const empty = s.holes.filter(h => !h.occ);
      if (empty.length) {
        const h = empty[(Math.random() * empty.length) | 0];
        const bomb = Math.random() < 0.15 + Math.min(0.13, G.score * 0.004);
        const stay = Math.max(0.5, 1.3 - G.score * 0.02);
        h.occ = { type: bomb ? 'bomb' : 'bug', life: stay, maxLife: stay, age: 0 };
      }
      s.spawnT = Math.max(0.3, 0.95 - G.score * 0.014) * (0.6 + Math.random() * 0.8);
    }
    for (const h of s.holes) {
      if (!h.occ) continue;
      h.occ.age += dt;
      h.occ.life -= dt;
      if (h.occ.life <= 0) {
        if (h.occ.type === 'bug') s.escapes++;
        h.occ = null;
      }
    }
    if (s.escapes >= 6) endGame();
  },
  down(G, x, y) {
    for (const h of G.s.holes) {
      if (!h.occ) continue;
      if (Math.abs(x - h.cx) < 56 && Math.abs(y - (h.cy - 8)) < 58) {
        if (h.occ.type === 'bomb') {
          G.shake = 0.5; burst(h.cx, h.cy - 10, 22, '#ff5a5a'); endGame(); return;
        }
        G.score++;
        burst(h.cx, h.cy - 10, 12, '#7dff9f');
        h.occ = null;
        return;
      }
    }
  },
  render(G, ctx) {
    skyGradient(ctx, W, H, '#2a3a2a', '#16241a');
    ctx.fillStyle = '#3a5a32';
    ctx.fillRect(0, GROUND, W, H - GROUND);
    for (let i = 0; i < 6; i++) {
      ctx.fillStyle = i < G.s.escapes ? '#ff5a5a' : '#2c3c2c';
      ctx.fillRect(W - 30 - i * 22, 64, 16, 10);
    }
    for (const h of G.s.holes) {
      ctx.fillStyle = '#10180f';
      ctx.beginPath();
      ctx.ellipse(h.cx, h.cy + 14, 58, 26, 0, 0, 6.283);
      ctx.fill();
      ctx.fillStyle = '#241b14';
      ctx.beginPath();
      ctx.ellipse(h.cx, h.cy + 10, 58, 24, 0, 0, 6.283);
      ctx.fill();
      if (!h.occ) continue;
      const o = h.occ;
      const show = Math.min(1, o.age * 7) * (o.life < 0.3 ? Math.max(0, o.life / 0.3) : 1);
      const ey = h.cy - 16 + (1 - show) * 50;
      ctx.save();
      ctx.beginPath();
      ctx.rect(h.cx - 58, 0, 116, h.cy + 14);
      ctx.clip();
      if (o.type === 'bug') {
        ctx.fillStyle = '#3aa84a';
        ctx.fillRect(h.cx - 22, ey - 18, 44, 36);
        ctx.fillStyle = '#5fd06a';
        ctx.fillRect(h.cx - 22, ey - 18, 44, 12);
        ctx.fillStyle = '#fff';
        ctx.fillRect(h.cx - 13, ey - 8, 11, 11);
        ctx.fillRect(h.cx + 3, ey - 8, 11, 11);
        ctx.fillStyle = '#10180f';
        ctx.fillRect(h.cx - 9, ey - 4, 5, 5);
        ctx.fillRect(h.cx + 6, ey - 4, 5, 5);
        ctx.fillStyle = '#2a7d36';
        ctx.fillRect(h.cx - 16, ey - 28, 5, 12);
        ctx.fillRect(h.cx + 11, ey - 28, 5, 12);
      } else {
        ctx.fillStyle = '#1a1a22';
        ctx.fillRect(h.cx - 20, ey - 16, 40, 38);
        ctx.fillStyle = '#3a3a48';
        ctx.fillRect(h.cx - 20, ey - 16, 40, 10);
        ctx.fillStyle = '#e0463f';
        ctx.fillRect(h.cx - 20, ey + 4, 40, 7);
        ctx.fillStyle = '#8a6526';
        ctx.fillRect(h.cx - 2, ey - 26, 4, 10);
        ctx.fillStyle = (Math.floor(G.time * 12) % 2) ? '#ffd34d' : '#ff8a3c';
        ctx.fillRect(h.cx - 4, ey - 32, 8, 8);
      }
      ctx.restore();
    }
  },
};

// ========================================================================
// GAME 7: MEMORY (Simon Says — color-sequence recall)
// ========================================================================
GAMES.memory = {
  _pads(G) {
    // Pre-computed once per init; positions stay stable through the run.
    const px = 70, py = 200, gap = 24, sz = 150;
    return [
      { id: 0, x: px,             y: py,            sz, col: '#e85a3a', hi: '#ff8a6a' },
      { id: 1, x: px + sz + gap,  y: py,            sz, col: '#5fc06e', hi: '#86df9d' },
      { id: 2, x: px,             y: py + sz + gap, sz, col: '#5fc0ff', hi: '#82c0ff' },
      { id: 3, x: px + sz + gap,  y: py + sz + gap, sz, col: '#f4d27b', hi: '#fff0c8' },
    ];
  },
  init(G) {
    G.s = {
      pads: this._pads(G),
      seq: [],
      step: 0,           // index of next sequence entry the player must tap
      mode: 'show',      // 'show' or 'input'
      showIdx: 0,        // index of the pad currently being shown
      showT: 0,
      gapT: 0.6,
      flash: -1,         // currently-lit pad id (any mode)
      flashT: 0,
      inputT: 0,         // seconds since last tap (timeout safety)
    };
    this._grow(G.s);
  },
  _grow(s) {
    // First round starts at length 3 for accessibility, then +1 per round.
    const add = s.seq.length === 0 ? 3 : 1;
    for (let i = 0; i < add; i++) s.seq.push((Math.random() * 4) | 0);
  },
  down(G, x, y) {
    const s = G.s;
    if (s.mode !== 'input' || G.over) return;
    for (const p of s.pads) {
      if (x >= p.x && x <= p.x + p.sz && y >= p.y && y <= p.y + p.sz) {
        s.flash = p.id; s.flashT = 0.22; s.inputT = 0;
        if (p.id === s.seq[s.step]) {
          s.step++;
          if (s.step >= s.seq.length) {
            G.score++;
            burst(p.x + p.sz / 2, p.y + p.sz / 2, 14, '#bda6ff');
            s.mode = 'show'; s.showIdx = 0; s.gapT = 0.55;
            this._grow(s);
          }
        } else {
          G.shake = 0.35;
          endGame();
        }
        return;
      }
    }
  },
  update(G, dt) {
    const s = G.s;
    if (s.flashT > 0) { s.flashT -= dt; if (s.flashT <= 0) s.flash = -1; }
    if (s.mode === 'show') {
      if (s.gapT > 0) { s.gapT -= dt; return; }
      if (s.flash < 0) {
        if (s.showIdx >= s.seq.length) {
          s.mode = 'input'; s.step = 0; s.inputT = 0;
          return;
        }
        s.flash = s.seq[s.showIdx];
        s.flashT = Math.max(0.22, 0.55 - G.score * 0.025);
      } else if (s.flashT <= 0) {
        s.gapT = 0.12;
        s.showIdx++;
      }
    } else {
      s.inputT += dt;
      // 8-second per-round soft timeout.
      if (s.inputT > 8) { G.shake = 0.3; endGame(); }
    }
  },
  render(G, ctx) {
    skyGradient(ctx, W, H, '#1a1230', '#0d0918');
    const s = G.s;
    // Header
    ctx.fillStyle = '#bda6ff';
    ctx.font = 'bold 20px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(s.mode === 'show' ? '◉ WATCH ◉' : '✓ YOUR TURN ✓', W / 2, 140);
    ctx.fillStyle = '#a8a0c4';
    ctx.font = '13px monospace';
    ctx.fillText('Length ' + s.seq.length + '   Step ' + (s.mode === 'input' ? s.step + 1 : s.showIdx + 1) + '/' + s.seq.length,
      W / 2, 168);
    // Pads
    for (const p of s.pads) {
      const lit = s.flash === p.id;
      ctx.fillStyle = '#070315';
      ctx.fillRect(p.x - 3, p.y - 3, p.sz + 6, p.sz + 6);
      ctx.fillStyle = lit ? p.hi : p.col;
      ctx.fillRect(p.x, p.y, p.sz, p.sz);
      if (lit) {
        ctx.fillStyle = 'rgba(255,255,255,0.4)';
        ctx.fillRect(p.x, p.y, p.sz, p.sz);
      }
      // Inner border for pixel-art look.
      ctx.fillStyle = 'rgba(0,0,0,0.25)';
      ctx.fillRect(p.x, p.y + p.sz - 6, p.sz, 6);
      ctx.fillRect(p.x + p.sz - 6, p.y, 6, p.sz);
    }
  },
};

// ---- HUD ---------------------------------------------------------------
function drawHud() {
  ctx.font = 'bold 30px monospace';
  ctx.textAlign = 'left';
  ctx.fillStyle = 'rgba(0,0,0,0.4)';
  ctx.fillText(G.score, 18, 46);
  ctx.fillStyle = '#fff';
  ctx.fillText(G.score, 16, 44);
  if (G.lives >= 0) {
    ctx.textAlign = 'right';
    let hs = '';
    for (let i = 0; i < G.lives; i++) hs += '♥';
    ctx.font = 'bold 24px monospace';
    ctx.fillStyle = '#ff5a6e';
    ctx.fillText(hs || ' ', W - 16, 42);
  }
  if (!G.started) {
    ctx.fillStyle = 'rgba(10,8,20,0.62)';
    ctx.fillRect(0, H / 2 - 90, W, 180);
    ctx.textAlign = 'center';
    ctx.fillStyle = '#ffd34d';
    ctx.font = 'bold 26px monospace';
    ctx.fillText(gameName(G.id), W / 2, H / 2 - 36);
    ctx.fillStyle = '#fff';
    ctx.font = '15px monospace';
    wrapText(gameInstr(G.id), W / 2, H / 2, W - 70, 22);
    ctx.fillStyle = '#7fe8ff';
    ctx.font = 'bold 17px monospace';
    ctx.fillText(t('tapStart'), W / 2, H / 2 + 62);
  }
}
function wrapText(text, cx, y, maxW, lh) {
  const words = text.split(' ');
  let line = '', yy = y;
  for (const w of words) {
    const test = line ? line + ' ' + w : w;
    if (ctx.measureText(test).width > maxW && line) {
      ctx.fillText(line, cx, yy); line = w; yy += lh;
    } else line = test;
  }
  ctx.fillText(line, cx, yy);
}

// ---- loop --------------------------------------------------------------
function loop(now) {
  rafId = requestAnimationFrame(loop);
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.05) dt = 0.05;
  if (!G || $('screen-play').classList.contains('hidden')) return;

  if (G.started && !G.over && !paused) {
    G.time += dt;
    GAMES[G.id].update(G, dt);
    for (const p of particles) { p.life -= dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 260 * dt; }
    particles = particles.filter(p => p.life > 0);
  }
  if (G.shake > 0) G.shake -= dt;

  ctx.save();
  if (G.shake > 0) {
    ctx.translate((Math.random() - 0.5) * G.shake * 28, (Math.random() - 0.5) * G.shake * 28);
  }
  GAMES[G.id].render(G, ctx);
  for (const p of particles) {
    ctx.globalAlpha = Math.max(0, p.life / p.max);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x - p.size / 2, p.y - p.size / 2, p.size, p.size);
  }
  ctx.globalAlpha = 1;
  drawHud();
  ctx.restore();
}

// ---- input -------------------------------------------------------------
let rect = null;
function updateRect() { rect = canvas.getBoundingClientRect(); }
function toCanvas(cx, cy) {
  if (!rect) updateRect();
  return { x: (cx - rect.left) / rect.width * W, y: (cy - rect.top) / rect.height * H };
}
function handle(type, cx, cy) {
  if (!G || G.over || paused) return;
  const p = toCanvas(cx, cy);
  if (!G.started) {
    if (type === 'down') { G.started = true; G.time = 0; }
    return; // the first tap only starts the game
  }
  const g = GAMES[G.id];
  if (type === 'down' && g.down) g.down(G, p.x, p.y);
  if (type === 'move' && g.move) g.move(G, p.x, p.y);
}
canvas.addEventListener('pointerdown', e => { e.preventDefault(); handle('down', e.clientX, e.clientY); });
canvas.addEventListener('pointermove', e => {
  if (e.pressure > 0 || e.buttons || e.pointerType === 'touch') handle('move', e.clientX, e.clientY);
});
window.addEventListener('keydown', e => {
  if (e.key === ' ' || e.key === 'ArrowUp') {
    e.preventDefault();
    if (G && !$('screen-play').classList.contains('hidden')) handle('down', rect ? rect.left + rect.width / 2 : 0, rect ? rect.top + rect.height / 2 : 0);
  }
});

// ---- resize ------------------------------------------------------------
function resize() {
  const stage = $('stage');
  const scl = Math.min(stage.clientWidth / W, stage.clientHeight / H);
  canvas.style.width = Math.floor(W * scl) + 'px';
  canvas.style.height = Math.floor(H * scl) + 'px';
  updateRect();
}
window.addEventListener('resize', resize);
window.addEventListener('orientationchange', () => setTimeout(resize, 200));

// ---- hub ---------------------------------------------------------------
function renderHub() {
  const grid = $('game-grid');
  grid.innerHTML = '';
  for (const id of GAME_IDS) {
    const best = save.best[id] || 0;
    const med = medalOf(id, best);
    const card = document.createElement('button');
    card.className = 'game-card gc-' + id;
    card.innerHTML =
      `<div class="gc-art"></div>` +
      `<div class="gc-name">${gameName(id)}</div>` +
      `<div class="gc-best">${t('best')}: ${best} ${med ? MEDAL_ICON[med] : ''}</div>`;
    card.onclick = () => startGame(id);
    grid.appendChild(card);
  }
  // medal tally
  let gold = 0;
  for (const id of GAME_IDS) { if (medalOf(id, save.best[id] || 0) === 3) gold++; }
  $('hub-medals').textContent = '🥇 ' + gold + '/' + GAME_IDS.length;
}

// ---- wire --------------------------------------------------------------
function bindUI() {
  $('btn-back').onclick = () => { showScreen('hub'); renderHub(); };
  $('btn-pause').onclick = () => {
    if (!G || G.over || !G.started) return;
    paused = true; $('overlay-pause').classList.remove('hidden');
  };
  $('btn-resume').onclick = () => { paused = false; $('overlay-pause').classList.add('hidden'); };
  $('btn-pause-quit').onclick = () => {
    paused = false; $('overlay-pause').classList.add('hidden');
    showScreen('hub'); renderHub();
  };
  $('btn-over-retry').onclick = () => startGame(G.id);
  $('btn-over-hub').onclick = () => { showScreen('hub'); renderHub(); };
  setupLanguageToggle(() => { renderHub(); });
}

bindUI();
applyStaticText();
renderHub();
showScreen('hub');
lastT = performance.now();
rafId = requestAnimationFrame(loop);

})();
