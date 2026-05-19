// Pixel Sky Climber - content data: dimensions, physics, platform tuning

const VW = 400;
const VH = 640;

const PHYS = {
  gravity: 1650,        // px/s^2
  jump: 760,            // bounce velocity (up)
  springMul: 1.7,       // spring pad boost
  jetMul: 1.0,
  move: 340,            // horizontal speed px/s
  scrollAt: 0.42,       // player rises above this fraction -> world scrolls
};

const PLAT_W = 64;
const PLAT_H = 14;

// Platform types. weight is the base spawn share; it shifts with height.
const PLAT_TYPES = {
  normal:    { color: '#5fcf7a', weight: 60 },
  moving:    { color: '#5fa8e0', weight: 16 },
  breakable: { color: '#b07a4a', weight: 16 },
  spring:    { color: '#f4c85a', weight: 8 },
};

// Vertical gap between platforms grows with height (harder higher up).
function gapFor(height) {
  return Math.min(128, 66 + height / 380);
}

// How likely a platform spawns a coin / monster, by height.
function coinChance(height) { return 0.22; }
function monsterChance(height) { return Math.min(0.16, height / 90000); }
function jetChance(height) { return height > 4000 ? 0.018 : 0; }
