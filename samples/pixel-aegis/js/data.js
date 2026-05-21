// Pixel Aegis - rotate-the-shield core defence. Pure real-time logic.
//
// A core sits at the centre, ringed by shooters that fire straight at it.
// You drag a shield arc around the core; a shot that crosses the shield is
// blocked and the blow rebounds onto the shooter that fired it. A pulse
// flashes the shield full-circle on a cooldown. Clear every shooter.

const VW = 360, VH = 480;

const CORE_X = 180, CORE_Y = 250;
const CORE_R = 20;            // a shot reaching this radius hits the core
const SHIELD_R = 92;          // the shield orbits here
const PERIM_R = 165;          // shooters sit here, shots start here
const CORE_HP = 6;
const SHIELD_SPEED = 5.5;     // radians/sec the shield can rotate
const SHIELD_HALF = 0.78;     // half-width of the shield arc (radians)
const PROJ_SPEED = 44;        // px/sec a shot travels inward
const PULSE_DUR = 0.32;       // seconds the pulse keeps the shield full-circle
const PULSE_CD = 7.0;         // pulse cooldown
const BURST_GAP = 0.24;       // seconds between shots of a burst

const TYPE_HP = { gunner: 2, twin: 3, burst: 2 };

// Each stage: a list of shooters { a: angle(deg), t: type, p: fire period }.
// Hand-authored, verified clearable by a bot in the test.
const STAGES = [
  { name: ['Picket', '哨位'], shooters: [
    { a: 90, t: 'gunner', p: 5.0 }, { a: 210, t: 'gunner', p: 5.0 }, { a: 330, t: 'gunner', p: 5.0 },
  ] },
  { name: ['Cordon', '警戒线'], shooters: [
    { a: 55, t: 'gunner', p: 4.6 }, { a: 130, t: 'gunner', p: 4.6 }, { a: 200, t: 'twin', p: 5.0 },
    { a: 270, t: 'gunner', p: 4.6 }, { a: 340, t: 'gunner', p: 4.6 },
  ] },
  { name: ['Volley', '齐射'], shooters: [
    { a: 45, t: 'gunner', p: 4.4 }, { a: 110, t: 'burst', p: 6.0 }, { a: 175, t: 'gunner', p: 4.4 },
    { a: 245, t: 'gunner', p: 4.4 }, { a: 310, t: 'twin', p: 4.8 },
  ] },
  { name: ['Crossfire', '交叉火'], shooters: [
    { a: 35, t: 'gunner', p: 4.2 }, { a: 95, t: 'twin', p: 4.8 }, { a: 150, t: 'gunner', p: 4.2 },
    { a: 210, t: 'burst', p: 5.6 }, { a: 270, t: 'gunner', p: 4.2 }, { a: 330, t: 'twin', p: 4.8 },
  ] },
  { name: ['Barrage', '弹幕'], shooters: [
    { a: 30, t: 'gunner', p: 4.0 }, { a: 80, t: 'burst', p: 5.4 }, { a: 130, t: 'gunner', p: 4.0 },
    { a: 180, t: 'twin', p: 4.6 }, { a: 230, t: 'gunner', p: 4.0 }, { a: 285, t: 'gunner', p: 4.0 },
    { a: 335, t: 'twin', p: 4.6 },
  ] },
  { name: ['Onslaught', '猛攻'], shooters: [
    { a: 25, t: 'gunner', p: 4.2 }, { a: 70, t: 'twin', p: 4.8 }, { a: 115, t: 'gunner', p: 4.2 },
    { a: 160, t: 'burst', p: 5.6 }, { a: 205, t: 'gunner', p: 4.2 }, { a: 250, t: 'twin', p: 4.8 },
    { a: 295, t: 'gunner', p: 4.2 }, { a: 340, t: 'gunner', p: 4.2 },
  ] },
];

const STAGE_COUNT = STAGES.length;

function angNorm(a) {                       // wrap to (-PI, PI]
  while (a > Math.PI) a -= Math.PI * 2;
  while (a <= -Math.PI) a += Math.PI * 2;
  return a;
}
function angDist(a, b) { return Math.abs(angNorm(a - b)); }

