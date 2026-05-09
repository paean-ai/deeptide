const TILE_SIZE = 48;

const TILE_TYPES = {
  GRASS: 0,
  SAND: 1,
  STONE: 2,
  ICE: 3,
  FLOWER: 4,
  RUIN: 5,
};

const TILE_COLORS = {
  [TILE_TYPES.GRASS]:  { base: [34, 86, 42], light: [72, 132, 62], dark: [20, 58, 31], accent: [120, 185, 80] },
  [TILE_TYPES.SAND]:   { base: [142, 126, 82], light: [181, 158, 100], dark: [104, 91, 62], accent: [218, 190, 116] },
  [TILE_TYPES.STONE]:  { base: [78, 82, 92], light: [116, 122, 132], dark: [48, 51, 62], accent: [150, 154, 160] },
  [TILE_TYPES.ICE]:    { base: [96, 142, 165], light: [164, 211, 224], dark: [62, 104, 130], accent: [232, 252, 255] },
  [TILE_TYPES.FLOWER]: { base: [42, 98, 46], light: [82, 145, 69], dark: [22, 68, 32], accent: [222, 136, 204] },
  [TILE_TYPES.RUIN]:   { base: [62, 68, 76], light: [124, 116, 103], dark: [35, 39, 49], accent: [92, 150, 103] },
};

function hash(x, y, seed = 1) {
  let h = Math.sin(x * 12.9898 + y * 78.233 + seed * 37.719) * 43758.5453;
  return h - Math.floor(h);
}

function mixColor(a, b, t) {
  return [
    Math.round(a[0] + (b[0] - a[0]) * t),
    Math.round(a[1] + (b[1] - a[1]) * t),
    Math.round(a[2] + (b[2] - a[2]) * t),
  ];
}

function rgb(c, alpha) {
  if (alpha == null) return `rgb(${c[0]},${c[1]},${c[2]})`;
  return `rgba(${c[0]},${c[1]},${c[2]},${alpha})`;
}

class Tile {
  constructor(type, variant, detailType, detailSeed) {
    this.type = type;
    this.variant = variant;
    this.detailType = detailType;
    this.detailSeed = detailSeed;
  }
}

class Tilemap {
  constructor(cols, rows) {
    this.cols = cols;
    this.rows = rows;
    this.tiles = [];
    this._generateChunked();
  }

  get worldWidth() { return this.cols * TILE_SIZE; }
  get worldHeight() { return this.rows * TILE_SIZE; }

  _regionType(cx, cy) {
    const r = hash(cx, cy, 9);
    const drift = hash(cx + 11, cy - 7, 4);
    if (r < 0.50 + drift * 0.08) return TILE_TYPES.GRASS;
    if (r < 0.68) return TILE_TYPES.SAND;
    if (r < 0.84) return TILE_TYPES.STONE;
    if (r < 0.94) return TILE_TYPES.ICE;
    return TILE_TYPES.RUIN;
  }

  _generateChunked() {
    const chunk = 7;
    const regionTypes = new Map();
    for (let cy = 0; cy < Math.ceil(this.rows / chunk); cy++) {
      for (let cx = 0; cx < Math.ceil(this.cols / chunk); cx++) {
        regionTypes.set(`${cx},${cy}`, this._regionType(cx, cy));
      }
    }

    for (let row = 0; row < this.rows; row++) {
      this.tiles[row] = [];
      for (let col = 0; col < this.cols; col++) {
        const cx = Math.floor(col / chunk);
        const cy = Math.floor(row / chunk);
        let type = regionTypes.get(`${cx},${cy}`);

        const edgeBlend = hash(col, row, 14);
        const local = hash(col, row, 3);
        if ((col % chunk === 0 || row % chunk === 0) && edgeBlend < 0.34) {
          const neighbor = regionTypes.get(`${cx + (edgeBlend < 0.17 ? -1 : 1)},${cy}`) ||
                           regionTypes.get(`${cx},${cy + (edgeBlend < 0.17 ? -1 : 1)}`);
          if (neighbor != null) type = neighbor;
        }
        if (type === TILE_TYPES.GRASS && local > 0.91) type = TILE_TYPES.FLOWER;
        if ((type === TILE_TYPES.STONE || type === TILE_TYPES.GRASS) && hash(col, row, 66) > 0.975) type = TILE_TYPES.RUIN;

        let detailType = 0;
        const d = hash(col, row, 22);
        if (type === TILE_TYPES.GRASS && d < 0.17) detailType = 1;
        else if (type === TILE_TYPES.FLOWER && d < 0.65) detailType = 4;
        else if (type === TILE_TYPES.STONE && d < 0.24) detailType = 2;
        else if (type === TILE_TYPES.ICE && d < 0.21) detailType = 3;
        else if (type === TILE_TYPES.SAND && d < 0.20) detailType = 5;
        else if (type === TILE_TYPES.RUIN && d < 0.28) detailType = 6;
        else if (type === TILE_TYPES.RUIN && d < 0.50) detailType = 7;
        else if (type === TILE_TYPES.GRASS && d > 0.94) detailType = 8;
        else if (type === TILE_TYPES.ICE && d > 0.94) detailType = 9;

        this.tiles[row][col] = new Tile(type, local, detailType, hash(col, row, 31));
      }
    }
  }

