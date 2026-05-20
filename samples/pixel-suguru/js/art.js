// Pixel-art rendering for Pixel Suguru. 360x480 world units.

const PALETTE = {
  bg: '#1d2240',
  card: '#262d54',
  cell: '#3a4274',
  cellAlt: '#404a82',
  cellSel: '#7d8ed8',
  cellPeer: '#4c5996',
  given: '#f8f5e8',
  user: '#9fd9a6',
  noteText: '#bfc7e6',
  conflict: '#e8554f',
  border: '#0c1230',
  regionEdge: '#f8f5e8',
  hud: '#0d1228',
  hudText: '#f8f5e8',
  hudDim: '#9aa6cc',
  ok: '#54c47c',
};

function gridGeometry(n) {
  // 280px square board centred horizontally, sits below the HUD bar.
  const size = 280;
  const cell = (size / n) | 0;
  const total = cell * n;
  const ox = ((360 - total) / 2) | 0;
  const oy = 72;
  return { cell, total, ox, oy };
}

function drawGrid(ctx, n, regions, clues, user, notes, selected, conflicts) {
  const { cell, ox, oy, total } = gridGeometry(n);
  // Region lookup.
  const cellReg = new Array(n * n);
  regions.forEach((r, id) => r.forEach(c => cellReg[c] = id));
  // Selected region for peer highlight.
  const selReg = selected != null ? cellReg[selected] : -1;
  const selVal = selected != null ? (user[selected] || clues[selected]) : 0;
  // Cell tiles.
  ctx.font = 'bold ' + ((cell * 0.55) | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let i = 0; i < n * n; i++) {
    const r = (i / n) | 0, c = i % n;
    const x = ox + c * cell, y = oy + r * cell;
    let fill = (cellReg[i] % 2 === 0) ? PALETTE.cell : PALETTE.cellAlt;
    if (selected === i) fill = PALETTE.cellSel;
    else if (selVal && (user[i] === selVal || clues[i] === selVal)) fill = PALETTE.cellPeer;
    else if (selReg === cellReg[i]) fill = PALETTE.cellPeer;
    ctx.fillStyle = fill;
    ctx.fillRect(x, y, cell, cell);
    // Conflict tint.
    if (conflicts && conflicts.has(i)) {
      ctx.fillStyle = 'rgba(232, 85, 79, 0.4)';
      ctx.fillRect(x, y, cell, cell);
    }
    // Digit.
    const v = clues[i] || user[i];
    if (v) {
      ctx.fillStyle = clues[i] ? PALETTE.given : PALETTE.user;
      ctx.fillText(String(v), x + cell / 2, y + cell / 2 + 1);
    } else if (notes[i] && notes[i].size) {
      // Pencil marks: small digits in a 3x2-ish grid.
      ctx.font = ((cell * 0.26) | 0) + 'px monospace';
      ctx.fillStyle = PALETTE.noteText;
      const arr = [...notes[i]].sort();
      for (const d of arr) {
        const idx = d - 1;
        const nc = idx % 3, nr = (idx / 3) | 0;
        const nx = x + 4 + nc * ((cell - 8) / 3) + ((cell - 8) / 6);
        const ny = y + 4 + nr * ((cell - 8) / 2) + ((cell - 8) / 4);
        ctx.fillText(String(d), nx, ny);
      }
      ctx.font = 'bold ' + ((cell * 0.55) | 0) + 'px monospace';
    }
  }
  // Thin grid lines.
  ctx.strokeStyle = PALETTE.border;
  ctx.lineWidth = 1;
  for (let i = 0; i <= n; i++) {
    ctx.beginPath();
    ctx.moveTo(ox + i * cell + 0.5, oy);
    ctx.lineTo(ox + i * cell + 0.5, oy + total);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(ox,         oy + i * cell + 0.5);
    ctx.lineTo(ox + total, oy + i * cell + 0.5);
    ctx.stroke();
  }
  // Thick region borders.
  ctx.strokeStyle = PALETTE.regionEdge;
  ctx.lineWidth = 2;
  for (let i = 0; i < n * n; i++) {
    const r = (i / n) | 0, c = i % n;
    const x = ox + c * cell, y = oy + r * cell;
    const reg = cellReg[i];
    // Right edge.
    if (c === n - 1 || cellReg[r * n + c + 1] !== reg) {
      ctx.beginPath();
      ctx.moveTo(x + cell + 0.5, y);
      ctx.lineTo(x + cell + 0.5, y + cell);
      ctx.stroke();
    }
    // Bottom edge.
    if (r === n - 1 || cellReg[(r + 1) * n + c] !== reg) {
      ctx.beginPath();
      ctx.moveTo(x,        y + cell + 0.5);
      ctx.lineTo(x + cell, y + cell + 0.5);
      ctx.stroke();
    }
    // Top/left frame.
    if (r === 0) {
      ctx.beginPath();
      ctx.moveTo(x,        y + 0.5);
      ctx.lineTo(x + cell, y + 0.5);
      ctx.stroke();
    }
    if (c === 0) {
      ctx.beginPath();
      ctx.moveTo(x + 0.5, y);
      ctx.lineTo(x + 0.5, y + cell);
      ctx.stroke();
    }
  }
}

