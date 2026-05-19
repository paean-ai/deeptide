// Pixel Quest - battle layout and rendering.

const UNIT_COL = {
  knight: '#6f9fe0', mage: '#c77dff', cleric: '#5fd3a0',
  Slime: '#7fce5f', Goblin: '#d09a4a', Wolf: '#9aa0ad',
  Ogre: '#c5705a', Dragon: '#e0554f',
};
const MENU_BTN = [
  { key: 'attack', x: 16,  y: 390, w: 104, h: 44 },
  { key: 'skill',  x: 128, y: 390, w: 104, h: 44 },
  { key: 'defend', x: 240, y: 390, w: 104, h: 44 },
];

function unitColor(u) {
  return UNIT_COL[u.key] || UNIT_COL[u.name[0]] || '#888';
}
function enemyRect(i, count) {
  const slot = VW / count;
  return { x: Math.round(i * slot + slot / 2 - 30), y: 80, w: 60, h: 56 };
}
function heroRect(i) {
  return { x: 32 + i * 110, y: 222, w: 56, h: 56 };
}
function unitRect(s, u) {
  return u.side === 'enemy' ? enemyRect(u.slot, s.enemies.length) : heroRect(u.slot);
}

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#2a2440');
  g.addColorStop(0.6, '#1a1730');
  g.addColorStop(1, '#0c0a18');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
  // ground band
  ctx.fillStyle = 'rgba(120,90,160,0.14)';
  ctx.fillRect(0, 158, VW, 2);
  ctx.fillRect(0, 296, VW, 2);
}

function drawBattle(ctx, s, ui) {
  drawBackground(ctx);
  const cur = ui.current;
  for (const e of s.enemies) {
    drawUnit(ctx, enemyRect(e.slot, s.enemies.length), e,
      cur === e, ui.pickable && ui.pickable.includes(e), ui.pulse);
  }
  for (const h of s.heroes) {
    drawUnit(ctx, heroRect(h.slot), h,
      cur === h, ui.pickable && ui.pickable.includes(h), ui.pulse);
  }
  // turn banner
  ctx.fillStyle = '#ffe07a';
  ctx.font = 'bold 13px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(ui.banner || '', VW / 2, 178);
  // menu or target prompt
  if (ui.mode === 'menu') drawMenu(ctx, s, ui);
  else if (ui.mode === 'enemy' || ui.mode === 'ally') {
    ctx.fillStyle = '#9fb4d8';
    ctx.font = 'bold 13px monospace';
    ctx.fillText(ui.mode === 'enemy' ? t('pickEnemy') : t('pickAlly'), VW / 2, 412);
  }
}

function drawUnit(ctx, r, u, isCurrent, pickable, pulse) {
  const dead = u.hp <= 0;
  const cx = r.x + r.w / 2;
  if (isCurrent && !dead) {
    ctx.fillStyle = '#ffe07a';
    ctx.beginPath();
    ctx.moveTo(cx, r.y - 14);
    ctx.lineTo(cx - 6, r.y - 22);
    ctx.lineTo(cx + 6, r.y - 22);
    ctx.closePath();
    ctx.fill();
  }
  if (pickable && !dead) {
    const a = 0.4 + 0.4 * Math.sin(pulse * 4);
    ctx.strokeStyle = `rgba(255,224,122,${a})`;
    ctx.lineWidth = 3;
    ctx.strokeRect(r.x - 3, r.y - 3, r.w + 6, r.h + 6);
  }
  // body
  ctx.globalAlpha = dead ? 0.32 : 1;
  ctx.fillStyle = unitColor(u);
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = 'rgba(0,0,0,0.28)';
  ctx.fillRect(r.x, r.y + r.h - 8, r.w, 8);
  // eyes
  ctx.fillStyle = '#10101a';
  const ey = r.y + r.h * 0.34, es = Math.max(4, r.w * 0.13);
  ctx.fillRect(r.x + r.w * 0.26, ey, es, es);
  ctx.fillRect(r.x + r.w * 0.61, ey, es, es);
  ctx.globalAlpha = 1;
  if (dead) {
    ctx.strokeStyle = '#ff6e7a';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(r.x + 6, r.y + 6); ctx.lineTo(r.x + r.w - 6, r.y + r.h - 6);
    ctx.moveTo(r.x + r.w - 6, r.y + 6); ctx.lineTo(r.x + 6, r.y + r.h - 6);
    ctx.stroke();
    return;
  }
  // name
  ctx.fillStyle = '#e8ecf6';
  ctx.font = 'bold 9px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'bottom';
  ctx.fillText(u.name[lang === 'en' ? 0 : 1], cx, r.y - 3);
  // hp bar
  bar(ctx, r.x, r.y + r.h + 3, r.w, 5, u.hp / u.maxhp, '#5fd36e', '#1a1f14');
  // mp bar (heroes)
  if (u.side === 'hero') {
    bar(ctx, r.x, r.y + r.h + 10, r.w, 4, u.mp / u.maxmp, '#5fa8e8', '#141a26');
  }
}

function bar(ctx, x, y, w, h, frac, fg, bg) {
  ctx.fillStyle = bg;
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = fg;
  ctx.fillRect(x, y, Math.max(0, Math.min(1, frac)) * w, h);
  ctx.strokeStyle = 'rgba(0,0,0,0.5)';
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
}

function drawMenu(ctx, s, ui) {
  for (const b of MENU_BTN) {
    let label = t(b.key), enabled = true;
    if (b.key === 'skill') {
      const u = ui.current;
      label = u && u.skill && SKILLS[u.skill] ? SKILLS[u.skill].name[lang === 'en' ? 0 : 1] : t('skill');
      enabled = !!(u && canCast(u));
    }
    ctx.fillStyle = enabled ? '#3b3358' : '#24222e';
    ctx.fillRect(b.x, b.y, b.w, b.h);
    ctx.strokeStyle = enabled ? '#ffe07a' : '#3a3a48';
    ctx.lineWidth = 2;
    ctx.strokeRect(b.x + 1, b.y + 1, b.w - 2, b.h - 2);
    ctx.fillStyle = enabled ? '#ffe9b0' : '#6a6a78';
    ctx.font = 'bold 13px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(label, b.x + b.w / 2, b.y + b.h / 2 - 5);
    if (b.key === 'skill' && ui.current && ui.current.skill) {
      ctx.fillStyle = enabled ? '#9fb4d8' : '#55555f';
      ctx.font = '9px monospace';
      ctx.fillText(SKILLS[ui.current.skill].mp + ' MP', b.x + b.w / 2, b.y + b.h - 9);
    }
  }
}
