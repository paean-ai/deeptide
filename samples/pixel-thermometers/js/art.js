// Pixel-art rendering for Pixel Thermometers. 360x480 world units.

const PALETTE = {
  bg:        '#0d1530',
  board:     '#202a4a',
  gridLine:  '#2c3860',
  glass:     '#cdd7e8',
  glassHi:   '#eef2fb',
  glassLo:   '#9aa6c4',
  mercury:   '#e8554f',
  mercuryHi: '#ff8f72',
  mercuryLo: '#9c2f29',
  outline:   '#0a0e1f',
  shine:     '#ffffff',
  clue:      '#dfe6f4',
  clueOk:    '#5fd07a',
  clueOver:  '#ff6a6a',
  conflict:  'rgba(232,85,79,0.42)',
  hud:       '#0a1024',
  hudText:   '#f4f3ea',
  hudDim:    '#8b96b6',
  accent:    '#f4c44a',
};

function gridGeometry(n) {
  const size = 300;
  const cell = (size / n) | 0;
  const total = cell * n;
  const ox = ((360 - total) / 2) | 0;
  const oy = 70;
  return { cell, total, ox, oy };
}

function cellCenter(g, n, cell) {
  const x = cell % n, y = (cell / n) | 0;
  return { cx: g.ox + x * g.cell + g.cell / 2, cy: g.oy + y * g.cell + g.cell / 2 };
}

