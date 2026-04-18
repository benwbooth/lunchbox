const path = require('path');
const { app, BrowserWindow, shell } = require('electron');

const DEV_URL = process.env.LUNCHBOX_ELECTRON_URL || 'http://127.0.0.1:1420';
const WINDOW_TITLE = 'Lunchbox';
const RETRY_DELAY_MS = 1000;
const WINDOW_ICON = path.join(__dirname, '..', 'src-tauri', 'icons', 'icon.png');
const USE_STABLE_CHROMIUM = process.env.LUNCHBOX_STABLE_CHROMIUM === '1';
const USE_AGGRESSIVE_GPU = process.env.LUNCHBOX_AGGRESSIVE_GPU === '1';

let mainWindow = null;
let retryTimer = null;

if (USE_STABLE_CHROMIUM) {
  app.commandLine.appendSwitch('ozone-platform-hint', 'auto');
  app.commandLine.appendSwitch('disable-features', 'Vulkan');
} else {
  // Middle ground for Linux/Electron:
  // - keep WebGPU enabled
  // - allow Vulkan-backed hardware adapters
  // - bypass Chromium's conservative GPU blocklist
  // - avoid forcing Vulkan compositor mode or disabling fallback paths
  app.commandLine.appendSwitch('ozone-platform-hint', 'auto');
  app.commandLine.appendSwitch('enable-unsafe-webgpu');
  app.commandLine.appendSwitch('ignore-gpu-blocklist');
  app.commandLine.appendSwitch('enable-features', 'Vulkan');
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

function scheduleReload() {
  clearRetryTimer();
  retryTimer = setTimeout(() => {
    if (!mainWindow || mainWindow.isDestroyed()) {
      return;
    }
    mainWindow.loadURL(DEV_URL).catch(() => {
      scheduleReload();
    });
  }, RETRY_DELAY_MS);
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1600,
    height: 980,
    minWidth: 1100,
    minHeight: 720,
    show: false,
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
    clearRetryTimer();
    if (mainWindow && !mainWindow.isVisible()) {
      mainWindow.show();
    }
  });

  mainWindow.webContents.on('did-fail-load', (_event, errorCode, errorDescription, validatedURL, isMainFrame) => {
    if (!isMainFrame) {
      return;
    }
    console.error(`Electron failed to load ${validatedURL}: [${errorCode}] ${errorDescription}`);
    scheduleReload();
  });

  mainWindow.on('closed', () => {
    clearRetryTimer();
    mainWindow = null;
  });

  mainWindow.loadURL(DEV_URL).catch((error) => {
    console.error(`Initial Electron load failed: ${error}`);
    scheduleReload();
  });
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
