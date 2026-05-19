// Pixel Tower Stack - content data: dimensions and tuning

const FW = 360;          // canvas width
const FH = 600;          // canvas height
const BH = 30;           // block height
const W0 = 220;          // starting block width
const BASE_LEVELS = 8;   // how many levels sit visible above the camera base

const PERFECT_TOL = 4;   // px alignment tolerance for a perfect stack
const REGROW = 6;        // width regained per perfect once the combo is hot
const REGROW_AT = 8;     // perfects needed before width starts regrowing

// Horizontal slide speed of the moving block, by tower height.
function blockSpeed(height) {
  return Math.min(440, 132 + height * 7);
}

// Hue cycles up the tower for a smooth rainbow gradient.
function hueFor(level) {
  return (level * 9 + 190) % 360;
}
