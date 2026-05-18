// Pixel Arcade - shared pixel art helpers

function pr(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(Math.round(x), Math.round(y), Math.ceil(w), Math.ceil(h)); }

function skyGradient(ctx, w, h, top, bot) {
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, top); g.addColorStop(1, bot);
  ctx.fillStyle = g; ctx.fillRect(0, 0, w, h);
}
function drawCloud(ctx, x, y, s) {
  ctx.fillStyle = 'rgba(255,255,255,0.85)';
  ctx.fillRect(x, y, 26 * s, 12 * s);
  ctx.fillRect(x + 6 * s, y - 7 * s, 16 * s, 12 * s);
  ctx.fillRect(x + 14 * s, y - 3 * s, 14 * s, 10 * s);
}

// flap bird
function drawBird(ctx, x, y, t, vy) {
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(Math.max(-0.5, Math.min(0.9, vy / 600)));
  const flap = Math.sin(t * 20) > 0 ? -6 : 4;
  pr(ctx, -14, 4, 14, 8, flap < 0 ? '#e8a838' : '#c98a28'); // wing
  pr(ctx, -13, -11, 24, 22, '#ffd24a');                     // body
  pr(ctx, -13, -11, 24, 6, '#ffe78a');
  pr(ctx, 11, -4, 8, 7, '#ff8a3a');                         // beak
  pr(ctx, 3, -8, 8, 8, '#fff');                             // eye
  pr(ctx, 7, -6, 4, 4, '#1a1422');
  pr(ctx, -16, 2, 6, 6, '#c98a28');
  ctx.restore();
}
function drawPipe(ctx, x, gapY, gapH, w, h) {
  const c = '#4fae4a', cd = '#2f7d2c', cl = '#7ad06a';
  // top
  pr(ctx, x, 0, w, gapY, c);
  pr(ctx, x, 0, 6, gapY, cl);
  pr(ctx, x - 5, gapY - 26, w + 10, 26, c);
  pr(ctx, x - 5, gapY - 26, w + 10, 6, cl);
  pr(ctx, x + w - 6, 0, 6, gapY, cd);
  // bottom
  const by = gapY + gapH;
  pr(ctx, x, by, w, h - by, c);
  pr(ctx, x, by, 6, h - by, cl);
  pr(ctx, x - 5, by, w + 10, 26, c);
  pr(ctx, x - 5, by, w + 10, 6, cl);
  pr(ctx, x + w - 6, by, 6, h - by, cd);
}

// catch basket
function drawBasket(ctx, x, y, w) {
  pr(ctx, x - w / 2, y, w, 8, '#caa14a');
  pr(ctx, x - w / 2, y, w, 3, '#e8c66a');
  pr(ctx, x - w / 2, y + 8, w, 18, '#8a6238');
  for (let i = 0; i < 5; i++) pr(ctx, x - w / 2 + 4 + i * (w - 8) / 5, y + 8, 3, 18, '#6a4a28');
}
function drawFruit(ctx, x, y, kind, t) {
  const cols = ['#ff5a6e', '#ffb13a', '#5fd06a', '#b06ff0'];
  const c = cols[kind % 4];
  ctx.fillStyle = '#3a8a2a';
  pr(ctx, x - 1, y - 14, 3, 6, '#3a8a2a');
  ctx.fillStyle = c;
  ctx.beginPath(); ctx.arc(x, y, 11, 0, 6.28); ctx.fill();
  ctx.fillStyle = 'rgba(255,255,255,0.55)';
  ctx.beginPath(); ctx.arc(x - 4, y - 4, 3.5, 0, 6.28); ctx.fill();
}
function drawBomb(ctx, x, y, t) {
  ctx.fillStyle = '#2a2535';
  ctx.beginPath(); ctx.arc(x, y, 11, 0, 6.28); ctx.fill();
  ctx.fillStyle = '#54506a';
  ctx.beginPath(); ctx.arc(x - 4, y - 4, 3.5, 0, 6.28); ctx.fill();
  pr(ctx, x - 2, y - 16, 4, 6, '#8a6238');
  ctx.fillStyle = Math.sin(t * 18) > 0 ? '#ffd34d' : '#ff7a3a';
  ctx.beginPath(); ctx.arc(x + 1, y - 17, 3, 0, 6.28); ctx.fill();
}

// reflex target
function drawTarget(ctx, x, y, r, ttl) {
  const rings = ['#ff5a6e', '#fff', '#ff5a6e', '#ffd34d'];
  for (let i = 0; i < 4; i++) {
    ctx.fillStyle = rings[i];
    ctx.beginPath(); ctx.arc(x, y, r * (1 - i * 0.25), 0, 6.28); ctx.fill();
  }
  // ttl ring
  ctx.strokeStyle = '#fff'; ctx.lineWidth = 3;
  ctx.beginPath(); ctx.arc(x, y, r + 5, -Math.PI / 2, -Math.PI / 2 + ttl * 6.28); ctx.stroke();
}

// stack block
function drawBlock(ctx, x, y, w, h, hue) {
  const c = `hsl(${hue},58%,58%)`, cl = `hsl(${hue},58%,70%)`, cd = `hsl(${hue},48%,42%)`;
  pr(ctx, x, y, w, h, c);
  pr(ctx, x, y, w, 5, cl);
  pr(ctx, x, y + h - 5, w, 5, cd);
}

// dash runner
function drawRunner(ctx, x, y, t, jumping) {
  ctx.save();
  ctx.translate(x, y);
  const run = jumping ? 0 : Math.sin(t * 18) * 4;
  pr(ctx, -7, 8, 6, 10 + run, '#3a4f8c');     // leg
  pr(ctx, 1, 8, 6, 10 - run, '#3a4f8c');
  pr(ctx, -9, -10, 18, 18, '#5b78c4');         // body
  pr(ctx, -9, -10, 18, 5, '#88a0e0');
  pr(ctx, -7, -24, 14, 14, '#e8b98a');         // head
  pr(ctx, -7, -24, 14, 5, '#c2c9d6');
  pr(ctx, 0, -19, 4, 4, '#1a1422');            // eye
  pr(ctx, 8, -8, 6, 10 + run, '#e8b98a');      // arm
  ctx.restore();
}
function drawSpike(ctx, x, y, w) {
  ctx.fillStyle = '#c44';
  const n = Math.max(1, Math.round(w / 14));
  for (let i = 0; i < n; i++) {
    const sx = x + i * (w / n);
    ctx.beginPath();
    ctx.moveTo(sx, y); ctx.lineTo(sx + w / n / 2, y - 22); ctx.lineTo(sx + w / n, y);
    ctx.closePath(); ctx.fill();
  }
  ctx.fillStyle = '#7a2424';
  pr(ctx, x, y - 2, w, 4, '#7a2424');
}
function drawCrateObs(ctx, x, y, w, h) {
  pr(ctx, x, y, w, h, '#8a6238');
  pr(ctx, x, y, w, 5, '#a8804a');
  ctx.strokeStyle = '#4a3318'; ctx.lineWidth = 3;
  ctx.strokeRect(x + 1.5, y + 1.5, w - 3, h - 3);
}
function drawGround(ctx, y, w, h, t, color) {
  pr(ctx, 0, y, w, h - y, color || '#3a2f2a');
  ctx.fillStyle = 'rgba(255,255,255,0.06)';
  for (let x = -(t * 180 % 40); x < w; x += 40) pr(ctx, x, y, 20, 5, 'rgba(255,255,255,0.08)');
}
