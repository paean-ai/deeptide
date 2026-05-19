// Pixel Mahjong - layout campaign and tile symbols.

const VW = 360, VH = 480;

// 9 distinct tile faces. Each is matched against its own kind.
const SYMBOL_COUNT = 9;

// Each layout is a centred pyramid: layer L occupies the rectangle inset by L
// on every side of a base w x h. Tile counts are all even (so they pair up).
const LAYOUTS = [
  { name: ['Dawn', '晨曦'],     w: 6,  h: 4, layers: 2 },
  { name: ['Garden', '花园'],   w: 8,  h: 4, layers: 2 },
  { name: ['Terrace', '露台'],  w: 8,  h: 6, layers: 2 },
  { name: ['Pavilion', '楼阁'], w: 10, h: 6, layers: 2 },
  { name: ['Pagoda', '宝塔'],   w: 10, h: 6, layers: 3 },
  { name: ['Dragon Hall', '龙阙'], w: 10, h: 8, layers: 3 },
  { name: ['Citadel', '城塞'],  w: 11, h: 8, layers: 3 },
  { name: ['Empire', '帝阙'],   w: 12, h: 9, layers: 3 },
];
const LAYOUT_COUNT = LAYOUTS.length;

// Expand a layout into its list of {layer, row, col} cells.
function layoutCells(layout) {
  const cells = [];
  for (let L = 0; L < layout.layers; L++) {
    for (let r = L; r < layout.h - L; r++) {
      for (let c = L; c < layout.w - L; c++) cells.push({ layer: L, row: r, col: c });
    }
  }
  return cells;
}