function drawNumberPad(ctx, n, padRect, lang, notesMode, padHits) {
  const { x, y, w, h } = padRect;
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(x, y, w, h);
  // 1..n buttons in a row, then ERASE / NOTES.
  const slots = n + 2;
  const bw = ((w - 8) / slots) | 0;
  const bh = h - 8;
  ctx.font = 'bold 16px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let i = 0; i < n; i++) {
    const bx = x + 4 + i * bw;
    const by = y + 4;
    ctx.fillStyle = notesMode ? '#3a4274' : '#54c47c';
    ctx.fillRect(bx, by, bw - 2, bh);
    ctx.fillStyle = PALETTE.hudText;
    ctx.fillText(String(i + 1), bx + (bw - 2) / 2, by + bh / 2 + 1);
  }
  // Erase.
  const ex = x + 4 + n * bw;
  ctx.fillStyle = '#a05050';
  ctx.fillRect(ex, y + 4, bw - 2, bh);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 10px monospace';
  ctx.fillText(t(lang, 'erase'), ex + (bw - 2) / 2, y + 4 + bh / 2 + 1);
  // Notes toggle.
  const nx = x + 4 + (n + 1) * bw;
  ctx.fillStyle = notesMode ? '#9a6cd8' : '#3a4274';
  ctx.fillRect(nx, y + 4, bw - 2, bh);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 10px monospace';
  ctx.fillText(t(lang, notesMode ? 'notesOn' : 'notesOff'), nx + (bw - 2) / 2, y + 4 + bh / 2 + 1);
  // Store hitboxes for the input layer.
  padHits.length = 0;
  for (let i = 0; i < n; i++) padHits.push({ kind: 'digit', v: i + 1, x: x + 4 + i * bw, y: y + 4, w: bw - 2, h: bh });
  padHits.push({ kind: 'erase', x: ex, y: y + 4, w: bw - 2, h: bh });
  padHits.push({ kind: 'notes', x: nx, y: y + 4, w: bw - 2, h: bh });
}

function drawHud(ctx, lang, levelIndex, mistakes, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText('L' + (levelIndex + 1), 8, 16);
  ctx.textAlign = 'center';
  const min = (elapsedSec / 60) | 0;
  const sec = (elapsedSec % 60) | 0;
  ctx.fillText(`${t(lang, 'timeStr')} ${min}:${sec.toString().padStart(2,'0')}`, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = mistakes ? PALETTE.conflict : PALETTE.hudText;
  ctx.fillText(`${t(lang, 'mistakes')}: ${mistakes}`, 352, 16);
}
