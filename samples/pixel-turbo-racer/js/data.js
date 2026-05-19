// Pixel Turbo Racer - content data: dimensions, road, traffic, tuning

const VW = 400;          // canvas logical width
const VH = 640;          // canvas logical height

const ROAD_X = 56;       // left edge of tarmac
const ROAD_W = 288;      // tarmac width
const LANES = 4;
const LANE_W = ROAD_W / LANES;

const PLAYER = {
  w: 34, h: 56, y: VH - 116,
  steer: 320,            // horizontal px/s
  startSpeed: 250,       // world scroll px/s
  maxSpeed: 720,
  accel: 6,              // speed gained per second of driving
  boostSpeed: 360,       // extra px/s while boosting
  boostTime: 1.6,        // seconds per nitro charge
  maxNitro: 3,
};

// Traffic cars drive forward too, so they only drift down slowly relative to
// the player. `rel` is their own forward speed.
const TRAFFIC_COLORS = ['#e0563f', '#3f8fe0', '#e0c63f', '#7a5fe0', '#3fc77a', '#e07ac0'];

const PICKUPS = {
  coin:  { color: '#ffd24d', r: 11, score: 25 },
  nitro: { color: '#7ad0ff', r: 13 },
};

const HAZARDS = {
  cone: { color: '#ff8a3c', w: 20, h: 24 },
  oil:  { color: '#1a1a22', w: 40, h: 22 },
};

// Difficulty curve by distance travelled (metres).
function diffOf(metres) {
  return {
    spawnGap: Math.max(0.42, 1.25 - metres / 4200),
    trafficRel: 60 + Math.min(170, metres / 26),
  };
}

function laneCenter(i) { return ROAD_X + LANE_W * (i + 0.5); }
