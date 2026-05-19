// Pixel Mart Manager - drawing primitives. Call artInit(ctx) once at startup.

let _ctx = null;
function artInit(ctx) { _ctx = ctx; }

function hash2(x, y, seed = 0) {
  const n = Math.sin(x * 12.9898 + y * 78.233 + seed * 41.17) * 43758.5453;
  return n - Math.floor(n);
}

function rect(x, y, w, h, color) {
  _ctx.fillStyle = color;
  _ctx.fillRect(Math.round(x), Math.round(y), Math.round(w), Math.round(h));
}

function pixelText(text, x, y, color = PAL.white, align = 'left', size = 12) {
  _ctx.font = `bold ${size}px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`;
  _ctx.textAlign = align;
  _ctx.fillStyle = PAL.shadow;
  _ctx.fillText(text, x + 1, y + 1);
  _ctx.fillStyle = color;
  _ctx.fillText(text, x, y);
  _ctx.textAlign = 'left';
}

function drawPixelSprite(sprite, x, y, scale = 3, flip = 1, alpha = 1) {
  _ctx.save();
  _ctx.globalAlpha = alpha;
  const rows = sprite.length, cols = sprite[0].length;
  const ox = Math.round(x - cols * scale / 2);
  const oy = Math.round(y - rows * scale);
  _ctx.translate(ox + (flip < 0 ? cols * scale : 0), oy);
  _ctx.scale(flip, 1);
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const ch = sprite[row][col];
      if (ch !== '.' && SPRITE_MAP[ch]) {
        _ctx.fillStyle = SPRITE_MAP[ch];
        _ctx.fillRect(col * scale, row * scale, scale, scale);
      }
    }
  }
  _ctx.restore();
}

function drawShadow(x, y, w = 34, h = 10, alpha = 0.32) {
  _ctx.save();
  _ctx.globalAlpha = alpha;
  _ctx.fillStyle = PAL.shadow;
  for (let px = -w; px <= w; px += 3) {
    for (let py = -h; py <= h; py += 3) {
      if ((px * px) / (w * w) + (py * py) / (h * h) <= 1) {
        _ctx.fillRect(Math.round(x + px), Math.round(y + py), 3, 3);
      }
    }
  }
  _ctx.restore();
}

function drawPanel(x, y, w, h, fill, hi, edge = PAL.ink) {
  rect(x + 4, y + 4, w, h, '#00000030');
  rect(x, y, w, h, edge);
  rect(x + 3, y + 3, w - 6, h - 6, fill);
  rect(x + 3, y + 3, w - 6, 5, hi);
  rect(x + 3, y + h - 8, w - 6, 5, '#00000022');
}

// Small progress pip bar - used for customer patience.
function drawBar(x, y, w, frac, color) {
  rect(x - 1, y - 1, w + 2, 6, PAL.ink);
  rect(x, y, w, 4, '#00000044');
  rect(x, y, Math.max(0, w * frac), 4, color);
}

function setGlobalAlpha(a) { _ctx.globalAlpha = a; }
