// Dragging + (later) random_walk.
// For M0: dragging only. The Rust-side click-through poller already routes
// mouse events to the window when the cursor is over the pet, so we just need
// to translate mousedown → window.startDragging().

let dragHandler = null;
let dblHandler = null;

// LCD device + thin border now occupies 168×168 centered in the 192×192
// window (placeholder_capybara.js + ADR 0010). Mouse interaction outside
// that box should fall through to the desktop, so we gate handlers here.
function insideCase(canvas, e) {
  const rect = canvas.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  return x >= 12 && x <= 180 && y >= 12 && y <= 180;
}

export function attachDragging(canvas, getCurrentWindow) {
  if (dragHandler) canvas.removeEventListener('mousedown', dragHandler);
  dragHandler = async (e) => {
    if (e.button !== 0) return;
    if (!insideCase(canvas, e)) return;
    // Suppress drag if a double-click is in progress; the dblclick handler
    // will open the bubble.
    if (e.detail >= 2) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (err) {
      console.warn('startDragging failed', err);
    }
  };
  canvas.addEventListener('mousedown', dragHandler);
}

export function attachDoubleClickToTalk(canvas, invoke) {
  if (dblHandler) canvas.removeEventListener('dblclick', dblHandler);
  dblHandler = (e) => {
    if (!insideCase(canvas, e)) return;
    invoke('open_bubble').catch((err) => console.warn('open_bubble failed', err));
  };
  canvas.addEventListener('dblclick', dblHandler);
}
