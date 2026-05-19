// Pixel Mart Manager - content data: palette, sprites, zones, upgrade tuning

const W = 960, H = 540;

const PAL = {
  ink: '#080a0f', edge: '#141922', shadow: '#050608',
  wall: '#1f2b3a', wallDark: '#111924', wallLight: '#34465c',
  floor: '#263447', floorAlt: '#2d3c51', floorDark: '#1a2533',
  cream: '#f2e6bd', gold: '#f2c14e', goldDark: '#a66a25',
  green: '#43d17a', greenDark: '#1f7f46', greenLight: '#8df09f',
  blue: '#2f80ed', blueLight: '#a9e8ff',
  red: '#e05243', redDark: '#8d2630',
  violet: '#b66cff', violetDark: '#5d328f',
  wood: '#8b5a32', woodDark: '#4e2e1d',
  skin: '#e9b57f', skinDark: '#a96f4a',
  white: '#f3f7ff', metal: '#8b93a1',
};

const SPRITE_MAP = {
  k: PAL.ink, e: PAL.edge, w: PAL.white, y: PAL.gold, Y: PAL.goldDark,
  g: PAL.green, G: PAL.greenDark, l: PAL.greenLight, b: PAL.blue, B: PAL.blueLight,
  r: PAL.red, R: PAL.redDark, v: PAL.violet, V: PAL.violetDark,
  n: PAL.wood, N: PAL.woodDark, s: PAL.skin, S: PAL.skinDark, m: PAL.metal, c: PAL.cream,
};

const SPRITES = {
  player: [
    '...kkkk...', '..kssssk..', '..kswwsk..', '.kyyyyyk.', 'kyggggyk',
    'kyggggyk', '.kyNNyk.', '..kn.nk..', '.kn...nk.',
  ],
  helper: [
    '...kkkk...', '..kssssk..', '..kswwsk..', '.kgggggk.', 'kgbbbbgk',
    'kggbbggk', '.kgNNgk.', '..kn.nk..', '.kn...nk.',
  ],
  customerA: [
    '...kkkk...', '..kssssk..', '..kswwsk..', '.kvvvvvk.', 'kvyyyyvk',
    'kvyyyyvk', '.kvNNvk.', '..kn.nk..', '.kn...nk.',
  ],
  customerB: [
    '...kkkk...', '..kssssk..', '..kswwsk..', '.kbbbbbk.', 'kbccccbk',
    'kbccccbk', '.kbNNbk.', '..kn.nk..', '.kn...nk.',
  ],
  customerC: [
    '...kkkk...', '..kssssk..', '..kswwsk..', '.krrrrrk.', 'krllllrk',
    'krllllrk', '.krNNrk.', '..kn.nk..', '.kn...nk.',
  ],
  banana: ['..yy.', '.yYy.', '.yy..', 'yY...'],
  crate: ['NNNNNN', 'NnYYnN', 'NYyyYN', 'NnYYnN', 'NNNNNN'],
  register: ['mmmmmmmm', 'mBBBBBBm', 'mBkkkkBm', 'mmmmmmmm', 'NNNNNNNN', 'NyyyyyyN'],
};

const ZONES = {
  field:    { x: 80,  y: 125, w: 160, h: 130, labelKey: 'zoneField' },
  shelf:    { x: 430, y: 175, w: 125, h: 105, labelKey: 'zoneShelf' },
  register: { x: 705, y: 310, w: 110, h: 75,  labelKey: 'zoneRegister' },
};

// Upgrade cost formulas (level -> cost).
const COSTS = {
  shelf:     lvl => 40 + lvl * 35,
  helper:    lvl => 90 + lvl * 120,
  expand:    lvl => 160 + lvl * 180,
  marketing: lvl => 70 + lvl * 95,
};

const OFFLINE_CAP_SECONDS = 8 * 3600;

function shelfCap(shelfLevel) { return 5 + shelfLevel * 3; }
