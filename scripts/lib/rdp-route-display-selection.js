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