// One thermometer drawn as a chain of capsule segments + a round bulb.
// mode: 'outline' | 'glass' | 'mercury' (mercury only paints the filled run).
function drawThermoLayer(ctx, g, n, thermo, fill, mode, color) {
  const tw = g.cell * 0.66;
  const w = mode === 'outline' ? tw + 3 : tw;
  const rB = (mode === 'outline' ? tw * 0.66 + 2 : tw * 0.66);
  ctx.fillStyle = color;
  // Connectors (each covers the two cells it spans, so the chain is solid).
  const last = mode === 'mercury' ? fill : thermo.length;
  for (let j = 0; j < thermo.length - 1; j++) {
    if (mode === 'mercury' && j + 1 >= last) break;
    const a = cellCenter(g, n, thermo[j]);
    const b = cellCenter(g, n, thermo[j + 1]);
    const x0 = Math.min(a.cx, b.cx) - w / 2, x1 = Math.max(a.cx, b.cx) + w / 2;
    const y0 = Math.min(a.cy, b.cy) - w / 2, y1 = Math.max(a.cy, b.cy) + w / 2;
    ctx.fillRect(x0 | 0, y0 | 0, (x1 - x0) | 0, (y1 - y0) | 0);
  }
  // Bulb at index 0.
  if (!(mode === 'mercury' && fill === 0)) {
    const c0 = cellCenter(g, n, thermo[0]);
    ctx.beginPath();
    ctx.arc(c0.cx, c0.cy, rB, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawScene(ctx, p, marks, conflicts) {
  const g = gridGeometry(p.n);
  const { rowS, colS } = lineCounts(p.n, p.thermos, marks);
  // Board panel.
  ctx.fillStyle = PALETTE.board;
  ctx.fillRect(g.ox - 6, g.oy - 6, g.total + 12, g.total + 12);
  // Faint cell grid.
  ctx.strokeStyle = PALETTE.gridLine;
  ctx.lineWidth = 1;
  for (let i = 0; i <= p.n; i++) {
    ctx.beginPath(); ctx.moveTo(g.ox + i * g.cell + 0.5, g.oy); ctx.lineTo(g.ox + i * g.cell + 0.5, g.oy + g.total); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(g.ox, g.oy + i * g.cell + 0.5); ctx.lineTo(g.ox + g.total, g.oy + i * g.cell + 0.5); ctx.stroke();
  }
  // Row / column clues, tinted by how the current fill compares.
  ctx.font = 'bold ' + Math.min(15, (g.cell * 0.42) | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let x = 0; x < p.n; x++) {
    ctx.fillStyle = colS[x] === p.cc[x] ? PALETTE.clueOk : (colS[x] > p.cc[x] ? PALETTE.clueOver : PALETTE.clue);
    ctx.fillText(String(p.cc[x]), g.ox + x * g.cell + g.cell / 2, g.oy - 16);
  }
  for (let y = 0; y < p.n; y++) {
    ctx.fillStyle = rowS[y] === p.rc[y] ? PALETTE.clueOk : (rowS[y] > p.rc[y] ? PALETTE.clueOver : PALETTE.clue);
    ctx.fillText(String(p.rc[y]), g.ox - 17, g.oy + y * g.cell + g.cell / 2);
  }
  // Thermometers, in three sweeps so outlines never cover a glass tube.
  for (const th of p.thermos) drawThermoLayer(ctx, g, p.n, th, 0, 'outline', PALETTE.outline);
  for (const th of p.thermos) drawThermoLayer(ctx, g, p.n, th, 0, 'glass', PALETTE.glass);
  // Glass shine: a soft highlight along each tube and a bulb dot.
  for (const th of p.thermos) {
    const c0 = cellCenter(g, p.n, th[0]);
    ctx.fillStyle = PALETTE.glassHi;
    ctx.fillRect((c0.cx - g.cell * 0.2) | 0, (c0.cy - g.cell * 0.28) | 0, 3, 3);
  }
  for (let ti = 0; ti < p.thermos.length; ti++) {
    drawThermoLayer(ctx, g, p.n, p.thermos[ti], marks[ti], 'mercury', PALETTE.mercury);
  }
  // Mercury bulb sheen.
  for (let ti = 0; ti < p.thermos.length; ti++) {
    if (marks[ti] === 0) continue;
    const c0 = cellCenter(g, p.n, p.thermos[ti][0]);
    ctx.fillStyle = PALETTE.mercuryHi;
    ctx.fillRect((c0.cx - g.cell * 0.18) | 0, (c0.cy - g.cell * 0.22) | 0, 3, 3);
  }
  // Live conflict tint.
  if (conflicts && conflicts.size) {
    ctx.fillStyle = PALETTE.conflict;
    for (const c of conflicts) {
      const x = c % p.n, y = (c / p.n) | 0;
      ctx.fillRect(g.ox + x * g.cell, g.oy + y * g.cell, g.cell, g.cell);
    }
  }
  // Frame.
  ctx.strokeStyle = PALETTE.gridLine;
  ctx.lineWidth = 2;
  ctx.strokeRect(g.ox - 6, g.oy - 6, g.total + 12, g.total + 12);
}

function drawHud(ctx, lang, p, marks, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (p.levelIndex + 1) + ' ' + p.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  const merc = marks.reduce((a, b) => a + b, 0);
  const target = p.rc.reduce((a, b) => a + b, 0);
  ctx.fillText(t(lang, 'mercury') + ' ' + merc + '/' + target, 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}

// A small standalone thermometer used to decorate the title screen.
function drawTitleThermo(ctx, x, y, h, fillFrac) {
  const w = 16;
  ctx.fillStyle = PALETTE.outline;
  ctx.fillRect(x - w / 2 - 2, y - h - 2, w + 4, h + 4);
  ctx.beginPath(); ctx.arc(x, y + 8, w * 0.95, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.glass;
  ctx.fillRect(x - w / 2, y - h, w, h);
  ctx.beginPath(); ctx.arc(x, y + 8, w * 0.78, 0, Math.PI * 2); ctx.fill();
  const fh = (h * fillFrac) | 0;
  ctx.fillStyle = PALETTE.mercury;
  ctx.fillRect(x - w / 2, y - fh, w, fh);
  ctx.beginPath(); ctx.arc(x, y + 8, w * 0.78, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.mercuryHi;
  ctx.fillRect(x - w / 2 + 2, y - fh, 2, fh);
}
