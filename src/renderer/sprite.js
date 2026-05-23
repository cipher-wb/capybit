// Sprite sheet loader / frame animator.
//
// Rendering route is decided at boot:
//   - if PREFER_CODE_DRAWN === true → always use the Canvas-drawn capybara
//     (src/renderer/placeholder_capybara.js), even if PNGs exist.
//   - else → try to load PNGs from src/assets/sprites/. Falls back to the
//     code-drawn renderer if any frame is missing.
//
// Cipher (2026-05-23): chose to stay on the code-drawn renderer permanently.
// The earlier sprite-image plan (PRD §6) is shelved — see ADR 0008.

/** Hard switch: stay on the code-drawn renderer regardless of any PNGs
 *  sitting in src/assets/sprites/. Flip to false if you ever want to A/B
 *  against real sprite frames. */
export const PREFER_CODE_DRAWN = true;

export const ACTIONS = {
  idle:      { frames: 2, fps: 4 },
  walk:      { frames: 4, fps: 8 },
  sleep:     { frames: 1, fps: 1 },
  talk:      { frames: 2, fps: 6 },
  happy:     { frames: 1, fps: 1 },
  sad:       { frames: 1, fps: 1 },
  surprised: { frames: 1, fps: 1 },
  curious:   { frames: 1, fps: 1 },
  stretch:   { frames: 1, fps: 1 },
  sneeze:    { frames: 1, fps: 1 },
};

const ASSET_BASE = './assets/sprites';

/**
 * Try to preload all sprite frames. Resolves with a {action: Image[]} map on
 * success, or `null` if any required frame is missing — in that case the
 * caller should fall back to the placeholder renderer.
 */
export async function tryPreloadSprites() {
  if (PREFER_CODE_DRAWN) return null;
  const result = {};
  try {
    for (const [action, { frames }] of Object.entries(ACTIONS)) {
      result[action] = [];
      for (let i = 0; i < frames; i++) {
        const img = new Image();
        img.src = `${ASSET_BASE}/${action}_${i}.png`;
        await img.decode();
        result[action].push(img);
      }
    }
    return result;
  } catch {
    return null;
  }
}

/**
 * Pick the current frame index for the given action at time `t` (ms).
 */
export function frameAt(action, t) {
  const def = ACTIONS[action] ?? ACTIONS.idle;
  if (def.frames <= 1) return 0;
  return Math.floor((t / 1000) * def.fps) % def.frames;
}
