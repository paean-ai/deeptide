// Pixel Peg Pop - physics constants and the peg-layout campaign.

const VW = 360, VH = 480;
const GRAVITY = 470;
const PEG_R = 9, BALL_R = 6;
const LAUNCH_X = VW / 2, LAUNCH_Y = 44, LAUNCH_SPEED = 300;

// Peg grid: odd rows are offset half a cell for a honeycomb bounce field.
const GRID_X0 = 30, GRID_Y0 = 108, GRID_DX = 30, GRID_DY = 30;

function pegX(col, row) { return GRID_X0 + col * GRID_DX + (row % 2) * (GRID_DX / 2); }
function pegY(row) { return GRID_Y0 + row * GRID_DY; }

// Layout grid chars: '.' empty, 'o' blue peg, 'x' orange (target) peg.
const LEVELS = [
  { name: ['First Drops', '初降'], balls: 9, grid: [
    '...........',
    '...ooooo...',
    '..oxoooxo..',
    '..ooooooo..',
    '..oxoooxo..',
    '...ooooo...',
    '...oxoxo...',
    '...........',
    '...........',
  ] },
  { name: ['Diamond', '钻石'], balls: 10, grid: [
    '.....o.....',
    '....ooo....',
    '...oxoxo...',
    '..ooooooo..',
    '.oxooxooxo.',
    '..ooooooo..',
    '...oxoxo...',
    '....ooo....',
    '.....o.....',
  ] },
  { name: ['Pillars', '石柱'], balls: 11, grid: [
    'o.o.o.o.o.o',
    'o.o.o.o.o.o',
    'x.o.x.o.x.o',
    'o.o.o.o.o.o',
    'o.x.o.x.o.x',
    'o.o.o.o.o.o',
    'x.o.x.o.o.o',
    'o.o.o.o.o.o',
    '...........',
  ] },
  { name: ['Citadel', '城塞'], balls: 15, grid: [
    'ooooooooooo',
    'o.........o',
    'o.xxxxxxx.o',
    'o.x.....x.o',
    'o.x..o..x.o',
    'o.x.....x.o',
    'o.xxxxxxx.o',
    'o.........o',
    'ooooooooooo',
  ] },
  { name: ['Cascade', '瀑落'], balls: 10, grid: [
    'oo.........',
    '.oox.......',
    '...oox.....',
    '.....oox...',
    '.......oox.',
    '.....oox...',
    '...oox.....',
    '.oox.......',
    'oo.........',
  ] },
  { name: ['Twin Rings', '双环'], balls: 13, grid: [
    '...ooooo...',
    '..o.....o..',
    '.o.oxxxo.o.',
    '.o.x...x.o.',
    '.o.x.o.x.o.',
    '.o.x...x.o.',
    '.o.oxxxo.o.',
    '..o.....o..',
    '...ooooo...',
  ] },
  { name: ['Bloom', '绽放'], balls: 11, grid: [
    '..oo...oo..',
    '.oooo.oooo.',
    'ooxoooooxoo',
    'ooooooooooo',
    '.ooxoooxoo.',
    '..ooooooo..',
    '...oxoxo...',
    '....ooo....',
    '.....x.....',
  ] },
  { name: ['Grid Storm', '风暴格'], balls: 16, grid: [
    'o.o.o.o.o.o',
    '.x.x.x.x.x.',
    'o.o.o.o.o.o',
    '.x.x.x.x.x.',
    'o.o.o.o.o.o',
    '.x.x.x.x.x.',
    'o.o.o.o.o.o',
    '.x.x.x.x.x.',
    'o.o.o.o.o.o',
  ] },
];
const LEVEL_COUNT = LEVELS.length;
