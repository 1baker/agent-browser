#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  createDisposableSmokeProfile,
  findLatestInstalledSmokeBrowser,
  isWslWindowsBrowserExecutable,
  resolveDashboardSmokeBrowserCapability,
  resolveWslWindowsProfileRoot,
  selectSmokeBrowserExecutable,
} from './lib/smoke-browser-fixture.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-smoke-fixture-test-'));
const linuxRoot = mkdtempSync(join(root, 'linux-'));
const windowsRoot = mkdtempSync(join(root, 'windows-'));
const dashboardSmoke = readFileSync('scripts/smoke-local-dashboard-runtime.js', 'utf8');

try {
  const installedBrowserRoot = join(root, 'installed-home', '.agent-browser', 'browsers');
  for (const version of ['chrome-152.0.7977.9', 'chrome-152.0.7977.64']) {
    const versionRoot = join(installedBrowserRoot, version);
    mkdirSync(versionRoot, { recursive: true });
    writeFileSync(join(versionRoot, 'chrome'), 'fixture');
  }
  assert.equal(
    findLatestInstalledSmokeBrowser({ homeDir: join(root, 'installed-home') }),
    join(installedBrowserRoot, 'chrome-152.0.7977.64', 'chrome'),
    'installed smoke browser selection should use numeric version order',
  );

  const selected = selectSmokeBrowserExecutable({
    configuredExecutable: '/mnt/c/Users/Baker/AppData/Local/chromium/chrome.exe',
    fallbackExecutable: '/usr/bin/google-chrome',
    fallbackExists: () => true,
  });
  assert.equal(
    selected,
    '/mnt/c/Users/Baker/AppData/Local/chromium/chrome.exe',
    'an installed Linux fallback must not replace the configured executable',
  );
  assert.equal(
    selectSmokeBrowserExecutable({
      fallbackExecutable: '/usr/bin/google-chrome',
      fallbackExists: () => true,
    }),
    '/usr/bin/google-chrome',
  );
  assert.equal(
    selectSmokeBrowserExecutable({ fallbackExists: () => false }),
    null,
  );
  assert.equal(isWslWindowsBrowserExecutable(selected), true);
  assert.equal(isWslWindowsBrowserExecutable('/usr/bin/google-chrome'), false);
  assert.equal(
    resolveWslWindowsProfileRoot('/mnt/c/Users/Baker/AppData/Local/chromium/chrome.exe'),
    '/mnt/c/Users/Baker/AppData/Local/Temp',
  );
  assert.equal(resolveWslWindowsProfileRoot('/usr/bin/google-chrome'), null);

  const windowsProfile = createDisposableSmokeProfile({
    browserExecutable: selected,
    defaultRoot: linuxRoot,
    windowsTempRoot: windowsRoot,
    prefix: 'cdp-stream-',
  });
  assert.equal(windowsProfile.startsWith(`${windowsRoot}/cdp-stream-`), true);

  const linuxProfile = createDisposableSmokeProfile({
    browserExecutable: '/usr/bin/google-chrome',
    defaultRoot: linuxRoot,
    windowsTempRoot: windowsRoot,
    prefix: 'cdp-stream-',
  });
  assert.equal(linuxProfile.startsWith(`${linuxRoot}/cdp-stream-`), true);

  const launchConfig = {
    defaultBrowserBuild: 'stealthcdp_chromium',
    executablePath: selected,
    profileSmoke: { available: true },
    browserBuildManifests: {
      stealthcdp_chromium: {
        ready: true,
        manifestValid: true,
        smokeSuccess: true,
        executablePath: selected,
      },
    },
  };
  const capability = resolveDashboardSmokeBrowserCapability({
    launchConfig,
    defaultProfileRoot: linuxRoot,
    exists: () => true,
    createProfile: (input) => ({ ...input, path: `${windowsRoot}/dashboard-profile` }).path,
  });
  assert.deepEqual(capability, {
    status: 'ready',
    browserBuild: 'stealthcdp_chromium',
    executablePath: selected,
    selectionSource: 'service_default_browser_build',
    profilePath: `${windowsRoot}/dashboard-profile`,
    profileSource: 'generated_disposable',
    disposableProfile: true,
    unsafeLaunchArgs: [],
  });

  const staleCapability = resolveDashboardSmokeBrowserCapability({
    launchConfig: {
      ...launchConfig,
      executablePath: '/mnt/c/Users/Baker/AppData/Local/other/chrome.exe',
      profileSmoke: { available: false },
      browserBuildManifests: {
        stealthcdp_chromium: {
          ready: false,
          manifestValid: true,
          smokeSuccess: false,
          executablePath: selected,
        },
      },
    },
    defaultProfileRoot: linuxRoot,
    exists: () => false,
  });
  assert.equal(staleCapability.status, 'unavailable');
  assert.deepEqual(staleCapability.failures, [
    'manifest_not_ready',
    'artifact_smoke_not_passed',
    'executable_not_found',
    'resolved_executable_mismatch',
    'wsl_windows_profile_smoke_unavailable',
  ]);
  assert.deepEqual(staleCapability.unsafeLaunchArgs, []);

  const explicitProfile = resolveDashboardSmokeBrowserCapability({
    launchConfig,
    requestedBrowserBuild: 'stealthcdp_chromium',
    requestedProfile: '/mnt/c/Users/Baker/AppData/Local/Temp/reviewed-profile',
    defaultProfileRoot: linuxRoot,
    exists: () => true,
  });
  assert.equal(explicitProfile.selectionSource, 'explicit_browser_build');
  assert.equal(explicitProfile.profileSource, 'explicit');
  assert.equal(explicitProfile.disposableProfile, false);
  assert.match(
    dashboardSmoke,
    /service', 'status'[\s\S]*resolveDashboardSmokeBrowserCapability/,
    'dashboard smoke must resolve installed launch capability',
  );
  assert.match(
    dashboardSmoke,
    /service', 'status'[\s\S]*maxBuffer: 16 \* 1024 \* 1024/,
    'dashboard smoke must capture rich service status without ENOBUFS',
  );
  assert.match(
    dashboardSmoke,
    /rmSync\(capability\.profilePath/,
    'dashboard smoke must remove its disposable profile',
  );
  assert.match(
    dashboardSmoke,
    /command\.push\('--browser-build', options\.browserBuild\)/,
    'dashboard smoke must pass the resolved browser build through agent-browser',
  );
  assert.doesNotMatch(
    dashboardSmoke,
    /--no-sandbox/,
    'dashboard smoke must not add a general no-sandbox launch argument',
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Smoke browser fixture tests passed');
