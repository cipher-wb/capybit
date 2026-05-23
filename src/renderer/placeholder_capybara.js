// LCD scene compositor — the desktop window is rendered as a 32×32
// dot-matrix screen with a thin border. Content is composed from layered
// "scene" objects:
//
//   Scene = {
//     sprite: 'idle' | 'happy' | ... | null,   // base 16×16 pet bitmap
//     spriteAt: { x, y },                       // top-left in grid cells; default center
//     overlays: Overlay[],                      // 0+ floating bits
//     bg: 'plain' | 'sparkle' | 'stars',        // optional pattern
//   }
//
//   Overlay =
//     | { type:'icon',  name:'heart'|..., x, y, blink?, animate? }
//     | { type:'text',  text:string,      x, y, blink? }
//     | { type:'cells', cells:[[0/1...]], x, y }      // raw bitmap, AI can author
//
// Default scene comes from the current pet action. External code can push a
// timed override via window.__capybit_pushScene(scene, durationMs); Rust
// (later) will emit a `lcd-scene` event to call it during conversations.
//
// Grid layout (32 cols × 32 rows):
//   rows 0..9   = top overlay zone (icons, text)
//   rows 10..25 = main 16×16 sprite slot at cols 8..23
//   rows 26..31 = bottom overlay zone (status, small text)
//
// Cell size 5px → screen 160×160; window 192×192 with 16px transparent margin.

const GRID_W = 32;
const GRID_H = 32;
const CELL = 5;
const SCREEN_W = GRID_W * CELL;
const SCREEN_H = GRID_H * CELL;
const SCREEN_X = (192 - SCREEN_W) / 2;
const SCREEN_Y = (192 - SCREEN_H) / 2;

const PALETTE = {
  lcdBg:   '#a8bb73',
  lcdGrid: '#92a558',
  lcdOn:   '#1a2510',
  border:  '#2a1f14',
  borderHL:'#5a4630',
};

// ---------------------------------------------------------------------------
// Bitmap parsing.

function parseBitmap(rows, w) {
  const h = rows.length;
  const cells = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) {
    const row = rows[y] || '';
    for (let x = 0; x < w; x++) {
      cells[y * w + x] = row[x] === '#' ? 1 : 0;
    }
  }
  return { w, h, cells };
}

// ---------------------------------------------------------------------------
// Sprite library — 16×16 base poses for the pet.

const SPRITES = {
  idle: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '##.##########.##',
    '##.##########.##',
    '################',
    '.##############.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  idle_blink: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  happy: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '################',
    '###.########.###',
    '################',
    '.####.####.####.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  sad: parseBitmap([
    '................',
    '................',
    '....##....##....',
    '...####..####...',
    '..############..',
    '.##############.',
    '.###.######.###.',
    '.##############.',
    '..############..',
    '.##############.',
    '.##############.',
    '.##############.',
    '.##############.',
    '..############..',
    '..##.##..##.##..',
    '................',
  ], 16),
  sleep: parseBitmap([
    '................',
    '................',
    '................',
    '................',
    '................',
    '...##......##...',
    '..############..',
    '.##############.',
    '.##.########.##.',
    '.##############.',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  surprised: parseBitmap([
    '................',
    '..####....####..',
    '..####....####..',
    '.##############.',
    '################',
    '##.####..####.##',
    '##.####..####.##',
    '################',
    '.####.####.####.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  curious: parseBitmap([
    '................',
    '....##......###.',
    '...####....#####',
    '..#############.',
    '.###############',
    '.###.##########.',
    '.###.##########.',
    '.###############',
    '..#############.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  stretch: parseBitmap([
    '...##......##...',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '##.##########.##',
    '##.##########.##',
    '################',
    '################',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  walk_0: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '##.##########.##',
    '##.##########.##',
    '################',
    '.##############.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '..##.##...##.##.',
    '................',
  ], 16),
  walk_1: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '##.##########.##',
    '##.##########.##',
    '################',
    '.##############.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##...##.##..',
    '................',
  ], 16),
  talk_0: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '##.##########.##',
    '##.##########.##',
    '################',
    '.#####....######',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  talk_1: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '##.##########.##',
    '##.##########.##',
    '################',
    '.######..#######',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  sneeze_windup: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '################',
    '###.########.###',
    '################',
    '.####.####.####.',
    '################',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
  sneeze_burst: parseBitmap([
    '................',
    '...##......##...',
    '..####....####..',
    '.##############.',
    '################',
    '################',
    '################',
    '################',
    '..####..####....',
    '..####..####....',
    '################',
    '################',
    '################',
    '.##############.',
    '.##.##....##.##.',
    '................',
  ], 16),
};

