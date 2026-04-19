const path = require('path');
const { app, BrowserWindow, shell } = require('electron');

const DEV_URL = process.env.LUNCHBOX_ELECTRON_URL || 'http://127.0.0.1:1420';
const BACKEND_READY_URL = 'http://127.0.0.1:3001/api/health';
const WINDOW_TITLE = 'Lunchbox';
const RETRY_DELAY_MS = 1000;
const WINDOW_ICON = path.join(__dirname, '..', 'backend', 'icons', 'icon.png');
const USE_STABLE_CHROMIUM = process.env.LUNCHBOX_STABLE_CHROMIUM === '1';
const USE_AGGRESSIVE_GPU = process.env.LUNCHBOX_AGGRESSIVE_GPU === '1';

let mainWindow = null;
let retryTimer = null;
let splashLoaded = false;
let showingSplash = false;
let appLoadInFlight = false;
let lastSplashState = null;

if (USE_STABLE_CHROMIUM) {
  app.commandLine.appendSwitch('ozone-platform-hint', 'auto');
  app.commandLine.appendSwitch('disable-features', 'Vulkan');
} else {
  // Default to the least opinionated hardware path that still enables WebGPU.
  // Electron window creation has been unstable when Vulkan/platform selection is forced here.
  app.commandLine.appendSwitch('ozone-platform-hint', 'auto');
  app.commandLine.appendSwitch('enable-unsafe-webgpu');
  app.commandLine.appendSwitch('ignore-gpu-blocklist');
  if (USE_AGGRESSIVE_GPU) {
    app.commandLine.appendSwitch('ozone-platform', 'wayland');
    app.commandLine.appendSwitch('enable-features', 'UseOzonePlatform,Vulkan');
    app.commandLine.appendSwitch('use-vulkan');
    app.commandLine.appendSwitch('enable-webgpu-developer-features');
    app.commandLine.appendSwitch('disable-software-rasterizer');
  }
}

function clearRetryTimer() {
  if (retryTimer) {
    clearTimeout(retryTimer);
    retryTimer = null;
  }
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function buildSplashHtml(state) {
  const frontendStatus = state.frontendReady ? 'Ready' : 'Compiling';
  const backendStatus = state.backendReady ? 'Ready' : 'Compiling';
  const frontendClass = state.frontendReady ? 'ready' : 'pending';
  const backendClass = state.backendReady ? 'ready' : 'pending';
  const percent = Math.max(8, Math.min(100, Math.round((state.progress || 0.08) * 100)));

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${WINDOW_TITLE}</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #090909;
      --panel: #111111;
      --border: #262626;
      --text: #efefef;
      --muted: #929292;
      --accent: #f1c40f;
      --accent-2: #0077ff;
      --ready: #43c979;
    }
    * { box-sizing: border-box; }
    html, body {
      margin: 0;
      width: 100%;
      height: 100%;
      background: radial-gradient(circle at top, #141414 0%, var(--bg) 58%);
      color: var(--text);
      font-family: Inter, system-ui, sans-serif;
    }
    body {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 24px;
    }
    .shell {
      width: min(640px, 100%);
      padding: 28px 28px 24px;
      background: linear-gradient(180deg, rgba(255,255,255,0.03), rgba(255,255,255,0.015));
      border: 1px solid var(--border);
      border-radius: 18px;
      box-shadow: 0 24px 60px rgba(0,0,0,0.45);
    }
    .eyebrow {
      display: inline-flex;
      align-items: center;
      gap: 10px;
      color: var(--muted);
      font-size: 12px;
      letter-spacing: 0.12em;
      text-transform: uppercase;
    }
    .dot {
      width: 9px;
      height: 9px;
      border-radius: 999px;
      background: var(--accent);
      box-shadow: 0 0 18px rgba(241, 196, 15, 0.55);
      animation: pulse 1.1s ease-in-out infinite;
    }
    h1 {
      margin: 14px 0 10px;
      font-size: 36px;
      line-height: 1.05;
      letter-spacing: -0.03em;
    }
    p {
      margin: 0;
      color: var(--muted);
      font-size: 15px;
      line-height: 1.55;
    }
    .bar {
      margin-top: 22px;
      width: 100%;
      height: 12px;
      background: #171717;
      border: 1px solid #2a2a2a;
      border-radius: 999px;
      overflow: hidden;
    }
    .bar-fill {
      height: 100%;
      width: ${percent}%;
      background: linear-gradient(90deg, var(--accent), var(--accent-2));
      border-radius: inherit;
      transition: width 180ms ease;
      position: relative;
    }
    .bar-fill::after {
      content: "";
      position: absolute;
      inset: 0;
      background: linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent);
      transform: translateX(-100%);
      animation: shimmer 1.2s linear infinite;
    }
    .percent {
      margin-top: 10px;
      font-size: 12px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--muted);
    }
    .status {
      margin-top: 22px;
      display: grid;
      gap: 10px;
    }
    .row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 10px 12px;
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 12px;
      font-size: 14px;
    }
    .row strong {
      font-weight: 600;
    }
    .state {
      font-size: 12px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--muted);
    }
    .state.ready { color: var(--ready); }
    .footer {
      margin-top: 18px;
      min-height: 24px;
      font-size: 13px;
      color: #c5c5c5;
    }
    @keyframes pulse {
      0%, 100% { transform: scale(1); opacity: 0.95; }
      50% { transform: scale(1.25); opacity: 1; }
    }
    @keyframes shimmer {
      100% { transform: translateX(120%); }
    }
  </style>