  render(ctx, camera) {
    const startCol = Math.max(0, Math.floor(camera.x / TILE_SIZE) - 1);
    const endCol = Math.min(this.cols - 1, Math.ceil((camera.x + camera.screenWidth) / TILE_SIZE) + 1);
    const startRow = Math.max(0, Math.floor(camera.y / TILE_SIZE) - 1);
    const endRow = Math.min(this.rows - 1, Math.ceil((camera.y + camera.screenHeight) / TILE_SIZE) + 1);

    for (let row = startRow; row <= endRow; row++) {
      for (let col = startCol; col <= endCol; col++) {
        this._renderTile(ctx, this.tiles[row][col], col * TILE_SIZE, row * TILE_SIZE, col, row);
      }
    }
  }

  _renderTile(ctx, tile, x, y, col, row) {
    const c = TILE_COLORS[tile.type];
    const v = (tile.variant - 0.5) * 0.18;
    const base = mixColor(c.base, tile.variant > 0.5 ? c.light : c.dark, Math.abs(v));
    ctx.fillStyle = rgb(base);
    ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);

    const p = 4;
    ctx.fillStyle = rgb(c.light, 0.13);
    if (hash(col, row, 41) > 0.45) ctx.fillRect(x + p, y + p, TILE_SIZE - p * 2, p);
    ctx.fillStyle = rgb(c.dark, 0.18);
    ctx.fillRect(x, y + TILE_SIZE - p, TILE_SIZE, p);
    ctx.fillRect(x + TILE_SIZE - p, y, p, TILE_SIZE);

