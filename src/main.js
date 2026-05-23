// Capybit frontend entry — M0.
// Wiring: preload sprites (fall back to placeholder if missing) → start
// render loop → attach dragging.

import { tryPreloadSprites } from './renderer/sprite.js';
import { startRenderLoop } from './renderer/canvas.js';
import { attachDragging, attachDoubleClickToTalk } from './renderer/motion.js';

const canvas = document.getElementById('stage');

// Behavior state — driven by `pet-state` events from the Rust tick scheduler
// (M4). Initial value is just a render-time placeholder.
const state = {
  action: 'idle',
};

function wireStateEvents(tauri) {
  if (!tauri?.event?.listen) return;
  tauri.event.listen('pet-state', (event) => {
    const payload = event.payload;
    if (payload?.current_action && payload.current_action !== state.action) {
      state.action = payload.current_action;
    }
  });
  // Rust → LCD scene override channel. See placeholder_capybara.js for the
  // scene schema; back-end emits this via `set_scene` command (or its own
  // proactive triggers later). Schema is intentionally identical to the
  // shape the renderer accepts.
  tauri.event.listen('lcd-scene', (event) => {
    const payload = event.payload || {};
    if (payload.scene && typeof window.__capybit_pushScene === 'function') {
      window.__capybit_pushScene(payload.scene, payload.duration_ms || 5000);
    }
  });
}

function wireHoverTooltip(canvas, tauri) {
  if (!tauri?.core?.invoke) return;
  const el = document.getElementById('tooltip');
  let inflight = false;
  let hideTimer = 0;

  function hide() {
    el.classList.remove('show');
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => { el.hidden = true; }, 200);
  }

  canvas.addEventListener('mouseenter', async () => {
    // The whole device is the hit zone now (ADR 0009); always fetch.
    if (inflight) return;
    inflight = true;
    // Show a "thinking" placeholder immediately so the box isn't blank
    // during the network round-trip on cold cache.
    el.textContent = '…';
    el.hidden = false;
    clearTimeout(hideTimer);
    requestAnimationFrame(() => el.classList.add('show'));
    try {
      const text = await tauri.core.invoke('get_inner_monologue');
      console.info('[capybit] monologue:', JSON.stringify(text));
      el.textContent = (text && text.trim()) || '（没说话。）';
    } catch (err) {
      console.warn('[capybit] monologue failed', err);
      el.textContent = '（出了点小问题：' + String(err).slice(0, 60) + '）';
    } finally {
      inflight = false;
    }
  });

  canvas.addEventListener('mouseleave', hide);
}

(async function boot() {
  // Tauri 2 with `app.withGlobalTauri = true` exposes the convenience API on
  // `window.__TAURI__`. When opened in a plain browser (preview without
  // building Tauri), the global is absent — degrade to render-only.
  const tauri = window.__TAURI__;
  const tauriWindow = tauri?.window?.getCurrentWindow ?? null;
  if (!tauri) {
    console.info('[capybit] running outside Tauri — drag disabled, preview only');
  }

  const sprites = await tryPreloadSprites();
  console.info(
    sprites
      ? '[capybit] sprite frames loaded — using real sprites'
      : '[capybit] no sprites found — drawing placeholder capybara'
  );

  startRenderLoop({
    canvas,
    sprites,
    getAction: () => state.action,
  });

  if (tauriWindow) {
    attachDragging(canvas, tauriWindow);
    attachDoubleClickToTalk(canvas, tauri.core.invoke);
    wireStateEvents(tauri);
    wireHoverTooltip(canvas, tauri);
  }
})();
