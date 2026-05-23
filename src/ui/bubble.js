// Bubble window controller.
// Talks to Rust via Tauri 2 global API (`window.__TAURI__`).

const tauri = window.__TAURI__;
const invoke = tauri.core.invoke;
const listen = tauri.event.listen;

const input = document.getElementById('input');
const reply = document.getElementById('reply');
const status = document.getElementById('status');
const closeBtn = document.getElementById('closeBtn');
const proactivePanel = document.getElementById('proactive');
const proactiveText = document.getElementById('proactiveText');
const respondBtn = document.getElementById('respondBtn');
const laterBtn = document.getElementById('laterBtn');

let activeRequestId = null;
let typingTarget = ''; // full text received so far for the active request
let displayedChars = 0;
let typingTimer = null;
const TYPING_INTERVAL_MS = 22; // PRD §3.2.2 "温柔语速"

input.focus();
window.addEventListener('focus', () => input.focus());

input.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    submit();
  } else if (e.key === 'Escape') {
    invoke('close_bubble').catch(() => {});
  }
});

closeBtn.addEventListener('click', () => {
  invoke('close_bubble').catch(() => {});
});

async function submit() {
  const text = input.value.trim();
  if (!text) return;
  input.value = '';
  input.disabled = true;

  activeRequestId = crypto.randomUUID();
  typingTarget = '';
  displayedChars = 0;
  reply.hidden = false;
  reply.textContent = '';
  status.textContent = '…思考中';

  try {
    await invoke('send_message', { requestId: activeRequestId, message: text });
  } catch (err) {
    status.textContent = '出错: ' + (err?.message ?? err);
    input.disabled = false;
  }
}

// Render the typing buffer one char at a time for the "温柔语速" effect.
function pumpTyping() {
  if (typingTimer) return;
  typingTimer = setInterval(() => {
    if (displayedChars >= typingTarget.length) {
      clearInterval(typingTimer);
      typingTimer = null;
      return;
    }
    displayedChars += 1;
    reply.textContent = typingTarget.slice(0, displayedChars);
    reply.scrollTop = reply.scrollHeight;
  }, TYPING_INTERVAL_MS);
}

listen('llm-chunk', (event) => {
  const { request_id, delta } = event.payload;
  if (request_id !== activeRequestId) return;
  typingTarget += delta;
  status.textContent = '…在说';
  pumpTyping();
});

listen('llm-done', (event) => {
  const { request_id, prompt_tokens, completion_tokens } = event.payload;
  if (request_id !== activeRequestId) return;
  // Drain remainder of typing buffer immediately if the stream finished
  // faster than the typing animation.
  const flush = () => {
    if (displayedChars < typingTarget.length) {
      requestAnimationFrame(flush);
    } else {
      const cost = `${prompt_tokens}+${completion_tokens} tok`;
      status.textContent = cost;
      input.disabled = false;
      input.focus();
    }
  };
  flush();
});

// --- Proactive opener flow ----------------------------------------
// Rust scheduler fires `proactive-opener` when urge > 70 + !in_flow + cooldown.
// We open the bubble (Rust also calls open_bubble before emit? No — we do it
// here to ensure focus order works) and show the opener with two buttons.
listen('proactive-opener', async (event) => {
  const text = event.payload?.text ?? '';
  if (!text) return;
  proactiveText.textContent = text;
  proactivePanel.hidden = false;
  reply.hidden = true;
  status.textContent = '·';
  input.value = '';
  try {
    // Make sure the bubble is open and focused. If already open this is a no-op.
    await invoke('open_bubble');
  } catch (err) {
    console.warn('open_bubble failed', err);
  }
});

respondBtn.addEventListener('click', () => {
  proactivePanel.hidden = true;
  input.focus();
});

laterBtn.addEventListener('click', async () => {
  proactivePanel.hidden = true;
  try {
    await invoke('dismiss_proactive');
    await invoke('close_bubble');
  } catch (err) {
    console.warn('dismiss_proactive failed', err);
  }
});

listen('llm-error', (event) => {
  const { request_id, message } = event.payload;
  if (request_id !== activeRequestId) return;
  if (typingTimer) {
    clearInterval(typingTimer);
    typingTimer = null;
  }
  reply.hidden = false;
  reply.textContent = '出错了：' + message;
  status.textContent = '失败';
  input.disabled = false;
  input.focus();
});
