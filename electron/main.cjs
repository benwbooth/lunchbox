const fs = require('fs');
const path = require('path');
const { pathToFileURL } = require('url');
const { spawn } = require('child_process');
const { app, BrowserWindow, shell } = require('electron');

const DEV_URL = process.env.LUNCHBOX_ELECTRON_URL || 'http://127.0.0.1:1420';
const BACKEND_READY_URL = 'http://127.0.0.1:3001/api/health';
const WINDOW_TITLE = 'Lunchbox';
const RETRY_DELAY_MS = 1000;
const SOURCE_WINDOW_ICON = path.join(__dirname, '..', 'backend', 'icons', 'icon.png');
const USE_STABLE_CHROMIUM = process.env.LUNCHBOX_STABLE_CHROMIUM === '1';
const USE_AGGRESSIVE_GPU = process.env.LUNCHBOX_AGGRESSIVE_GPU === '1';
const IS_RELEASE =
  app.isPackaged ||
  process.env.LUNCHBOX_RELEASE === '1' ||
  Boolean(process.env.LUNCHBOX_FRONTEND_DIR || process.env.LUNCHBOX_BACKEND_BIN);

function backendExecutableName() {
  return process.platform === 'win32' ? 'lunchbox-server.exe' : 'lunchbox-server';
}

function releaseFrontendDir() {
  return process.env.LUNCHBOX_FRONTEND_DIR || path.join(process.resourcesPath, 'frontend');
}

function releaseBackendBin() {
  return process.env.LUNCHBOX_BACKEND_BIN || path.join(process.resourcesPath, 'bin', backendExecutableName());
}

function releaseSharedDataDir() {
  return process.env.LUNCHBOX_SHARED_DATA_DIR || path.join(process.resourcesPath, 'share', 'lunchbox');
}

function windowIcon() {
  return process.env.LUNCHBOX_WINDOW_ICON || (app.isPackaged ? path.join(process.resourcesPath, 'icons', 'icon.png') : SOURCE_WINDOW_ICON);
}

function frontendUrl() {
  if (!IS_RELEASE) {
    return DEV_URL;
  }
  return pathToFileURL(path.join(releaseFrontendDir(), 'index.html')).toString();
}

let mainWindow = null;
let retryTimer = null;
let splashLoaded = false;
let showingSplash = false;
let appLoadInFlight = false;
let lastSplashState = null;
let backendProcess = null;
let quitting = false;

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

function pendingStatusLabel() {
  return IS_RELEASE ? 'Starting' : 'Compiling';
}

