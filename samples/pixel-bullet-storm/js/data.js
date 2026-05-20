// Pixel Bullet Storm - enemy, boss and wave definitions.

const VW = 360, VH = 480;

// Enemy archetypes. pattern names are resolved by the emitter in game.js.
const ENEMY_TYPES = {
  drone:   { hp: 3,  r: 11, score: 100, color: '#5fd17a', pattern: 'aimed',  fireEvery: 1.15, speed: 64 },
  weaver:  { hp: 5,  r: 12, score: 220, color: '#ffae4a', pattern: 'fan',    fireEvery: 1.30, speed: 74 },
  turret:  { hp: 7,  r: 13, score: 260, color: '#5aa9ff', pattern: 'ring',   fireEvery: 1.70, speed: 42 },
  spinner: { hp: 9,  r: 13, score: 320, color: '#b98bff', pattern: 'spiral', fireEvery: 0.17, speed: 36 },
  // Stationary "lattice" drops a horizontal row of 7 bullets straight down —
  // a moving wall the player must side-step. Higher score for the time it
  // takes to deal with.
  lattice: { hp: 12, r: 14, score: 420, color: '#5fe8d0', pattern: 'wall',   fireEvery: 2.10, speed: 28 },
};

const BOSS = { r: 30, color: '#ff6b6b', score: 3000 };

// Enemy bullet speed rises gently with the wave number.
function bulletSpeed(wave) {
  return Math.min(212, 102 + wave * 5);
}

// A boss appears every 5th wave; otherwise a staggered squad of enemies.
function wavePlan(wave) {
  if (wave % 5 === 0) {
    const bossNum = wave / 5;
    return { boss: true, hp: 150 + bossNum * 95 };
  }
  const count = Math.min(16, 4 + Math.floor(wave * 0.85));
  const pool = ['drone'];
  if (wave >= 2) pool.push('weaver');
  if (wave >= 3) pool.push('turret');
  if (wave >= 4) pool.push('spinner');
  if (wave >= 6) pool.push('lattice');
  const gap = Math.max(0.26, 0.6 - wave * 0.02);
  const spawns = [];
  for (let i = 0; i < count; i++) {
    spawns.push({
      type: pool[(Math.random() * pool.length) | 0],
      x: 42 + Math.random() * (VW - 84),
      targetY: 52 + Math.random() * 120,
      delay: i * gap,
    });
  }
  return { boss: false, spawns };
}
