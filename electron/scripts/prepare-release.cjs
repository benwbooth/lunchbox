const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const electronDir = path.resolve(__dirname, '..');
const repoRoot = path.resolve(electronDir, '..');
const releaseResources = path.join(electronDir, 'release-resources');
const frontendOut = path.join(releaseResources, 'frontend');
const backendOut = path.join(releaseResources, 'bin');
const sharedOut = path.join(releaseResources, 'share', 'lunchbox');
const iconsOut = path.join(releaseResources, 'icons');

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    stdio: 'inherit',
    shell: process.platform === 'win32',
    ...options,
  });
  if (result.status !== 0) {
    process.exit(result.status || 1);
  }
}

function copyDir(src, dest) {
  fs.rmSync(dest, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.cpSync(src, dest, { recursive: true });
}

function copyFile(src, dest) {
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(src, dest);
}

function cargoTargetDir() {
  if (process.env.CARGO_TARGET_DIR) {
    return path.resolve(repoRoot, process.env.CARGO_TARGET_DIR);
  }
  return path.join(repoRoot, 'target');
}

function backendBinaryPath() {
  const exe = process.platform === 'win32' ? 'dev_server.exe' : 'dev_server';
  return path.join(cargoTargetDir(), 'release', exe);
}

function packagedBackendName() {
  return process.platform === 'win32' ? 'lunchbox-server.exe' : 'lunchbox-server';
}

fs.rmSync(releaseResources, { recursive: true, force: true });
fs.mkdirSync(releaseResources, { recursive: true });

run('cargo', ['build', '--release', '-p', 'lunchbox', '--bin', 'dev_server']);
run('trunk', ['build', '--release', '--public-url', './'], {
  env: {
    ...process.env,
    NO_COLOR: 'false',
  },
});

copyDir(path.join(repoRoot, 'dist'), frontendOut);
copyFile(backendBinaryPath(), path.join(backendOut, packagedBackendName()));
copyDir(path.join(repoRoot, 'backend', 'icons'), iconsOut);

fs.mkdirSync(sharedOut, { recursive: true });
for (const entry of fs.readdirSync(path.join(repoRoot, 'db'))) {
  if (!entry.endsWith('.db') && !entry.endsWith('.db.zst')) {
    continue;
  }
  copyFile(path.join(repoRoot, 'db', entry), path.join(sharedOut, entry));
}

console.log(`Prepared Electron release resources in ${releaseResources}`);