</head>
<body>
  <div class="shell">
    <div class="eyebrow"><span class="dot"></span>Lunchbox Development Shell</div>
    <h1>Starting Electron</h1>
    <p>The slow part is compilation. The frontend WASM bundle and Rust backend are still building before the app can load.</p>
    <div class="bar"><div class="bar-fill" id="progress-fill"></div></div>
    <div class="percent" id="progress-label">${percent}% ready</div>
    <div class="status">
      <div class="row">
        <strong>Frontend</strong>
        <span class="state ${frontendClass}" id="frontend-state">${frontendStatus}</span>
      </div>
      <div class="row">
        <strong>Backend API</strong>
        <span class="state ${backendClass}" id="backend-state">${backendStatus}</span>
      </div>
    </div>
    <div class="footer" id="status-text">${escapeHtml(state.message)}</div>
  </div>
  <script>
    window.__updateLunchboxSplash = (state) => {
      const percent = Math.max(8, Math.min(100, Math.round((state.progress || 0.08) * 100)));
      document.getElementById('progress-fill').style.width = percent + '%';
      document.getElementById('progress-label').textContent = percent + '% ready';
      const frontend = document.getElementById('frontend-state');
      frontend.textContent = state.frontendReady ? 'Ready' : 'Compiling';
      frontend.className = 'state ' + (state.frontendReady ? 'ready' : 'pending');
      const backend = document.getElementById('backend-state');
      backend.textContent = state.backendReady ? 'Ready' : 'Compiling';
      backend.className = 'state ' + (state.backendReady ? 'ready' : 'pending');
      document.getElementById('status-text').textContent = state.message;
    };
  </script>
