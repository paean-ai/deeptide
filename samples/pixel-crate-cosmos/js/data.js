// Pixel Crate Cosmos - level data for a space-station box-pushing puzzle.
//
// Level legend:
//   #  wall            (space) floor      .  floor (explicit)
//   _  socket/target   $  power-core      *  core already on a socket
//   @  robot           +  robot on socket ~  ice panel (robot slides)
//
// Rule: the robot slides across ice panels until it hits a wall, a core, or
// reaches normal floor. Cores never slide. Pushing a core stops a slide.

const LEVELS = [
  // 1 - first push
  [
    '#######',
    '#  _  #',
    '#  $  #',
    '#  @  #',
    '#######',
  ],
  // 2 - line up first
  [
    '#######',
    '# _   #',
    '# $   #',
    '#   @ #',
    '#######',
  ],
  // 3 - push left
  [
    '########',
    '#      #',
    '#_$  @ #',
    '#      #',
    '########',
  ],
  // 4 - two cores
  [
    '#########',
    '# _   _ #',
    '# $   $ #',
    '#   @   #',
    '#########',
  ],
  // 5 - the long L
  [
    '#########',
    '#    _  #',
    '#       #',
    '#  $    #',
    '#       #',
    '#  @    #',
    '#########',
  ],
  // 6 - ice glide
  [
    '########',
    '#   _  #',
    '#   $  #',
    '#@~~~  #',
    '########',
  ],
  // 7 - L-path push
  [
    '#########',
    '#   _   #',
    '#       #',
    '#  @$   #',
    '#       #',
    '#########',
  ],
  // 8 - three cores
  [
    '#########',
    '# _ _ _ #',
    '# $ $ $ #',
    '#       #',
    '#   @   #',
    '#########',
  ],
  // 9 - mirror pair
  [
    '#########',
    '#  _    #',
    '#  $    #',
    '#    @  #',
    '#  $    #',
    '#  _    #',
    '#########',
  ],
  // 10 - corner haul
  [
    '########',
    '#     _#',
    '#      #',
    '#  $   #',
    '#      #',
    '# @    #',
    '########',
  ],
  // 11 - two pushes per core
  [
    '#########',
    '#_      #',
    '# ##### #',
    '# $   @ #',
    '#       #',
    '#########',
  ],
  // 12 - mirrored cores
  [
    '##########',
    '#_      _#',
    '#$      $#',
    '#   @    #',
    '#        #',
    '##########',
  ],
  // 13 - ice and a wall
  [
    '##########',
    '#      _ #',
    '#  ###  ##',
    '#@~~~~$  #',
    '#  ###   #',
    '##########',
  ],
  // 14 - the vault
  [
    '##########',
    '#_      _#',
    '# $    $ #',
    '#   @    #',
    '# $    $ #',
    '#_      _#',
    '##########',
  ],
];

// tile codes used at runtime
const T_WALL = 0, T_FLOOR = 1, T_ICE = 2;

function parseLevel(rows) {
  const h = rows.length, w = Math.max(...rows.map(r => r.length));
  const tiles = [];
  const targets = [];
  let crates = [];
  let player = { x: 0, y: 0 };
  for (let y = 0; y < h; y++) {
    const row = [];
    for (let x = 0; x < w; x++) {
      const ch = rows[y][x] || ' ';
      let tile = T_FLOOR;
      if (ch === '#') tile = T_WALL;
      else if (ch === '~') tile = T_ICE;
      row.push(tile);
      if (ch === '_' || ch === '*' || ch === '+') targets.push({ x, y });
      if (ch === '$' || ch === '*') crates.push({ x, y });
      if (ch === '@' || ch === '+') player = { x, y };
    }
    tiles.push(row);
  }
  return { w, h, tiles, targets, crates, player };
}

const LEVEL_COUNT = LEVELS.length;
