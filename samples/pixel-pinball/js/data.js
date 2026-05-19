// Pixel Pinball - table layout and physics constants.

const VW = 320, VH = 440;
const BALL_R = 7;
const GRAVITY = 880;          // px/s^2 downward
const WALL_REST = 0.58;       // wall bounce energy retention
const BUMP_KICK = 240;        // round-bumper impulse
const SLING_KICK = 170;       // slingshot impulse
const MAX_SPEED = 920;
const DRAIN_Y = 432;          // ball below this (in the gap) is lost
const START_BALLS = 3;

// Table boundary segments [x1, y1, x2, y2].
const WALLS = [
  [12, 12, 308, 12],          // top
  [12, 12, 12, 356],          // left
  [308, 12, 308, 356],        // right
  [12, 356, 112, 420],        // lower-left funnel
  [308, 356, 208, 420],       // lower-right funnel
  [42, 286, 42, 332],         // left outlane guard post
  [278, 286, 278, 332],       // right outlane guard post
];

// Round pop bumpers.
const BUMPERS = [
  { x: 96, y: 134, r: 19 },
  { x: 224, y: 134, r: 19 },
  { x: 160, y: 84, r: 20 },
];

// Slingshot bumpers above the flippers.
const SLINGS = [
  { x: 74, y: 320, r: 15 },
  { x: 246, y: 320, r: 15 },
];

// Drop targets [x, y, w, h] - clear all for a bonus.
const TARGETS = [
  [66, 210, 30, 13],
  [104, 188, 30, 13],
  [186, 188, 30, 13],
  [224, 210, 30, 13],
];
const TARGET_BONUS = 3000;

// Flippers: pivot (px,py), length, rest & active angles (radians, y-down).
const FLIPPERS = [
  { side: 'L', px: 116, py: 398, len: 54, rest: 0.42, active: -0.62 },
  { side: 'R', px: 204, py: 398, len: 54, rest: Math.PI - 0.42, active: Math.PI + 0.62 },
];
const FLIP_THICK = 6;

// Ball launch tee (lower-right) and charge tuning.
const TEE = { x: 292, y: 300 };
const LAUNCH_MIN = 470, LAUNCH_RANGE = 430;
