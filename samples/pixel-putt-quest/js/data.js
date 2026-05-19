// Pixel Putt Quest - mini-golf course data.
//
// Rects are [x, y, w, h]. The course border is added automatically; holes only
// list interior walls / sand / water. start & cup are {x, y}.

const VW = 480, VH = 340;
const WALL = 14;                 // border wall thickness
const BALL_R = 7;
const CUP_R = 13;
const MAX_POWER = 560;           // launch speed at full drag
const DRAG_MAX = 116;            // drag pixels for full power

const HOLES = [
  { par: 2, start: { x: 70, y: 170 }, cup: { x: 410, y: 170 },
    walls: [[228, 70, 24, 130]], sand: [], water: [] },
  { par: 3, start: { x: 70, y: 270 }, cup: { x: 410, y: 70 },
    walls: [[150, 130, 24, 190], [300, 30, 24, 190]], sand: [], water: [] },
  { par: 3, start: { x: 70, y: 170 }, cup: { x: 410, y: 170 },
    walls: [[210, 24, 24, 110], [210, 206, 24, 110]], sand: [[210, 134, 24, 72]], water: [] },
  { par: 3, start: { x: 240, y: 285 }, cup: { x: 240, y: 60 },
    walls: [[120, 130, 110, 24], [250, 186, 110, 24]], sand: [], water: [] },
  { par: 4, start: { x: 70, y: 270 }, cup: { x: 410, y: 70 },
    walls: [[160, 24, 24, 200], [296, 116, 24, 200]],
    sand: [[40, 160, 100, 100]], water: [] },
  { par: 3, start: { x: 70, y: 170 }, cup: { x: 410, y: 170 },
    walls: [[230, 70, 24, 60], [230, 210, 24, 60]],
    sand: [], water: [[200, 130, 84, 80]] },
  { par: 4, start: { x: 70, y: 70 }, cup: { x: 410, y: 270 },
    walls: [[120, 24, 24, 180], [240, 136, 24, 180], [336, 24, 24, 180]],
    sand: [[260, 230, 130, 70]], water: [] },
  { par: 4, start: { x: 70, y: 270 }, cup: { x: 410, y: 270 },
    walls: [[150, 150, 24, 166], [306, 150, 24, 166], [150, 150, 180, 24]],
    sand: [], water: [[200, 60, 80, 70]] },
  { par: 5, start: { x: 70, y: 170 }, cup: { x: 410, y: 170 },
    walls: [[130, 24, 24, 130], [130, 154, 150, 24], [256, 178, 24, 138],
            [330, 60, 24, 200]],
    sand: [[40, 220, 100, 90]], water: [[290, 24, 90, 90]] },
];
const HOLE_COUNT = HOLES.length;

// Auto-built border walls (4 rects framing the course).
function borderWalls() {
  return [
    [0, 0, VW, WALL], [0, VH - WALL, VW, WALL],
    [0, 0, WALL, VH], [VW - WALL, 0, WALL, VH],
  ];
}