function newGame(stageIndex) {
  const st = STAGES[stageIndex];
  return {
    stageIndex, stage: st,
    coreHp: CORE_HP,
    shieldAngle: -Math.PI / 2,
    shieldTarget: -Math.PI / 2,
    shooters: st.shooters.map((sh, id) => ({
      id, angle: sh.a * Math.PI / 180, type: sh.t, hp: TYPE_HP[sh.t],
      period: sh.p, timer: 1.2 + id * 0.85,        // staggered first shots
      burstLeft: 0, burstTimer: 0,
    })),
    projectiles: [],          // { angle, r, firerId, resolved }
    pulseCd: 0, pulseT: 0,
    time: 0, blocks: 0, hitsTaken: 0,
    over: false, won: false,
    fx: [],                   // transient: { kind, angle, t }
  };
}

function setAim(s, angle) { s.shieldTarget = angNorm(angle); }
function pulse(s) {
  if (s.over || s.pulseCd > 0) return false;
  s.pulseT = PULSE_DUR;
  s.pulseCd = PULSE_CD;
  return true;
}

function fireShot(s, sh) {
  s.projectiles.push({ angle: sh.angle, r: PERIM_R, firerId: sh.id, resolved: false });
}
function shooterById(s, id) {
  for (const sh of s.shooters) if (sh.id === id) return sh;
  return null;
}

function tick(s, dt) {
  if (s.over) return;
  s.time += dt;
  if (s.pulseCd > 0) s.pulseCd = Math.max(0, s.pulseCd - dt);
  if (s.pulseT > 0) s.pulseT = Math.max(0, s.pulseT - dt);
  for (let i = s.fx.length - 1; i >= 0; i--) { s.fx[i].t -= dt; if (s.fx[i].t <= 0) s.fx.splice(i, 1); }

  // rotate the shield toward its target along the shortest arc
  const d = angNorm(s.shieldTarget - s.shieldAngle);
  const step = SHIELD_SPEED * dt;
  s.shieldAngle = angNorm(Math.abs(d) <= step ? s.shieldTarget : s.shieldAngle + Math.sign(d) * step);

  // shooters fire
  for (const sh of s.shooters) {
    if (sh.burstLeft > 0) {
      sh.burstTimer -= dt;
      if (sh.burstTimer <= 0) { fireShot(s, sh); sh.burstLeft--; sh.burstTimer = BURST_GAP; }
    }
    sh.timer -= dt;
    if (sh.timer <= 0) {
      sh.timer = sh.period;
      fireShot(s, sh);
      if (sh.type === 'burst') { sh.burstLeft = 2; sh.burstTimer = BURST_GAP; }
    }
  }

  // advance projectiles, resolve at the shield and at the core
  const covered = (ang) => s.pulseT > 0 || angDist(ang, s.shieldAngle) <= SHIELD_HALF;
  for (let i = s.projectiles.length - 1; i >= 0; i--) {
    const p = s.projectiles[i];
    p.r -= PROJ_SPEED * dt;
    if (!p.resolved && p.r <= SHIELD_R) {
      p.resolved = true;
      if (covered(p.angle)) {
        s.blocks++;
        s.fx.push({ kind: 'block', angle: p.angle, t: 0.3 });
        const sh = shooterById(s, p.firerId);
        if (sh) {
          sh.hp--;
          if (sh.hp <= 0) {
            s.shooters = s.shooters.filter(x => x.id !== sh.id);
            s.fx.push({ kind: 'kill', angle: p.angle, t: 0.4 });
          }
        }
        s.projectiles.splice(i, 1);
        continue;
      }
    }
    if (p.r <= CORE_R) {
      s.coreHp--;
      s.hitsTaken++;
      s.fx.push({ kind: 'hurt', angle: p.angle, t: 0.4 });
      s.projectiles.splice(i, 1);
      if (s.coreHp <= 0) { s.coreHp = 0; s.over = true; s.won = false; return; }
    }
  }

  if (s.shooters.length === 0) {
    s.over = true; s.won = true;
    s.projectiles = [];
  }
}

function stars(s) {
  if (s.coreHp >= CORE_HP) return 3;
  if (s.coreHp >= 3) return 2;
  return 1;
}
