// === VOID DESCENT - Pixel art system ===

const VD_PALETTE = {
  ink: '#090b11',
  edge: '#161b26',
  white: '#f3f7ff',
  gold: '#f2c14e',
  red: '#e05243',
  redDark: '#8d2630',
  blue: '#2f80ed',
  blueLight: '#a9e8ff',
  green: '#43d17a',
  greenDark: '#1f8f4d',
  violet: '#b66cff',
  violetDark: '#5d328f',
  stone: '#4f5664',
  stoneLight: '#8b93a1',
  stoneDark: '#252b36',
  sand: '#a58d5a',
  sandLight: '#d5bb77',
  ice: '#62a8c8',
  fire: '#ff8a3d',
};

const VD_THEMES = {
  Void: {
    wall: ['#25213d', '#39305c', '#151426'],
    floor: ['#151722', '#222638', '#0d0f17'],
    fog: '#070910',
    accent: '#b66cff',
  },
  Depths: {
    wall: ['#1f3a27', '#396a42', '#102216'],
    floor: ['#132017', '#243a26', '#09120d'],
    fog: '#050a07',
    accent: '#43d17a',
  },
  Magma: {
    wall: ['#46231f', '#794034', '#1e1110'],
    floor: ['#221414', '#3a211b', '#100809'],
    fog: '#0d0606',
    accent: '#ff8a3d',
  },
  Frost: {
    wall: ['#234154', '#44708a', '#101d2a'],
    floor: ['#141d2a', '#24354a', '#09101a'],
    fog: '#070b12',
    accent: '#a9e8ff',
  },
  'Deep Void': {
    wall: ['#342246', '#5a3475', '#191124'],
    floor: ['#1b1325', '#2f2140', '#0c0813'],
    fog: '#08050d',
    accent: '#d081ff',
  },
  Halls: {
    wall: ['#3a3927', '#69643c', '#1d1c12'],
    floor: ['#1e1e15', '#34331f', '#0d0d08'],
    fog: '#080805',
    accent: '#f2c14e',
  },
};

const VD_SPRITES = {
  player: [
    '..kkkk..',
    '.kssysk.',
    '.ksyyk..',
    'kbbbbbk.',
    'kbrbbbk.',
    'kbwwwbk.',
    '.kbkkbk.',
    '.ks..sk.',
  ],
  slime: [
    '........',
    '..kkkk..',
    '.kggggk.',
    'kggllggk',
    'kggkkggk',
    '.kggggk.',
    '..kkkk..',
    '........',
  ],
  wraith: [
    '..kkkk..',
    '.kvvvvk.',
    'kvkkkkvk',
    'kvvvvvvk',
    '.kvvvvk.',
    'kvvkkvvk',
    '.k....k.',
    '........',
  ],
  shade: [
    '...kk...',
    '..khhk..',
    '.khkkhk.',
    'khhhhhhk',
    '.khhhhk.',
    '..khhk..',
    '.kh..hk.',
    '........',
  ],
  golem: [
    '.kkkkkk.',
    'kffffffk',
    'kfkkffk.',
    'kffffffk',
    '.kfnnfk.',
    '.kffffk.',
    'kf....fk',
    '........',
  ],
  void_lord: [
    'k..kk..k',
    'kdkddkdk',
    '.kddddk.',
    'kdkwwkdk',
    'kddddddk',
    '.kdrrdk.',
    'kdd..ddk',
    '........',
  ],
  bat: [
    'k......k',
    'kvk..kvk',
    'kvvkkvvk',
    '.kvvvvk.',
    '.kvhhvk.',
    '..kvvk..',
    '..k..k..',
    '........',
  ],
  revenant: [
    '..kkkk..',
    '.kwwwwk.',
    '.kwrrwk.',
    '.kwwwwk.',
    'kskwwksk',
    '.kswwsk.',
    '.ks..sk.',
    '........',
  ],
  potion: [
    '...ww...',
    '..krrk..',
    '.krrrrk.',
    '.krrrrk.',
    '..krrk..',
    '...kk...',
  ],
  big_potion: [
    '..kwwk..',
    '.krrrrk.',
    'krrrrrrk',
    'krrrrrrk',
    '.krrrrk.',
    '..kkkk..',
  ],
  atk_scroll: [
    '.kkkkk..',
    'kfffffk.',
    'kfkkffk.',
    'kfffkkk.',
    'kfffffk.',
    '.kkkkk..',
  ],
  def_scroll: [
    '.kkkkk..',
    'kbbbbb k',
    'kbkkbbk.',
    'kbbbkkk.',
    'kbbbbbk.',
    '.kkkkk..',
  ].map(row => row.replaceAll(' ', '')),
  essence: [
    '...v...',
    '..vvv..',
    '.vvwvv.',
    'vvwwwvv',
    '.vvwvv.',
    '..vvv..',
    '...v...',
  ],
  stairs: [
    'kkkkkkkk',
    'kvvvvvvk',
    'kvkkkvvk',
    'kvvkkvvk',
    'kvvvkvvk',
    'kvvvvvvk',
    'kkkkkkkk',
  ],
};

