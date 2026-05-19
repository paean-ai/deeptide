// Pixel Vector - rendering + on-screen controls.

const COL = {
  space1: '#080a16', space2: '#0d1024',
  star: '#9aa8d0',
  ship: '#7fffd4', shipThrust: '#ff8a3a', shipDim: '#3a6e6a',
  asteroid: '#8c8f99', asteroidEdge: '#bfc4d4',
  bullet: '#ffd86b',
  field: '#1a1f36',
  pad: '#2e2a48', padOn: '#7fffd4', padIco: '#e6ebf5',
  fire: '#ff6e3a', fireDark: '#7a2810',
};

const BTN = {
  left:   { x: 8,   y: 396, w: 78, h: 76, key: 'left' },
  right:  { x: 92,  y: 396, w: 78, h: 76, key: 'right' },
  thrust: { x: 178, y: 396, w: 70, h: 76, key: 'thrust' },
  fire:   { x: 254, y: 396, w: 98, h: 76, key: 'fire' },
};

const STARS = (() => {
  const out = [];
  let s = 12345;
  for (let i = 0; i < 36; i++) {
    s = (s * 16807) % 2147483647;
    out.push({ x: (s % FIELD_W), y: FIELD_TOP + (s * 7) % FIELD_H, b: ((s >> 4) % 3) === 0 ? 1.5 : 1 });
  }
  return out;
})();

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, COL.space1);
  g.addColorStop(1, COL.space2);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // field box
  ctx.fillStyle = COL.field;
  ctx.fillRect(0, FIELD_TOP, FIELD_W, FIELD_BOTTOM - FIELD_TOP);
  // stars
  ctx.fillStyle = COL.star;
  for (const st of STARS) ctx.fillRect(st.x, st.y, st.b, st.b);
  // field border
  ctx.strokeStyle = '#3a4068';
  ctx.lineWidth = 1;
  ctx.strokeRect(0.5, FIELD_TOP + 0.5, FIELD_W - 1, FIELD_BOTTOM - FIELD_TOP - 1);
}

function drawShip(ctx, s, input) {
  if (!s.ship.alive) return;
  const blink = s.invuln > 0 && (((s.invuln * 8) | 0) & 1);
  if (blink) return;
  ctx.save();
  ctx.translate(s.ship.x, s.ship.y);
  ctx.rotate(s.ship.angle + Math.PI / 2);
  if (input && input.thrust) {
    ctx.fillStyle = COL.shipThrust;
    ctx.beginPath();
    ctx.moveTo(-4, 8); ctx.lineTo(0, 16 + Math.random() * 4); ctx.lineTo(4, 8);
    ctx.closePath(); ctx.fill();
  }
  ctx.fillStyle = COL.ship;
  ctx.beginPath();
  ctx.moveTo(0, -10); ctx.lineTo(7, 8); ctx.lineTo(0, 4); ctx.lineTo(-7, 8);
  ctx.closePath(); ctx.fill();
  ctx.fillStyle = COL.shipDim;
  ctx.fillRect(-1, -2, 2, 4);
  ctx.restore();
}

function drawAsteroid(ctx, a) {
  ctx.save();
  ctx.translate(a.x, a.y);
  ctx.rotate(a.rot);
  ctx.fillStyle = COL.asteroid;
  ctx.beginPath();
  for (let i = 0; i < a.verts.length; i++) {
    const v = a.verts[i];
    if (i === 0) ctx.moveTo(v.x, v.y); else ctx.lineTo(v.x, v.y);
  }
  ctx.closePath();
  ctx.fill();
  ctx.strokeStyle = COL.asteroidEdge;
  ctx.lineWidth = 1.5;
  ctx.stroke();
  ctx.restore();
}

function drawWorld(ctx, s, input) {
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, FIELD_TOP, FIELD_W, FIELD_BOTTOM - FIELD_TOP);
  ctx.clip();
  for (const a of s.asteroids) {
    drawAsteroid(ctx, a);
    // ghost on wrap
    if (a.x < a.r) drawAsteroid(ctx, { ...a, x: a.x + FIELD_W });
    if (a.x > FIELD_W - a.r) drawAsteroid(ctx, { ...a, x: a.x - FIELD_W });
    if (a.y < FIELD_TOP + a.r) drawAsteroid(ctx, { ...a, y: a.y + FIELD_H });
    if (a.y > FIELD_BOTTOM - a.r) drawAsteroid(ctx, { ...a, y: a.y - FIELD_H });
  }
  ctx.fillStyle = COL.bullet;
  for (const b of s.bullets) ctx.fillRect(b.x - 1.5, b.y - 1.5, 3, 3);
  drawShip(ctx, s, input);
  ctx.restore();
}

function drawControls(ctx, pressed) {
  for (const k in BTN) {
    const b = BTN[k];
    const isFire = k === 'fire';
    ctx.fillStyle = pressed[k] ? (isFire ? '#ffd49a' : COL.padOn) : (isFire ? COL.fire : COL.pad);
    ctx.fillRect(b.x, b.y, b.w, b.h);
    ctx.strokeStyle = isFire ? COL.fireDark : '#1a1a26';
    ctx.lineWidth = 2;
    ctx.strokeRect(b.x + 1, b.y + 1, b.w - 2, b.h - 2);
    ctx.fillStyle = isFire ? '#1a0a06' : COL.padIco;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const cx = b.x + b.w / 2, cy = b.y + b.h / 2;
    if (k === 'left') {
      ctx.beginPath();
      ctx.moveTo(cx - 14, cy); ctx.lineTo(cx + 8, cy - 14); ctx.lineTo(cx + 8, cy + 14);
      ctx.closePath(); ctx.fill();
    } else if (k === 'right') {
      ctx.beginPath();
      ctx.moveTo(cx + 14, cy); ctx.lineTo(cx - 8, cy - 14); ctx.lineTo(cx - 8, cy + 14);
      ctx.closePath(); ctx.fill();
    } else if (k === 'thrust') {
      ctx.font = 'bold 11px monospace';
      ctx.fillText('THRUST', cx, cy);
      ctx.beginPath();
      ctx.moveTo(cx, cy - 18); ctx.lineTo(cx - 6, cy - 8); ctx.lineTo(cx + 6, cy - 8);
      ctx.closePath(); ctx.fill();
    } else if (k === 'fire') {
      ctx.font = 'bold 22px monospace';
      ctx.fillText('FIRE', cx, cy + 2);
    }
  }
}
