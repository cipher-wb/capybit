// Birth ritual controller. Drives the 4 steps in birth.html via Rust commands.

const { invoke } = window.__TAURI__.core;

const steps = {
  welcome: document.getElementById('step-welcome'),
  scanning: document.getElementById('step-scanning'),
  reveal: document.getElementById('step-reveal'),
  name: document.getElementById('step-name'),
};

const pickBtn = document.getElementById('pickBtn');
const scanFolderEl = document.getElementById('scanFolder');
const excerptEl = document.getElementById('excerpt');
const sourceFileEl = document.getElementById('sourceFile');
const continueBtn = document.getElementById('continueBtn');
const rescanBtn = document.getElementById('rescanBtn');
const capyName = document.getElementById('capyName');
const userName = document.getElementById('userName');
const finishBtn = document.getElementById('finishBtn');
const finishError = document.getElementById('finishError');

let currentFolder = null;
let currentExcerpt = null; // { source_file, excerpt }

function showStep(name) {
  for (const [k, el] of Object.entries(steps)) {
    el.classList.toggle('active', k === name);
  }
}

pickBtn.addEventListener('click', async () => {
  try {
    const folder = await invoke('pick_birth_folder');
    if (!folder) return; // user cancelled
    currentFolder = folder;
    scanFolderEl.textContent = folder;
    showStep('scanning');
    await scanOnce();
  } catch (err) {
    alert('打开文件夹失败：' + err);
  }
});

async function scanOnce() {
  try {
    // Tiny artificial pause so the "翻看你写过的字" feels human-paced.
    await new Promise((r) => setTimeout(r, 900));
    const result = await invoke('scan_for_excerpt', { folder: currentFolder });
    currentExcerpt = result;
    excerptEl.textContent = result.excerpt;
    sourceFileEl.textContent = '出处：' + shorten(result.source_file);
    showStep('reveal');
  } catch (err) {
    alert(
      '这个文件夹里没找到能用的 .md / .txt 文字。\n' +
      '要不再选一个？\n\n详细：' + err,
    );
    showStep('welcome');
  }
}

rescanBtn.addEventListener('click', async () => {
  if (!currentFolder) return;
  showStep('scanning');
  await scanOnce();
});

continueBtn.addEventListener('click', () => {
  showStep('name');
  capyName.focus();
});

capyName.addEventListener('input', () => {
  finishBtn.disabled = capyName.value.trim().length === 0;
});

finishBtn.addEventListener('click', async () => {
  const name = capyName.value.trim();
  if (!name) return;
  finishBtn.disabled = true;
  finishError.hidden = true;
  try {
    await invoke('confirm_birth', {
      capybaraName: name,
      sourceFile: currentExcerpt.source_file,
      sourceExcerpt: currentExcerpt.excerpt,
      userNameForCapybit: userName.value.trim() || null,
    });
    // Rust closes this window and shows the main pet.
  } catch (err) {
    finishError.textContent = '保存失败：' + err;
    finishError.hidden = false;
    finishBtn.disabled = false;
  }
});

function shorten(path) {
  // Show only the last 2 segments for visual breathing room.
  const parts = path.replace(/\\/g, '/').split('/').filter(Boolean);
  if (parts.length <= 2) return path;
  return '…/' + parts.slice(-2).join('/');
}

showStep('welcome');
