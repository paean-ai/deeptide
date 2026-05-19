// Pixel Brick Knight - layout constants, floor generation, upgrade pool.

const VW = 360, VH = 480;

const COLS = 9;
const BRICK_W = 37, BRICK_H = 16;
const BRICK_X0 = Math.round((VW - COLS * BRICK_W) / 2);
const BRICK_Y0 = 66;
const BRICK_COLORS = ['#e8554f', '#ef9b3e', '#f2cf3f', '#5fc06e', '#4a9be8', '#9a6cd8'];

// Build the brick list for a floor. Every 5th floor is a boss floor.
function genFloor(n) {
  const bricks = [];
  const baseHp = 1 + Math.floor((n - 1) / 3);
  if (n % 5 === 0) {
    const bw = BRICK_W * 5 + 6, bh = BRICK_H * 2 + 4;
    const bhp = 28 + n * 7;
    bricks.push({
      x: Math.round((VW - bw) / 2), y: BRICK_Y0 + BRICK_H, w: bw, h: bh,
      hp: bhp, maxhp: bhp, kind: 'boss', gold: n * 18,
    });
    for (let c = 0; c < COLS; c++) {
      if (Math.random() < 0.35) continue;
      const hp = baseHp + 1;
      bricks.push({
        x: BRICK_X0 + c * BRICK_W, y: BRICK_Y0 + BRICK_H * 3 + 6,
        w: BRICK_W - 2, h: BRICK_H - 2, hp, maxhp: hp, kind: 'brick', gold: 3 + n,
      });
    }
    return bricks;
  }
  const rows = Math.min(8, 3 + Math.floor(n / 2));
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < COLS; c++) {
      if (Math.random() < 0.16) continue;
      const hp = baseHp + (r < 2 ? 1 : 0);
      bricks.push({
        x: BRICK_X0 + c * BRICK_W, y: BRICK_Y0 + r * BRICK_H,
        w: BRICK_W - 2, h: BRICK_H - 2, hp, maxhp: hp, kind: 'brick', gold: 3 + n,
      });
    }
  }
  return bricks;
}

// Roguelite upgrade pool. `once: true` powers are offered only once.
const UPGRADES = [
  { id: 'power',    name: ['Heavy Strike', '重击'],   desc: ['+1 ball damage', '球伤害 +1'],
    apply: r => { r.dmg += 1; } },
  { id: 'wide',     name: ['Broad Guard', '宽盾'],    desc: ['Wider paddle', '加宽挡板'],
    apply: r => { r.paddleW += 14; } },
  { id: 'multiball',name: ['Split Orb', '分裂宝珠'],  desc: ['+1 ball each floor', '每层多 1 个球'],
    apply: r => { r.ballCount += 1; } },
  { id: 'life',     name: ['Second Wind', '回气'],    desc: ['+1 life', '生命 +1'],
    apply: r => { r.lives += 1; } },
  { id: 'pierce',   name: ['Phase Ball', '穿透之球'], desc: ['Balls pierce bricks', '球穿透砖块'],
    apply: r => { r.pierce = true; }, once: true },
  { id: 'control',  name: ['Steady Hand', '稳手'],    desc: ['Slower, tamer balls', '球速更慢更稳'],
    apply: r => { r.ballSpeed *= 0.9; } },
  { id: 'fortune',  name: ['Gilded Touch', '点金术'], desc: ['+60% gold', '金币 +60%'],
    apply: r => { r.goldMult += 0.6; } },
];
