import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

// Exercise the real command builders with injected process execution; no browser,
// daemon, authentication, environment file, or desktop is touched.
const source = readFileSync(new URL('./open-rdp-guac-route-displays.js', import.meta.url), 'utf8');
const builders = source.slice(source.indexOf('function routeViewerArgs('), source.indexOf('function parseJson('));
const navigation = source.slice(source.indexOf('function navigateRoute('), source.indexOf('function waitForRouteDisplay('));
const calls = [];
const env = {};
const context = vm.createContext({
  process: { env },
  agentBrowserCommand: () => 'fixture-browser',
  agentBrowserTimeoutMs: 1000,
  routeNavigationTimeoutMs: 1000,
  parseJson: JSON.parse,
  commandResult: (command, args) => {
    calls.push({ command, args: Array.from(args) });
    return { status: 0, stdout: '{"success":true}', stderr: '' };
  },
});
vm.runInContext(`${builders}\n${navigation}`, context);
context.runAgentBrowser(['--session', 'route-a', 'open', 'about:blank'], 'fixture');
assert.deepEqual(calls.pop().args, ['--session', 'route-a', 'open', 'about:blank']);
env.AGENT_BROWSER_RDP_ROUTE_VIEWER_BROWSER_BUILD = 'stock_chrome';
for (const args of [
  ['--executable-path', '/installed/chrome', 'open', 'about:blank'],
  ['--session', 'route-a', 'set', 'headers', '{}'],
]) {
  context.runAgentBrowser(args, 'fixture');
  assert.deepEqual(calls.pop().args, ['--browser-build', 'stock_chrome', ...args]);
}
context.navigateRoute(['--session', 'route-a', 'open', 'http://localhost/fixture'], 'fixture');
assert.deepEqual(calls.pop().args, ['--browser-build', 'stock_chrome', '--session', 'route-a', 'open', 'http://localhost/fixture']);
env.AGENT_BROWSER_RDP_ROUTE_VIEWER_BROWSER_BUILD = 'invalid';
assert.throws(() => context.runAgentBrowser(['open', 'about:blank'], 'fixture'), /invalid_route_viewer_browser_build/);
assert.throws(() => context.navigateRoute(['open', 'about:blank'], 'fixture'), /invalid_route_viewer_browser_build/);
assert.equal(calls.length, 0);
console.log('Route viewer build: launch, follow-up, navigation, legacy default, and rejection passed.');
