// Pixel Hill Glider - rolling-hills gliding game (Tiny-Wings style).
// Dive into downslopes to build speed, release at crests to launch.

const VW = 480, VH = 300;
const BIRD_X = 150;            // bird's fixed screen x; the world scrolls past

// physics tuning
const SLOPE_G   = 940;         // along-slope gravity while grounded
const DIVE_SLIDE = 540;        // extra grounded accel while tucking
const AIR_G     = 720;         // gravity while gliding
const DIVE_G    = 1560;        // gravity while tucking in the air
const GROUND_DRAG = 0.32;      // grounded speed bleed
const MIN_SPEED = 96;
const MAX_SPEED = 560;

// run economy
const LIGHT_DRAIN = 0.082;     // light lost per second
const ORB_LIGHT  = 0.15;       // light regained per orb

// terrain: a sum of sine octaves; amplitude grows slowly with distance.
function terrainY(wx) {
  const prog = Math.min(70, wx * 0.00055);
  const amp = 40 + prog;
  let y = VH * 0.60;
  y += Math.sin(wx * 0.0110) * amp;
  y += Math.sin(wx * 0.0231 + 1.3) * amp * 0.40;
  y += Math.sin(wx * 0.0049 + 0.6) * amp * 0.52;
  return y;
}
function terrainSlope(wx) {
  return (terrainY(wx + 1.6) - terrainY(wx - 1.6)) / 3.2;
}

// orbs sit above the hills; collecting them refuels light + scores.
const ORB_GAP = 196;
function orbPos(i) {
  const x = i * ORB_GAP + 300;
  return { x, y: terrainY(x) - 66 - 32 * Math.sin(i * 1.7) };
}