// ---------------------------------------------------------------------------
// Icon library — small bitmaps that can be placed anywhere on the grid.

const ICONS = {
  heart: parseBitmap([
    '.##.##.',
    '#######',
    '#######',
    '.#####.',
    '..###..',
    '...#...',
  ], 7),
  heart_small: parseBitmap([
    '##.##',
    '#####',
    '.###.',
    '..#..',
  ], 5),
  star: parseBitmap([
    '..#..',
    '..#..',
    '#####',
    '.###.',
    '##.##',
  ], 5),
  spark: parseBitmap([
    '.#.',
    '###',
    '.#.',
  ], 3),
  z: parseBitmap([
    '###',
    '..#',
    '.#.',
    '#..',
    '###',
  ], 3),
  exclaim: parseBitmap([
    '#',
    '#',
    '#',
    '#',
    '.',
    '#',
  ], 1),
  question: parseBitmap([
    '###',
    '#.#',
    '..#',
    '.#.',
    '...',
    '.#.',
  ], 3),
  music: parseBitmap([
    '.##',
    '.##',
    '.#.',
    '.#.',
    '##.',
  ], 3),
  sun: parseBitmap([
    '..#..',
    '#.#.#',
    '.###.',
    '#.#.#',
    '..#..',
  ], 5),
  moon: parseBitmap([
    '.##..',
    '##...',
    '##...',
    '##...',
    '.##..',
  ], 5),
  cloud: parseBitmap([
    '.####.',
    '######',
    '######',
  ], 6),
  dots3: parseBitmap([
    '#.#.#',
  ], 5),
  arrow_up: parseBitmap([
    '..#..',
    '.###.',
    '#.#.#',
    '..#..',
    '..#..',
  ], 5),
  arrow_down: parseBitmap([
    '..#..',
    '..#..',
    '#.#.#',
    '.###.',
    '..#..',
  ], 5),
  capy_face: parseBitmap([
    '##...##',
    '##...##',
    '#######',
    '#.#.#.#',
    '#######',
    '##...##',
  ], 7),
};

// ---------------------------------------------------------------------------
// 5×7 font. Lowercase falls through to uppercase. Unknown → '?' glyph.