const VD_COLOR_MAP = {
  k: VD_PALETTE.ink,
  w: VD_PALETTE.white,
  y: VD_PALETTE.gold,
  s: VD_PALETTE.stoneLight,
  b: VD_PALETTE.blue,
  r: VD_PALETTE.red,
  g: VD_PALETTE.green,
  l: '#b5f47a',
  v: VD_PALETTE.violet,
  h: VD_PALETTE.blueLight,
  f: VD_PALETTE.stone,
  n: '#8b5a32',
  d: VD_PALETTE.redDark,
};

const VD_GLYPHS = {
  vitality: ['..1..', '.111.', '11111', '.111.', '..1..'],
  power: ['..1..', '..1..', '.111.', '1.1..', '..1..'],
  armor: ['11111', '1...1', '1.1.1', '.1.1.', '..1..'],
  crit: ['..1..', '.111.', '11111', '.111.', '..1..'],
  lifesteal: ['..1..', '.111.', '.111.', '11111', '.1.1.'],
  thorns: ['1.1.1', '.111.', '..1..', '.111.', '1...1'],
  regen: ['..1..', '.111.', '11111', '..1..', '..1..'],
  swift: ['111..', '..111', '.111.', '111..', '..111'],
  doublestrike: ['1.1.1', '.111.', '..1..', '.111.', '1.1.1'],
  dodge: ['.111.', '1...1', '..11.', '.1...', '11111'],
  berserk: ['..1..', '.111.', '1111.', '.1111', '..1..'],
  shield: ['11111', '1...1', '1.1.1', '.111.', '..1..'],
  scout: ['.111.', '1...1', '1.1.1', '1...1', '.111.'],
  leech: ['.111.', '1.1.1', '1...1', '.111.', '..1..'],
  assassin: ['...1.', '..1..', '.111.', '1.1..', '..1..'],
  phoenix: ['..1..', '.111.', '1.1.1', '..1..', '.1.1.'],
  cleave: ['.111.', '1..1.', '..1..', '.1..1', '.111.'],
  alchemist: ['.111.', '..1..', '.111.', '11111', '.111.'],
  poison: ['.111.', '1.1.1', '1...1', '.111.', '.1.1.'],
  fortune: ['.1.1.', '11111', '.111.', '..1..', '.1.1.'],
  glasscanon: ['..1..', '.111.', '11111', '.111.', '..1..'],
  juggernaut: ['11111', '11.11', '.111.', '.111.', '1...1'],
  default: ['..1..', '.111.', '11111', '.111.', '..1..'],
};

function vdHash(x, y, seed = 0) {
  const h = Math.sin(x * 12.9898 + y * 78.233 + seed * 37.719) * 43758.5453;
  return h - Math.floor(h);
}

function vdTheme(name) {
  return VD_THEMES[name] || VD_THEMES.Void;
}

function vdDrawSprite(ctx, sprite, x, y, size, alpha = 1) {
  const rows = sprite.length;
  const cols = sprite[0].length;
  const px = Math.max(1, Math.floor(size / Math.max(cols, rows)));
  const ox = Math.round(x + (size - cols * px) / 2);
  const oy = Math.round(y + (size - rows * px) / 2);
  ctx.save();
  ctx.globalAlpha = alpha;
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const ch = sprite[row][col];
      if (ch !== '.' && VD_COLOR_MAP[ch]) {
        ctx.fillStyle = VD_COLOR_MAP[ch];
        ctx.fillRect(ox + col * px, oy + row * px, px, px);
      }
    }
  }
  ctx.restore();
}

