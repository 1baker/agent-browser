import { existsSync, mkdtempSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

export function findLatestInstalledSmokeBrowser({
  homeDir = process.env.HOME,
  exists = existsSync,
  readDir = readdirSync,
} = {}) {
  if (!homeDir) return null;
  const browsersRoot = join(homeDir, '.agent-browser', 'browsers');
  let entries;
  try {
    entries = readDir(browsersRoot, { withFileTypes: true });
  } catch {
    return null;
  }
  const versions = entries
    .filter((entry) => entry.isDirectory() && entry.name.startsWith('chrome-'))
    .map((entry) => entry.name)
    .sort((left, right) => right.localeCompare(left, undefined, { numeric: true }));
  for (const version of versions) {
    const executable = join(browsersRoot, version, 'chrome');
    if (exists(executable)) return executable;
  }
  return null;
}

export function selectSmokeBrowserExecutable({
  configuredExecutable,
  fallbackExecutable = '/usr/bin/google-chrome',
  fallbackExists = existsSync,
} = {}) {
  const configured = String(configuredExecutable || '').trim();
  if (configured) return configured;
  return fallbackExecutable && fallbackExists(fallbackExecutable)
    ? fallbackExecutable
    : null;
}

export function isWslWindowsBrowserExecutable(executablePath) {
  return /^\/mnt\/[a-z]\/.+\.exe$/i.test(String(executablePath || '').trim());
}

export function resolveWslWindowsProfileRoot(executablePath) {
  const executable = String(executablePath || '').trim();
  const marker = '/AppData/Local/';
  const markerIndex = executable.indexOf(marker);
  if (!isWslWindowsBrowserExecutable(executable) || markerIndex < 0) return null;
  return `${executable.slice(0, markerIndex)}${marker}Temp`;
}

export function resolveDashboardSmokeBrowserCapability({
  launchConfig,
  requestedBrowserBuild = '',
  requestedProfile = '',
  defaultProfileRoot,
  exists = existsSync,
  createProfile = createDisposableSmokeProfile,
  profilePrefix = 'agent-browser-dashboard-smoke-',
} = {}) {
  const requestedBuild = String(requestedBrowserBuild || '').trim();
  const configuredBuild = String(launchConfig?.defaultBrowserBuild || '').trim();
  const browserBuild = requestedBuild || configuredBuild || null;
  const selectionSource = requestedBuild
    ? 'explicit_browser_build'
    : configuredBuild ? 'service_default_browser_build' : 'agent_browser_default';
  const explicitProfile = String(requestedProfile || '').trim();

  if (browserBuild !== 'stealthcdp_chromium') {
    return {
      status: 'ready',
      browserBuild,
      executablePath: null,
      selectionSource,
      profilePath: explicitProfile || null,
      profileSource: explicitProfile ? 'explicit' : 'agent_browser_default',
      disposableProfile: false,
      unsafeLaunchArgs: [],
    };
  }

  const manifest = launchConfig?.browserBuildManifests?.stealthcdp_chromium;
  const executablePath = String(manifest?.executablePath || launchConfig?.executablePath || '').trim();
  const failures = [];
  if (!manifest?.ready) failures.push('manifest_not_ready');
  if (!manifest?.manifestValid) failures.push('manifest_invalid');
  if (!manifest?.smokeSuccess) failures.push('artifact_smoke_not_passed');
  if (!executablePath) failures.push('executable_path_missing');
  if (executablePath && !exists(executablePath)) failures.push('executable_not_found');
  if (launchConfig?.executablePath && launchConfig.executablePath !== executablePath) {
    failures.push('resolved_executable_mismatch');
  }
  if (isWslWindowsBrowserExecutable(executablePath) && !launchConfig?.profileSmoke?.available) {
    failures.push('wsl_windows_profile_smoke_unavailable');
  }
  if (failures.length > 0) {
    return {
      status: 'unavailable',
      browserBuild,
      executablePath: executablePath || null,
      selectionSource,
      failures,
      profilePath: explicitProfile || null,
      profileSource: explicitProfile ? 'explicit' : null,
      disposableProfile: false,
      unsafeLaunchArgs: [],
    };
  }

  if (explicitProfile) {
    return {
      status: 'ready',
      browserBuild,
      executablePath,
      selectionSource,
      profilePath: explicitProfile,
      profileSource: 'explicit',
      disposableProfile: false,
      unsafeLaunchArgs: [],
    };
  }

  const windowsTempRoot = resolveWslWindowsProfileRoot(executablePath);
  const profilePath = createProfile({
    browserExecutable: executablePath,
    defaultRoot: defaultProfileRoot,
    windowsTempRoot,
    prefix: profilePrefix,
  });
  return {
    status: 'ready',
    browserBuild,
    executablePath,
    selectionSource,
    profilePath,
    profileSource: 'generated_disposable',
    disposableProfile: true,
    unsafeLaunchArgs: [],
  };
}

export function createDisposableSmokeProfile({
  browserExecutable,
  defaultRoot,
  prefix,
  windowsTempRoot,
}) {
  const root = isWslWindowsBrowserExecutable(browserExecutable)
    ? windowsTempRoot
    : defaultRoot;
  if (!root) {
    throw new Error('Could not resolve a writable disposable browser profile root');
  }
  return mkdtempSync(join(root, prefix));
}
