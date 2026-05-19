// Pixel Beat Runner - a 4-lane rhythm game. Notes fall down lanes; tap each
// lane as its note crosses the line. Hits play procedural Web Audio tones.

const BEST_KEY = 'pixel-beat-runner-best';
const LEAD = (HIT_Y + 70) / SPEED;     // seconds a note is visible before the line

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();
const keys = { d: 0, f: 1, j: 2, k: 3 };

// ---- audio -------------------------------------------------------------
let actx = null;
function audioInit() {
  if (actx) { if (actx.state === 'suspended') actx.resume(); return; }
  try { actx = new (window.AudioContext || window.webkitAudioContext)(); }
  catch (e) { actx = null; }
}
function tone(freq, type, dur, vol) {
  if (!actx) return;
  const o = actx.createOscillator(), g = actx.createGain();
  o.type = type; o.frequency.value = freq;
  o.connect(g); g.connect(actx.destination);
  const now = actx.currentTime;
  g.gain.setValueAtTime(0.0001, now);
  g.gain.exponentialRampToValueAtTime(vol, now + 0.008);
  g.gain.exponentialRampToValueAtTime(0.0001, now + dur);
  o.start(now); o.stop(now + dur + 0.02);
}
function hitSound(lane, perfect) {
  tone(LANE_HZ[lane], 'square', 0.16, 0.2);
  if (perfect) tone(LANE_HZ[lane] * 2, 'triangle', 0.22, 0.12);
}
function missSound() { tone(98, 'sawtooth', 0.16, 0.16); }

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = {
    notes: [], gt: 0, nextBeat: 2.0, health: HEALTH_MAX,
    score: 0, combo: 0, maxCombo: 0, judged: 0, accSum: 0,
    fx: [], lanePulse: [0, 0, 0, 0], banner: null, over: false,
  };
}

function curBpm() {
  return Math.min(MAX_BPM, START_BPM + Math.floor(game.gt / RAMP_EVERY) * BPM_STEP);
}
function difficulty() { return Math.min(1, game.gt / 120); }

function spawnBeat(beatTime, gap) {
  const d = difficulty();
  const l1 = (Math.random() * LANES) | 0;
  game.notes.push({ lane: l1, time: beatTime, judged: false });
  if (Math.random() < 0.18 + d * 0.42) {
    let l2 = (Math.random() * LANES) | 0;
    if (l2 === l1) l2 = (l2 + 1) % LANES;
    game.notes.push({ lane: l2, time: beatTime, judged: false });
  }
  if (Math.random() < d * 0.34) {
    game.notes.push({ lane: (Math.random() * LANES) | 0, time: beatTime + gap / 2, judged: false });
  }
}

function setBanner(text, color) { game.banner = { text, color, life: 0.7 }; }

// ---- judgement ---------------------------------------------------------
function hitLane(lane) {
  if (!game || game.over || !overlaysClosed()) return;
  game.lanePulse[lane] = 1;
  // nearest un-judged note in this lane within the miss window
  let best = null, bestOff = W_MISS + 1;
  for (const n of game.notes) {
    if (n.judged || n.lane !== lane) continue;
    const off = Math.abs(game.gt - n.time);
    if (off < bestOff) { bestOff = off; best = n; }
  }
  if (!best || bestOff > W_MISS) return;       // empty tap - no penalty
  best.judged = true;
  game.judged++;
  if (bestOff <= W_PERFECT) {
    game.combo++; game.accSum += 1;
    game.score += 100 + Math.min(200, game.combo * 4);
    game.health = Math.min(HEALTH_MAX, game.health + HP_PERFECT);
    setBanner(t('perfect'), '#ffe9a0');
    hitSound(lane, true);
  } else if (bestOff <= W_GOOD) {
    game.combo++; game.accSum += 0.65;
    game.score += 50 + Math.min(120, game.combo * 2);
    game.health = Math.min(HEALTH_MAX, game.health + HP_GOOD);
    setBanner(t('good'), '#7fe0a0');
    hitSound(lane, false);
  } else {
    game.combo = 0;
    game.health += HP_MISS;
    setBanner(t('miss'), '#ff6b6b');
    missSound();
  }
  game.maxCombo = Math.max(game.maxCombo, game.combo);
  game.fx.push({ lane, color: LANE_COLOR[lane], life: 1 });
  if (game.health <= 0) gameOver();
}