    this._renderTransitions(ctx, tile, x, y, col, row);
    this._renderDetail(ctx, tile, x, y);
  }

  _renderTransitions(ctx, tile, x, y, col, row) {
    const neighbors = [
      { dx: 0, dy: -1, rect: [0, 0, TILE_SIZE, 4] },
      { dx: 1, dy: 0, rect: [TILE_SIZE - 4, 0, 4, TILE_SIZE] },
      { dx: 0, dy: 1, rect: [0, TILE_SIZE - 4, TILE_SIZE, 4] },
      { dx: -1, dy: 0, rect: [0, 0, 4, TILE_SIZE] },
    ];
    for (const n of neighbors) {
      const other = this.getTile(col + n.dx, row + n.dy);
      if (!other || other.type === tile.type) continue;
      const oc = TILE_COLORS[other.type];
      ctx.fillStyle = rgb(oc.base, 0.26);
      ctx.fillRect(x + n.rect[0], y + n.rect[1], n.rect[2], n.rect[3]);
    }
  }

  _renderDetail(ctx, tile, x, y) {
    const s = tile.detailSeed;
    const px = x + 8 + Math.floor(s * 28);
    const py = y + 8 + Math.floor(hash(s * 50, s * 80, 2) * 28);
    const c = TILE_COLORS[tile.type];

    if (tile.detailType === 1) {
      ctx.fillStyle = rgb(c.accent);
      ctx.fillRect(px, py + 8, 3, 8);
      ctx.fillRect(px - 4, py + 10, 4, 3);
      ctx.fillRect(px + 3, py + 6, 4, 3);
      ctx.fillStyle = rgb(c.dark);
      ctx.fillRect(px + 1, py + 14, 2, 2);
    } else if (tile.detailType === 2) {
      ctx.fillStyle = rgb(c.dark, 0.65);
      ctx.fillRect(px, py, 14, 3);
      ctx.fillRect(px + 10, py + 3, 3, 9);
      ctx.fillRect(px + 13, py + 10, 8, 3);
      ctx.fillStyle = rgb(c.light, 0.45);
      ctx.fillRect(px + 1, py - 2, 10, 2);
    } else if (tile.detailType === 3) {
      ctx.fillStyle = rgb(c.accent, 0.7);
      ctx.fillRect(px, py + 4, 16, 2);
      ctx.fillRect(px + 7, py - 3, 2, 16);
      ctx.fillRect(px + 4, py + 1, 8, 8);
    } else if (tile.detailType === 4) {
      ctx.fillStyle = '#f08ac8';
      ctx.fillRect(px, py, 3, 3);
      ctx.fillStyle = '#f2c14e';
      ctx.fillRect(px + 5, py + 3, 3, 3);
      ctx.fillStyle = '#b66cff';
      ctx.fillRect(px - 4, py + 5, 3, 3);
      ctx.fillStyle = rgb(c.accent);
      ctx.fillRect(px + 1, py + 7, 2, 5);
    } else if (tile.detailType === 5) {
      ctx.fillStyle = rgb(c.dark, 0.38);
      ctx.fillRect(px, py, 3, 3);
      ctx.fillRect(px + 12, py + 9, 2, 2);
      ctx.fillStyle = rgb(c.light, 0.35);
      ctx.fillRect(px + 5, py + 4, 7, 2);
    } else if (tile.detailType === 6) {
      ctx.fillStyle = rgb(c.dark, 0.85);
      ctx.fillRect(px - 4, py + 10, 24, 5);
      ctx.fillRect(px, py - 4, 5, 18);
      ctx.fillRect(px + 13, py, 5, 14);
      ctx.fillStyle = rgb(c.light, 0.55);
      ctx.fillRect(px, py - 5, 18, 3);
      ctx.fillStyle = rgb(c.accent, 0.55);
      ctx.fillRect(px + 4, py + 4, 3, 8);
      ctx.fillRect(px + 16, py + 5, 3, 6);
    } else if (tile.detailType === 7) {
      ctx.fillStyle = rgb(c.dark, 0.7);
      ctx.fillRect(px, py + 10, 20, 4);
      ctx.fillRect(px + 4, py + 4, 12, 4);
      ctx.fillStyle = rgb(c.light, 0.45);
      ctx.fillRect(px + 2, py + 2, 14, 2);
      ctx.fillStyle = rgb(c.accent, 0.6);
      ctx.fillRect(px + 1, py + 7, 3, 5);
      ctx.fillRect(px + 15, py + 8, 3, 4);
    } else if (tile.detailType === 8) {
      ctx.fillStyle = '#b5f47a';
      ctx.fillRect(px - 2, py + 8, 3, 8);
      ctx.fillRect(px + 4, py + 5, 3, 11);
      ctx.fillRect(px + 10, py + 9, 3, 7);
      ctx.fillStyle = '#f08ac8';
      ctx.fillRect(px + 3, py + 3, 4, 4);
    } else if (tile.detailType === 9) {
      ctx.fillStyle = '#232b38';
      ctx.fillRect(px - 1, py + 15, 22, 3);
      ctx.fillStyle = '#e8fcff';
      ctx.fillRect(px + 2, py + 4, 4, 12);
      ctx.fillRect(px + 9, py, 3, 16);
      ctx.fillRect(px + 15, py + 7, 4, 9);
      ctx.fillStyle = '#83d8ff';
      ctx.fillRect(px + 10, py + 3, 1, 11);
    }
  }

  getTile(col, row) {
    if (row < 0 || row >= this.rows || col < 0 || col >= this.cols) return null;
    return this.tiles[row][col];
  }

  getTileAt(wx, wy) {
    return this.getTile(Math.floor(wx / TILE_SIZE), Math.floor(wy / TILE_SIZE));
  }
}