</body>
</html>`;
}

function setWindowProgress(progress) {
  if (!mainWindow || mainWindow.isDestroyed()) {
    return;
  }
  if (progress == null) {
    mainWindow.setProgressBar(-1);
    return;
  }
  mainWindow.setProgressBar(Math.max(0.02, Math.min(1, progress)));
}

function showSplash(state) {
  lastSplashState = state;
  showingSplash = true;
  splashLoaded = false;
  setWindowProgress(state.progress);
  if (!mainWindow || mainWindow.isDestroyed()) {
    return;
  }
  const html = buildSplashHtml(state);
  const url = `data:text/html;charset=UTF-8,${encodeURIComponent(html)}`;
  mainWindow.loadURL(url).catch((error) => {
    console.error(`Failed to load Electron splash screen: ${error}`);
  });
}

function updateSplash(state) {
  lastSplashState = state;
  setWindowProgress(state.progress);
  if (!mainWindow || mainWindow.isDestroyed()) {
    return;
  }
  if (!showingSplash || !splashLoaded) {
    return;
  }
  const payload = JSON.stringify(state);
  mainWindow.webContents.executeJavaScript(`window.__updateLunchboxSplash(${payload});`, true).catch(() => {});
}

async function probeUrl(url) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 700);
  try {
    const response = await fetch(url, {
      method: 'GET',
      signal: controller.signal,
      cache: 'no-store',
    });
    return response.ok;
  } catch (_error) {
    return false;
  } finally {
    clearTimeout(timeout);
  }
}

function computeSplashState(frontendReady, backendReady) {
  if (frontendReady && backendReady) {
    return {
      frontendReady,
      backendReady,
      progress: 0.92,
      message: 'Frontend and backend are ready. Launching the app shell…',
    };
  }
  if (frontendReady) {
    return {
      frontendReady,
      backendReady,
      progress: 0.7,
      message: 'Frontend is ready. Waiting for the backend API to start…',
    };
  }
  if (backendReady) {
    return {
      frontendReady,
      backendReady,
      progress: 0.48,
      message: 'Backend is ready. Waiting for the frontend bundle to finish compiling…',
    };
  }
  return {
    frontendReady,
    backendReady,
    progress: 0.18,
    message: 'Compiling the frontend bundle and backend server…',
  };
}

async function tryLoadApp() {
  if (!mainWindow || mainWindow.isDestroyed() || appLoadInFlight) {
    return;
  }
  appLoadInFlight = true;
  updateSplash({
    frontendReady: true,
    backendReady: true,
    progress: 0.97,
    message: 'Loading Lunchbox…',
  });
  try {
    await mainWindow.loadURL(DEV_URL);
  } catch (error) {
    console.error(`Initial Electron load failed: ${error}`);
    appLoadInFlight = false;
    scheduleReload();
  }
}

function scheduleReload() {
  clearRetryTimer();
  retryTimer = setTimeout(async () => {
    if (!mainWindow || mainWindow.isDestroyed()) {
      return;
    }
    const [frontendReady, backendReady] = await Promise.all([
      probeUrl(DEV_URL),
      probeUrl(BACKEND_READY_URL),
    ]);
    const splashState = computeSplashState(frontendReady, backendReady);
    if (!showingSplash) {
      showSplash(splashState);
    } else {
      updateSplash(splashState);
    }
    if (frontendReady && backendReady) {
      tryLoadApp();
      return;
    }
    scheduleReload();
  }, RETRY_DELAY_MS);
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1600,
    height: 980,
    minWidth: 1100,
    minHeight: 720,
    show: true,
    autoHideMenuBar: true,
    title: WINDOW_TITLE,
    backgroundColor: '#121212',
    frame: true,
    icon: WINDOW_ICON,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      spellcheck: false,
    },
  });

  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url);
    return { action: 'deny' };
  });

  mainWindow.webContents.on('did-finish-load', () => {
    const currentUrl = mainWindow?.webContents.getURL() || '';
    if (currentUrl.startsWith('data:text/html')) {
      splashLoaded = true;
      if (lastSplashState) {
        updateSplash(lastSplashState);
      }
      return;
    }
    clearRetryTimer();
    appLoadInFlight = false;
    showingSplash = false;
    splashLoaded = false;
    setTimeout(() => setWindowProgress(null), 250);
  });

  mainWindow.webContents.on('did-fail-load', (_event, errorCode, errorDescription, validatedURL, isMainFrame) => {
    if (!isMainFrame) {
      return;
    }
    console.error(`Electron failed to load ${validatedURL}: [${errorCode}] ${errorDescription}`);
    appLoadInFlight = false;
    showSplash({
      frontendReady: false,
      backendReady: false,
      progress: 0.18,
      message: `Waiting for dev servers… (${errorDescription})`,
    });
    scheduleReload();
  });

  mainWindow.on('closed', () => {
    clearRetryTimer();
    mainWindow = null;
  });

  showSplash({
    frontendReady: false,
    backendReady: false,
    progress: 0.08,
    message: 'Starting dev services…',
  });
  scheduleReload();
}

app.setName(WINDOW_TITLE);

app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
