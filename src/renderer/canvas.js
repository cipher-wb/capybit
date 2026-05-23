// Canvas render loop. 60fps in dev; later we can throttle to 4-8fps in idle
// per the PRD performance budget (§8.2 in CLAUDE.md).

import { drawPlaceholder } from './placeholder_capybara.js';
import { frameAt } from './sprite.js';

export function startRenderLoop({ canvas, sprites, getAction }) {
  const ctx = canvas.getContext('2d');
  ctx.imageSmoothingEnabled = true;

  let raf = 0;
  const start = performance.now();

  function tick(now) {
    const t = now - start;
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    const action = getAction();
    if (sprites && sprites[action]) {
      const idx = frameAt(action, t);
      const img = sprites[action][idx];
      const dx = (canvas.width - img.width) / 2;
      const dy = (canvas.height - img.height) / 2;
      ctx.drawImage(img, dx, dy);
    } else {
      drawPlaceholder(ctx, { t, action });
    }

    raf = requestAnimationFrame(tick);
  }
  raf = requestAnimationFrame(tick);

  return () => cancelAnimationFrame(raf);
}