const FONT_W = 5;
const FONT_H = 7;
const FONT = {
  '0': ['.###.', '#...#', '#..##', '#.#.#', '##..#', '#...#', '.###.'],
  '1': ['..#..', '.##..', '..#..', '..#..', '..#..', '..#..', '.###.'],
  '2': ['.###.', '#...#', '....#', '...#.', '..#..', '.#...', '#####'],
  '3': ['.###.', '#...#', '....#', '..##.', '....#', '#...#', '.###.'],
  '4': ['...#.', '..##.', '.#.#.', '#..#.', '#####', '...#.', '...#.'],
  '5': ['#####', '#....', '####.', '....#', '....#', '#...#', '.###.'],
  '6': ['..##.', '.#...', '#....', '####.', '#...#', '#...#', '.###.'],
  '7': ['#####', '....#', '...#.', '..#..', '.#...', '#....', '#....'],
  '8': ['.###.', '#...#', '#...#', '.###.', '#...#', '#...#', '.###.'],
  '9': ['.###.', '#...#', '#...#', '.####', '....#', '...#.', '.##..'],
  ':': ['.....', '.....', '..#..', '.....', '..#..', '.....', '.....'],
  '.': ['.....', '.....', '.....', '.....', '.....', '.....', '..#..'],
  '-': ['.....', '.....', '.....', '#####', '.....', '.....', '.....'],
  '?': ['.###.', '#...#', '....#', '..##.', '..#..', '.....', '..#..'],
  '!': ['..#..', '..#..', '..#..', '..#..', '..#..', '.....', '..#..'],
  ' ': ['.....', '.....', '.....', '.....', '.....', '.....', '.....'],
  '/': ['....#', '....#', '...#.', '..#..', '.#...', '#....', '#....'],
  'A': ['.###.', '#...#', '#...#', '#####', '#...#', '#...#', '#...#'],
  'B': ['####.', '#...#', '#...#', '####.', '#...#', '#...#', '####.'],
  'C': ['.###.', '#...#', '#....', '#....', '#....', '#...#', '.###.'],
  'D': ['####.', '#...#', '#...#', '#...#', '#...#', '#...#', '####.'],
  'E': ['#####', '#....', '#....', '####.', '#....', '#....', '#####'],
  'F': ['#####', '#....', '#....', '####.', '#....', '#....', '#....'],
  'G': ['.###.', '#...#', '#....', '#.###', '#...#', '#...#', '.###.'],
  'H': ['#...#', '#...#', '#...#', '#####', '#...#', '#...#', '#...#'],
  'I': ['.###.', '..#..', '..#..', '..#..', '..#..', '..#..', '.###.'],
  'J': ['..###', '...#.', '...#.', '...#.', '...#.', '#..#.', '.##..'],
  'K': ['#...#', '#..#.', '#.#..', '##...', '#.#..', '#..#.', '#...#'],
  'L': ['#....', '#....', '#....', '#....', '#....', '#....', '#####'],
  'M': ['#...#', '##.##', '#.#.#', '#.#.#', '#...#', '#...#', '#...#'],
  'N': ['#...#', '##..#', '#.#.#', '#.#.#', '#.#.#', '#..##', '#...#'],
  'O': ['.###.', '#...#', '#...#', '#...#', '#...#', '#...#', '.###.'],
  'P': ['####.', '#...#', '#...#', '####.', '#....', '#....', '#....'],
  'Q': ['.###.', '#...#', '#...#', '#...#', '#.#.#', '#..#.', '.##.#'],
  'R': ['####.', '#...#', '#...#', '####.', '#.#..', '#..#.', '#...#'],
  'S': ['.####', '#....', '#....', '.###.', '....#', '....#', '####.'],
  'T': ['#####', '..#..', '..#..', '..#..', '..#..', '..#..', '..#..'],
  'U': ['#...#', '#...#', '#...#', '#...#', '#...#', '#...#', '.###.'],
  'V': ['#...#', '#...#', '#...#', '#...#', '#...#', '.#.#.', '..#..'],
  'W': ['#...#', '#...#', '#...#', '#.#.#', '#.#.#', '##.##', '#...#'],
  'X': ['#...#', '#...#', '.#.#.', '..#..', '.#.#.', '#...#', '#...#'],
  'Y': ['#...#', '#...#', '.#.#.', '..#..', '..#..', '..#..', '..#..'],
  'Z': ['#####', '....#', '...#.', '..#..', '.#...', '#....', '#####'],
};

// ---------------------------------------------------------------------------
// Grid primitives.

function setCell(grid, x, y) {
  if (x >= 0 && x < GRID_W && y >= 0 && y < GRID_H) {
    grid[y * GRID_W + x] = 1;
  }
}

function blit(grid, bitmap, dx, dy) {
  for (let y = 0; y < bitmap.h; y++) {
    for (let x = 0; x < bitmap.w; x++) {
      if (bitmap.cells[y * bitmap.w + x]) {
        setCell(grid, dx + x, dy + y);
      }
    }
  }
}

function drawText(grid, str, x0, y0) {
  let cx = x0;
  for (const ch of str) {
    const glyph = FONT[ch] || FONT[ch.toUpperCase()] || FONT['?'];
    for (let y = 0; y < FONT_H; y++) {
      const row = glyph[y];
      for (let x = 0; x < FONT_W; x++) {
        if (row[x] === '#') setCell(grid, cx + x, y0 + y);
      }
    }
    cx += FONT_W + 1;
  }
}

