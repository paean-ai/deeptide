// Pixel Puck - rendering.

const COL = {
  table: '#cfdde6', tableEdge: '#7a8e96', tableLine: '#7faabd', halfShade: 'rgba(56,80,96,0.08)',
  goalRim: '#1a2a36', goalNet: 'rgba(0,0,0,0.5)',
  puck: '#1a2030', puckRim: '#7a8298',
  player: '#5fd36e', playerDark: '#2f6e3a',
  cpu: '#ff6e7a', cpuDark: '#7a3a44',
  serve: '#ffd86b',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1a2a36');
  g.addColorStop(1, '#06101a');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

function drawField(ctx, s) {
  // table
  ctx.fillStyle = COL.table;
  ctx.fillRect(0, FIELD_TOP, VW, FIELD_BOTTOM - FIELD_TOP);
  // halves shading
  ctx.fillStyle = COL.halfShade;
  ctx.fillRect(0, FIELD_TOP, VW, MID_Y - FIELD_TOP);
  // centre line
  ctx.fillStyle = COL.tableLine;
  ctx.fillRect(0, MID_Y - 1, VW, 2);
  // centre circle
  ctx.strokeStyle = COL.tableLine;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.arc(VW / 2, MID_Y, 36, 0, Math.PI * 2);
  ctx.stroke();
  // table edge
  ctx.strokeStyle = COL.tableEdge;
  ctx.lineWidth = 2;
  ctx.strokeRect(1, FIELD_TOP + 1, VW - 2, FIELD_BOTTOM - FIELD_TOP - 2);
  // top goal opening
  ctx.fillStyle = COL.goalNet;
  ctx.fillRect(GOAL_X1, FIELD_TOP - 14, GOAL_X2 - GOAL_X1, 16);
  ctx.fillStyle = COL.goalRim;
  ctx.fillRect(GOAL_X1 - 4, FIELD_TOP - 14, 4, 16);
  ctx.fillRect(GOAL_X2, FIELD_TOP - 14, 4, 16);
  // bottom goal opening
  ctx.fillStyle = COL.goalNet;
  ctx.fillRect(GOAL_X1, FIELD_BOTTOM - 2, GOAL_X2 - GOAL_X1, 16);
  ctx.fillStyle = COL.goalRim;
  ctx.fillRect(GOAL_X1 - 4, FIELD_BOTTOM - 2, 4, 16);
  ctx.fillRect(GOAL_X2, FIELD_BOTTOM - 2, 4, 16);
}

function drawMallet(ctx, x, y, light, dark) {
  ctx.fillStyle = dark;
  ctx.beginPath();
  ctx.arc(x, y + 2, PADDLE_R, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = light;
  ctx.beginPath();
  ctx.arc(x, y, PADDLE_R, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = dark;
  ctx.beginPath();
  ctx.arc(x, y, PADDLE_R * 0.45, 0, Math.PI * 2);
  ctx.fill();
}

function drawPuck(ctx, x, y) {
  ctx.fillStyle = COL.puckRim;
  ctx.beginPath();
  ctx.arc(x, y + 2, PUCK_R, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = COL.puck;
  ctx.beginPath();
  ctx.arc(x, y, PUCK_R, 0, Math.PI * 2);
  ctx.fill();
}

function drawWorld(ctx, s) {
  // big watermark score in centre
  ctx.fillStyle = 'rgba(7,15,22,0.07)';
  ctx.font = 'bold 64px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(s.cpuScore + ' : ' + s.playerScore, VW / 2, MID_Y);
  drawMallet(ctx, s.cpu.x, s.cpu.y, COL.cpu, COL.cpuDark);
  drawMallet(ctx, s.player.x, s.player.y, COL.player, COL.playerDark);
  drawPuck(ctx, s.puck.x, s.puck.y);
  if (s.serveT > 0 && !s.over) {
    ctx.fillStyle = COL.serve;
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(t('serve') + ' ' + Math.max(1, Math.ceil(s.serveT)),
      VW / 2, MID_Y + 60);
  }
}