function buildSplashHtml(state) {
  const frontendStatus = state.frontendReady ? 'Ready' : pendingStatusLabel();
  const backendStatus = state.backendReady ? 'Ready' : pendingStatusLabel();
  const frontendClass = state.frontendReady ? 'ready' : 'pending';
  const backendClass = state.backendReady ? 'ready' : 'pending';
  const percent = Math.max(8, Math.min(100, Math.round((state.progress || 0.08) * 100)));
  const eyebrow = IS_RELEASE ? 'Lunchbox' : 'Lunchbox Development Shell';
  const title = IS_RELEASE ? 'Starting Lunchbox' : 'Starting Electron';
  const description = IS_RELEASE
    ? 'Lunchbox is starting its local backend and loading the packaged app.'
    : 'The slow part is compilation. The frontend WASM bundle and Rust backend are still building before the app can load.';

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
      letter-spacing: 0;
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
    <div class="eyebrow"><span class="dot"></span>${eyebrow}</div>
    <h1>${title}</h1>
    <p>${description}</p>
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
    const pendingStatusLabel = ${JSON.stringify(pendingStatusLabel())};
    window.__updateLunchboxSplash = (state) => {
      const percent = Math.max(8, Math.min(100, Math.round((state.progress || 0.08) * 100)));
      document.getElementById('progress-fill').style.width = percent + '%';
      document.getElementById('progress-label').textContent = percent + '% ready';
      const frontend = document.getElementById('frontend-state');
      frontend.textContent = state.frontendReady ? 'Ready' : pendingStatusLabel;
      frontend.className = 'state ' + (state.frontendReady ? 'ready' : 'pending');
      const backend = document.getElementById('backend-state');
      backend.textContent = state.backendReady ? 'Ready' : pendingStatusLabel;
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

async function frontendReady() {
  if (!IS_RELEASE) {
    return probeUrl(DEV_URL);
  }
  return fs.existsSync(path.join(releaseFrontendDir(), 'index.html'));
}

function startBundledBackend() {
  if (!IS_RELEASE || backendProcess) {
    return;
  }

  const backendBin = releaseBackendBin();
  if (!fs.existsSync(backendBin)) {
    console.error(`Bundled Lunchbox backend is missing: ${backendBin}`);
    return;
  }

  const env = {
    ...process.env,
    LUNCHBOX_RELEASE: '1',
    LUNCHBOX_SHARED_DATA_DIR: releaseSharedDataDir(),
  };

  backendProcess = spawn(backendBin, [], {
    cwd: path.dirname(backendBin),
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });

  backendProcess.stdout.on('data', (chunk) => {
    process.stdout.write(`[lunchbox-server] ${chunk}`);
  });

  backendProcess.stderr.on('data', (chunk) => {
    process.stderr.write(`[lunchbox-server] ${chunk}`);
  });

  backendProcess.on('error', (error) => {
    console.error(`Failed to launch bundled Lunchbox backend: ${error}`);
  });

  backendProcess.on('exit', (code, signal) => {
    backendProcess = null;
    if (!quitting) {
      console.error(`Bundled Lunchbox backend exited with code ${code} and signal ${signal}`);
      scheduleReload();
    }
  });
}

function stopBundledBackend() {
  quitting = true;
  clearRetryTimer();
  if (!backendProcess) {
    return;
  }
  const child = backendProcess;
  backendProcess = null;
  child.kill();
}

function computeSplashState(isFrontendReady, isBackendReady) {
  if (isFrontendReady && isBackendReady) {
    return {
      frontendReady: isFrontendReady,
      backendReady: isBackendReady,
      progress: 0.92,
      message: 'Frontend and backend are ready. Launching the app shell...',
    };
  }
  if (isFrontendReady) {
    return {
      frontendReady: isFrontendReady,
      backendReady: isBackendReady,
      progress: 0.7,
      message: 'Frontend is ready. Waiting for the backend API to start...',
    };
  }
  if (isBackendReady) {
    return {
      frontendReady: isFrontendReady,
      backendReady: isBackendReady,
      progress: 0.48,
      message: IS_RELEASE
        ? 'Backend is ready. Waiting for packaged frontend files...'
        : 'Backend is ready. Waiting for the frontend bundle to finish compiling...',
    };
  }
  return {
    frontendReady: isFrontendReady,
    backendReady: isBackendReady,
    progress: 0.18,
    message: IS_RELEASE
      ? 'Starting bundled services...'
      : 'Compiling the frontend bundle and backend server...',
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
    message: 'Loading Lunchbox...',
  });
  try {
    await mainWindow.loadURL(frontendUrl());
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
    if (IS_RELEASE) {
      startBundledBackend();
    }
    const [isFrontendReady, isBackendReady] = await Promise.all([
      frontendReady(),
      probeUrl(BACKEND_READY_URL),
    ]);
    const splashState = computeSplashState(isFrontendReady, isBackendReady);
    if (!showingSplash) {
      showSplash(splashState);
    } else {
      updateSplash(splashState);
    }
    if (isFrontendReady && isBackendReady) {
      tryLoadApp();
      return;
    }
    scheduleReload();
  }, RETRY_DELAY_MS);
}

function createWindow() {
  if (IS_RELEASE) {
    startBundledBackend();
  }

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
    icon: windowIcon(),
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
      message: IS_RELEASE
        ? `Waiting for packaged app resources... (${errorDescription})`
        : `Waiting for dev servers... (${errorDescription})`,
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
    message: IS_RELEASE ? 'Starting bundled services...' : 'Starting dev services...',
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

app.on('before-quit', () => {
  stopBundledBackend();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
