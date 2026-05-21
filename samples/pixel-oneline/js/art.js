// Pixel-art rendering for Pixel One-Line. 360x480 world units.

const PALETTE = {
  bg:        '#0d1228',
  bgHi:      '#161d3a',
  edgeOff:   '#3a4274',
  edgeOn:    '#5fc0ff',
  edgeGlow:  'rgba(95,192,255,0.30)',
  node:      '#bfc7e6',
  nodeEdge:  '#070b1a',
  nodeStart: '#5fc06e',
  nodeCur:   '#ffd34a',
  nodeCurHi: '#fff0c8',
  reach:     '#ff8fd0',
  hud:       '#070b1a',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  star:      '#f8d34a',
  starOff:   '#3a4274',
  win:       '#5fc06e',
};

// The graph is drawn inside a square play area below the HUD.
const PLAY_X = 30, PLAY_Y = 70, PLAY_W = 300, PLAY_H = 300;
const NODE_R = 13;

function nodePos(lv, i) {
  const n = lv.nodes[i];
  return { x: PLAY_X + n.x * PLAY_W, y: PLAY_Y + n.y * PLAY_H };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.bgHi;
  for (let i = 0; i < 26; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawGraph(ctx, s, reachable) {
  const lv = s.lv;
  // Edges first, under the nodes.
  lv.edges.forEach((e, i) => {
    const a = nodePos(lv, e[0]), b = nodePos(lv, e[1]);
    const on = s.used[i];
    if (on) {
      // Glow underlay.
      ctx.strokeStyle = PALETTE.edgeGlow;
      ctx.lineWidth = 12;
      ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
    }
    ctx.strokeStyle = on ? PALETTE.edgeOn : PALETTE.edgeOff;
    ctx.lineWidth = on ? 6 : 4;
    ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
  });
  // Nodes.
  for (let i = 0; i < lv.nodes.length; i++) {
    const p = nodePos(lv, i);
    let fill = PALETTE.node;
    if (i === s.current) fill = PALETTE.nodeCur;
    else if (reachable && reachable.has(i)) fill = PALETTE.reach;
    ctx.fillStyle = PALETTE.nodeEdge;
    fillDisk(ctx, p.x, p.y, NODE_R + 2);
    ctx.fillStyle = fill;
    fillDisk(ctx, p.x, p.y, NODE_R);
    ctx.fillStyle = PALETTE.nodeCurHi;
    fillDisk(ctx, p.x - 3, p.y - 3, NODE_R * 0.4);
    // Pen marker on the current node.
    if (i === s.current) {
      ctx.fillStyle = PALETTE.nodeEdge;
      ctx.fillRect((p.x - 2) | 0, (p.y - 2) | 0, 4, 4);
    }
  }
}

function fillDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const w = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - w) | 0, (cy + dy) | 0, w * 2 + 1, 1);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'edges') + ' ' + progress(s) + '/' + s.lv.edges.length, VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudDim;
  ctx.fillText(best ? '★' : '', VW - 6, 16);
}

function drawStars(ctx, x, y, n, w = 14) {
  for (let i = 0; i < 3; i++) {
    ctx.fillStyle = i < n ? PALETTE.star : PALETTE.starOff;
    drawStar(ctx, x + i * (w + 4) + w / 2, y, w / 2);
  }
}
function drawStar(ctx, cx, cy, r) {
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const a = (Math.PI / 5) * i - Math.PI / 2;
    const rr = i % 2 === 0 ? r : r * 0.45;
    ctx.lineTo(cx + Math.cos(a) * rr, cy + Math.sin(a) * rr);
  }
  ctx.closePath();
  ctx.fill();
}