function vdDrawShadow(ctx, x, y, size, alpha = 0.28) {
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.fillStyle = VD_PALETTE.ink;
  const w = Math.floor(size * 0.62);
  const h = Math.max(2, Math.floor(size * 0.14));
  ctx.fillRect(Math.round(x + (size - w) / 2), Math.round(y + size * 0.74), w, h);
  ctx.restore();
}

function vdDrawTile(ctx, tile, x, y, size, theme, gx, gy, visible) {
  if (!visible) {
    ctx.fillStyle = theme.fog;
    ctx.fillRect(x, y, size, size);
    if (vdHash(gx, gy, 91) > 0.92) {
      ctx.fillStyle = 'rgba(182,108,255,0.10)';
      ctx.fillRect(x + Math.floor(size * 0.45), y + Math.floor(size * 0.45), 2, 2);
    }
    return;
  }

  const c = tile === TILE.WALL ? theme.wall : theme.floor;
  const r = vdHash(gx, gy, 7);
  ctx.fillStyle = c[r > 0.56 ? 1 : r < 0.22 ? 2 : 0];
  ctx.fillRect(x, y, size, size);

  const lip = Math.max(2, Math.floor(size * 0.12));
  ctx.fillStyle = 'rgba(255,255,255,0.08)';
  ctx.fillRect(x + 1, y + 1, size - 2, Math.max(1, lip - 1));
  ctx.fillStyle = 'rgba(0,0,0,0.18)';
  ctx.fillRect(x, y + size - lip, size, lip);
  ctx.fillRect(x + size - lip, y, lip, size);

  if (tile === TILE.WALL) {
    ctx.fillStyle = 'rgba(0,0,0,0.24)';
    if (r > 0.45) ctx.fillRect(x + Math.floor(size * 0.18), y + Math.floor(size * 0.28), Math.floor(size * 0.52), 2);
    if (r > 0.78) ctx.fillRect(x + Math.floor(size * 0.55), y + Math.floor(size * 0.42), 2, Math.floor(size * 0.38));
  } else if (tile === TILE.FLOOR) {
    if (r > 0.82) {
      ctx.fillStyle = 'rgba(243,247,255,0.12)';
      ctx.fillRect(x + Math.floor(size * 0.2), y + Math.floor(size * 0.35), Math.max(2, Math.floor(size * 0.28)), 1);
    } else if (r < 0.12) {
      ctx.fillStyle = 'rgba(0,0,0,0.20)';
      ctx.fillRect(x + Math.floor(size * 0.58), y + Math.floor(size * 0.62), 2, 2);
      ctx.fillRect(x + Math.floor(size * 0.28), y + Math.floor(size * 0.48), 2, 2);
    }
  } else if (tile === TILE.STAIRS) {
    ctx.fillStyle = 'rgba(0,0,0,0.55)';
    ctx.fillRect(x + 1, y + 1, size - 2, size - 2);
    vdDrawSprite(ctx, VD_SPRITES.stairs, x, y, size);
  }
}

function vdDrawGlyph(ctx, id, x, y, size, color) {
  const glyph = VD_GLYPHS[id] || VD_GLYPHS.default;
  const cell = Math.max(2, Math.floor(size / 7));
  const ox = Math.round(x + (size - glyph[0].length * cell) / 2);
  const oy = Math.round(y + (size - glyph.length * cell) / 2);
  ctx.fillStyle = 'rgba(0,0,0,0.55)';
  ctx.fillRect(x, y, size, size);
  ctx.fillStyle = color || VD_PALETTE.gold;
  for (let row = 0; row < glyph.length; row++) {
    for (let col = 0; col < glyph[row].length; col++) {
      if (glyph[row][col] === '1') ctx.fillRect(ox + col * cell, oy + row * cell, cell, cell);
    }
  }
  ctx.fillStyle = VD_PALETTE.white;
  ctx.fillRect(ox + cell * 2, oy + cell, Math.max(1, cell - 1), Math.max(1, cell - 1));
}