function applyOverlay(grid, ov, t) {
  // blink/animate gating
  if (ov.blink && Math.floor(t / 500) % 2 === 0) return;

  if (ov.type === 'icon') {
    const icon = ICONS[ov.name];
    if (!icon) return;
    let y = ov.y;
    if (ov.animate === 'bob') y += Math.round(Math.sin(t / 300) * 1);
    blit(grid, icon, ov.x, y);
  } else if (ov.type === 'text') {
    drawText(grid, ov.text, ov.x, ov.y);
  } else if (ov.type === 'cells') {
    const cells = ov.cells || [];
    for (let yy = 0; yy < cells.length; yy++) {
      const row = cells[yy] || '';
      for (let xx = 0; xx < (row.length || 0); xx++) {
        const v = typeof row === 'string' ? row[xx] === '#' : row[xx];
        if (v) setCell(grid, ov.x + xx, ov.y + yy);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Default scene per pet action.

const SPRITE_SLOT_X = 8;
const SPRITE_SLOT_Y = 10;

function defaultSceneForAction(action, t) {
  const blinkPhase = (t % 3800) / 3800;
  const blinking = (blinkPhase > 0.930 && blinkPhase < 0.955) ||
                   (blinkPhase > 0.965 && blinkPhase < 0.985);

  const base = { spriteAt: { x: SPRITE_SLOT_X, y: SPRITE_SLOT_Y }, overlays: [] };

  switch (action) {
    case 'sleep':
      return {
        ...base,
        sprite: 'sleep',
        overlays: zParticles(t),
      };
    case 'walk': {
      const f = Math.floor(t / 220) % 2;
      return { ...base, sprite: f === 0 ? 'walk_0' : 'walk_1' };
    }
    case 'happy':
      return {
        ...base,
        sprite: 'happy',
        overlays: [
          { type: 'icon', name: 'heart', x: 25, y: 2, animate: 'bob' },
          { type: 'icon', name: 'spark', x: 3, y: 5 },
        ],
      };
    case 'sad':
      return {
        ...base,
        sprite: 'sad',
        overlays: [
          { type: 'icon', name: 'dots3', x: 13, y: 5 },
        ],
      };
    case 'surprised':
      return {
        ...base,
        sprite: 'surprised',
        overlays: [
          { type: 'icon', name: 'exclaim', x: 25, y: 4, blink: true },
        ],
      };
    case 'curious':
      return {
        ...base,
        sprite: 'curious',
        overlays: [
          { type: 'icon', name: 'question', x: 25, y: 4, animate: 'bob' },
        ],
      };
    case 'stretch':
      return { ...base, sprite: 'stretch' };
    case 'sneeze': {
      const cyc = t % 1600;
      if (cyc < 700) return { ...base, sprite: 'sneeze_windup' };
      if (cyc < 950) {
        return {
          ...base,
          sprite: 'sneeze_burst',
          overlays: sneezeBurstParticles(cyc - 700),
        };
      }
      return { ...base, sprite: 'sneeze_windup' };
    }
    case 'talk': {
      const f = Math.floor(t / 165) % 2;
      return {
        ...base,
        sprite: f === 0 ? 'talk_0' : 'talk_1',
        overlays: [
          { type: 'icon', name: 'dots3', x: 25, y: 5, blink: true },
        ],
      };
    }
    case 'idle':
    default:
      return { ...base, sprite: blinking ? 'idle_blink' : 'idle' };
  }
}

function zParticles(t) {
  const out = [];
  for (let i = 0; i < 3; i++) {
    const phase = (((t + i * 800) % 2400) / 2400);
    if (phase > 0.85) continue;
    const x = 25 + i;
    const y = Math.round(7 - phase * 6);
    out.push({ type: 'icon', name: 'z', x, y });
  }
  return out;
}

function sneezeBurstParticles(localT) {
  const out = [];
  const bt = localT / 250;
  for (let i = 0; i < 5; i++) {
    const ang = -Math.PI / 2 + (i - 2) * 0.35;
    const d = bt * 5;
    const x = 14 + Math.round(Math.cos(ang) * d);
    const y = 18 + Math.round(Math.sin(ang) * d);
    out.push({ type: 'cells', cells: ['#'], x, y });
  }
  return out;
}

// ---------------------------------------------------------------------------
// Scene composition.

function composeScene(grid, scene, t) {
  if (scene.bg === 'sparkle') drawSparkleBackground(grid, t);
  if (scene.sprite) {
    const sp = SPRITES[scene.sprite];
    if (sp) {
      const slot = scene.spriteAt || { x: SPRITE_SLOT_X, y: SPRITE_SLOT_Y };
      blit(grid, sp, slot.x, slot.y);
    }
  }
  for (const ov of scene.overlays || []) applyOverlay(grid, ov, t);

  // Heartbeat dot bottom-right (always-alive ambient detail).
  if (Math.floor(t / 600) % 2 === 0) setCell(grid, 30, 30);
}

function drawSparkleBackground(grid, t) {
  // Random-but-stable sparkle pattern shifting slowly.
  const seed = Math.floor(t / 800);
  for (let i = 0; i < 6; i++) {
    const h = ((seed * 9301 + 49297 + i * 13) % 233280) / 233280;
    const v = ((seed * 1297 + 12347 + i * 51) % 144) / 144;
    const x = Math.floor(h * GRID_W);
    const y = Math.floor(v * GRID_H);
    setCell(grid, x, y);
  }
}

// ---------------------------------------------------------------------------
// External scene override hooks.
//
// Other code (Rust event listener, console debug) can call
// `window.__capybit_pushScene(scene, durationMs)` to display a scene for a
// limited time. After durationMs the screen reverts to default-for-action.

let _externalScene = null;
let _externalUntil = 0;

function resolveScene(frame) {
  const now = performance.now();
  if (_externalScene && now < _externalUntil) return _externalScene;
  _externalScene = null;
  return defaultSceneForAction(frame.action || 'idle', frame.t);
}

if (typeof window !== 'undefined') {
  window.__capybit_pushScene = (scene, durationMs) => {
    _externalScene = scene || null;
    _externalUntil = performance.now() + (durationMs || 5000);
    console.info(
      '[capybit] external scene set, expires in',
      durationMs || 5000,
      'ms — sprite:',
      scene?.sprite,
      'overlays:',
      scene?.overlays?.length || 0,
    );
  };
  window.__capybit_clearScene = () => {
    _externalScene = null;
    _externalUntil = 0;
  };
  console.info('[capybit] __capybit_pushScene installed on window');
}

// ---------------------------------------------------------------------------
// Painting to the actual canvas.

export function drawPlaceholder(ctx, frame) {
  const grid = new Uint8Array(GRID_W * GRID_H);
  const scene = resolveScene(frame);
  composeScene(grid, scene, frame.t);

  paintBorder(ctx);
  paintLCD(ctx, grid);
}

function paintBorder(ctx) {
  // Outer thin border, slight bevel.
  const x = SCREEN_X - 4;
  const y = SCREEN_Y - 4;
  const w = SCREEN_W + 8;
  const h = SCREEN_H + 8;
  // shadow
  ctx.fillStyle = 'rgba(0, 0, 0, 0.25)';
  ctx.fillRect(x + 1, y + 2, w, h);
  // border frame
  ctx.fillStyle = PALETTE.border;
  ctx.fillRect(x, y, w, h);
  // 1px highlight along the top-left
  ctx.fillStyle = PALETTE.borderHL;
  ctx.fillRect(x, y, w, 1);
  ctx.fillRect(x, y, 1, h);
}

function paintLCD(ctx, grid) {
  // base wash
  ctx.fillStyle = PALETTE.lcdBg;
  ctx.fillRect(SCREEN_X, SCREEN_Y, SCREEN_W, SCREEN_H);

  // off-cell subtle grid
  ctx.fillStyle = PALETTE.lcdGrid;
  for (let y = 0; y < GRID_H; y++) {
    for (let x = 0; x < GRID_W; x++) {
      ctx.fillRect(
        SCREEN_X + x * CELL + 0.5,
        SCREEN_Y + y * CELL + 0.5,
        CELL - 1,
        CELL - 1,
      );
    }
  }

  // on-cells
  ctx.fillStyle = PALETTE.lcdOn;
  for (let y = 0; y < GRID_H; y++) {
    for (let x = 0; x < GRID_W; x++) {
      if (grid[y * GRID_W + x]) {
        ctx.fillRect(
          SCREEN_X + x * CELL + 0.5,
          SCREEN_Y + y * CELL + 0.5,
          CELL - 1,
          CELL - 1,
        );
      }
    }
  }
}