function gameOver() {
  if (game.over) return;
  game.over = true;
  if (game.score > bestScore()) localStorage.setItem(BEST_KEY, game.score);
  document.getElementById('over-score').textContent = t('finalScore', game.score);
  document.getElementById('over-combo').textContent = t('maxCombo', game.maxCombo);
  document.getElementById('over-best').textContent = t('bestScore', bestScore());
  showOverlay('overlay-over');
}

// ---- update ------------------------------------------------------------
function update(dt) {
  const g = game;
  g.gt += dt;
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }
  for (let i = 0; i < LANES; i++) if (g.lanePulse[i] > 0) g.lanePulse[i] -= dt * 4;
  for (const f of g.fx) f.life -= dt * 2.4;
  g.fx = g.fx.filter(f => f.life > 0);

  // spawn upcoming beats
  const gap = beatInterval(curBpm());
  let guard = 0;
  while (g.nextBeat - g.gt < LEAD && guard < 40) {
    spawnBeat(g.nextBeat, gap);
    g.nextBeat += gap;
    guard++;
  }
  // notes that slipped past unhit -> miss
  for (const n of g.notes) {
    if (!n.judged && g.gt > n.time + W_MISS) {
      n.judged = true;
      g.judged++;
      g.combo = 0;
      g.health += HP_MISS;
      g.fx.push({ lane: n.lane, color: '#5a3a4a', life: 1 });
      missSound();
      if (g.health <= 0) { gameOver(); return; }
    }
  }
  g.notes = g.notes.filter(n => !(n.judged && game.gt > n.time + 0.4));
}

// ---- render ------------------------------------------------------------
function render() {
  const g = game;
  drawHighway(ctx, g.gt, g.lanePulse);
  for (const n of g.notes) {
    if (n.judged) continue;
    const y = HIT_Y - (n.time - g.gt) * SPEED;
    if (y < -40 || y > VH + 40) continue;
    drawNote(ctx, n.lane, y);
  }
  for (const f of g.fx) drawHitFx(ctx, f);
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life * 2);
    ctx.fillStyle = g.banner.color;
    ctx.font = '900 30px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, VW / 2, HIT_Y - 70);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  updateHud();
}

function updateHud() {
  const g = game;
  document.getElementById('hud-score').textContent = g.score;
  document.getElementById('hud-combo').textContent = g.combo > 1 ? `x${g.combo}` : '—';
  const acc = g.judged ? Math.round(g.accSum / g.judged * 100) : 100;
  document.getElementById('hud-acc').textContent = `${acc}%`;
  const f = Math.max(0, g.health / HEALTH_MAX);
  document.getElementById('hp-fill').style.width = (f * 100) + '%';
  document.getElementById('hp-fill').style.background = f > 0.4 ? '#5fd9a0' : '#ff6b6b';
}

function bestScore() { return +(localStorage.getItem(BEST_KEY) || 0); }

// ---- screens / overlays -----------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() { document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden')); }
function overlaysClosed() { return document.querySelectorAll('.overlay:not(.hidden)').length === 0; }

function startGame() { audioInit(); newGame(); hideAllOverlays(); showScreen('screen-game'); }
function gotoTitle() {
  hideAllOverlays();
  document.getElementById('title-best').textContent = t('bestScore', bestScore());
  showScreen('screen-title');
}
function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideAllOverlays();
}

// ---- input -------------------------------------------------------------
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const k = e.key.toLowerCase();
  if (k in keys) { e.preventDefault(); hitLane(keys[k]); }
  else if (e.key === 'Escape') togglePause();
});
canvas.addEventListener('pointerdown', e => {
  e.preventDefault();
  const r = canvas.getBoundingClientRect();
  const x = (e.clientX - r.left) / r.width * VW;
  hitLane(Math.max(0, Math.min(LANES - 1, Math.floor(x / LANE_W))));
});
document.querySelectorAll('[data-lane]').forEach(btn => {
  btn.addEventListener('pointerdown', e => { e.preventDefault(); hitLane(+btn.dataset.lane); });
});

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-resume').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = startGame;
document.getElementById('btn-pause-menu').onclick = gotoTitle;
document.getElementById('btn-over-again').onclick = startGame;
document.getElementById('btn-over-menu').onclick = gotoTitle;
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestScore', bestScore());
});

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  if (game && !game.over && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    update(dt);
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
