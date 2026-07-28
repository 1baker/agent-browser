import { fileURLToPath } from 'node:url';

/**
 * Select the authoritative display for a managed RDP route.
 *
 * Current Xorg inspection is stronger evidence than a persisted allocation
 * hint because XRDP chooses the display number when the session is created.
 */
export function selectRouteDisplayName({
  configuredDisplayName,
  inferredDisplayName,
}) {
  return inferredDisplayName || configuredDisplayName || null;
}

/**
 * Resolve the display inspector independently of the caller's working
 * directory. The remote-view doctor intentionally runs helper scripts with
 * the scripts directory as cwd.
 */
export function routeDisplayInspectorPath(moduleUrl) {
  return fileURLToPath(new URL('inspect-rdp-route-displays.js', moduleUrl));
}
