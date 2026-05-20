// Pixel Bastion - rendering.

const COL = {
  skyTop: '#1a1d3a', skyBottom: '#2a1a3a',
  star: '#7e88c0',
  ground: '#3c2a4a', groundEdge: '#5a3a6a',
  city: '#7dffd4', cityWindow: '#0a1428', cityRubble: '#4a3a5a',
  silo: '#ffd86b', siloDark: '#7a6010',
  incoming: '#ff6e7a', incomingTrail: 'rgba(255,110,122,0.5)',
  counter: '#7dffd4', counterTrail: 'rgba(125,255,212,0.6)',
  blast1: '#ffd86b', blast2: '#ff8a3a', blast3: '#ff3a3a',
  ammoEmpty: '#3a2e3e',
};

const STARS = (() => {
  const out = [];
  let s = 4242;
  for (let i = 0; i < 28; i++) {
    s = (s * 16807) % 2147483647;
    out.push({ x: s % VW, y: 40 + (s * 7) % 280, b: ((s >> 4) % 4) === 0 ? 2 : 1 });
  }
  return out;
})();

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, GROUND_Y);
  g.addColorStop(0, COL.skyTop);
  g.addColorStop(1, COL.skyBottom);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, GROUND_Y);
  ctx.fillStyle = COL.star;
  for (const st of STARS) ctx.fillRect(st.x, st.y, st.b, st.b);
  // ground
  ctx.fillStyle = COL.ground;
  ctx.fillRect(0, GROUND_Y, VW, VH - GROUND_Y);
  ctx.fillStyle = COL.groundEdge;
  ctx.fillRect(0, GROUND_Y, VW, 2);
}

function drawWorld(ctx, s) {
  // cities
  for (const c of s.cities) {
    if (c.alive) {
      ctx.fillStyle = COL.city;
      ctx.fillRect(c.x - CITY_W / 2, c.y - CITY_H, CITY_W, CITY_H);
      ctx.fillStyle = COL.cityWindow;
      for (let yy = 0; yy < 3; yy++) {
        for (let xx = 0; xx < 3; xx++) {
          ctx.fillRect(c.x - CITY_W / 2 + 3 + xx * 7, c.y - CITY_H + 4 + yy * 6, 4, 3);
        }
      }
    } else {
      ctx.fillStyle = COL.cityRubble;
      ctx.fillRect(c.x - CITY_W / 2, c.y - 6, CITY_W, 6);
    }
  }
  // silos
  for (const z of s.silos) {
    ctx.fillStyle = z.ammo > 0 ? COL.silo : COL.ammoEmpty;
    ctx.fillRect(z.x - 10, z.y - 8, 20, 12);
    ctx.fillStyle = COL.siloDark;
    ctx.fillRect(z.x - 3, z.y - 14, 6, 8);
    if (z.ammo > 0) {
      ctx.fillStyle = '#1a1606';
      ctx.font = 'bold 10px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(z.ammo), z.x, z.y - 2);
    }
  }
  // incoming trails + heads
  for (const m of s.incoming) {
    if (!m.alive) continue;
    ctx.strokeStyle = COL.incomingTrail;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(m.sx, m.sy); ctx.lineTo(m.x, m.y);
    ctx.stroke();
    ctx.fillStyle = COL.incoming;
    ctx.fillRect(m.x - 2, m.y - 2, 4, 4);
  }
  // counter trails + heads
  for (const c of s.counters) {
    ctx.strokeStyle = COL.counterTrail;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(c.sx, c.sy); ctx.lineTo(c.x, c.y);
    ctx.stroke();
    ctx.fillStyle = COL.counter;
    ctx.fillRect(c.x - 2, c.y - 2, 4, 4);
  }
  // explosions
  for (const e of s.explosions) {
    const r = explosionRadius(e.age);
    if (r <= 0) continue;
    ctx.fillStyle = COL.blast1;
    ctx.globalAlpha = 0.85;
    ctx.beginPath(); ctx.arc(e.x, e.y, r, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = COL.blast2;
    ctx.globalAlpha = 0.75;
    ctx.beginPath(); ctx.arc(e.x, e.y, r * 0.7, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = COL.blast3;
    ctx.globalAlpha = 0.6;
    ctx.beginPath(); ctx.arc(e.x, e.y, r * 0.4, 0, Math.PI * 2); ctx.fill();
    ctx.globalAlpha = 1;
  }
}
