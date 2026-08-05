import {
  canOpenControlViewStream,
  canOpenViewStream,
  viewStreamReadinessLabel,
  type ServiceViewStream,
} from "./service-view-streams.ts";

/**
 * Combines stream projections reported by the service and daemon while
 * keeping the first authoritative copy of an overlapping source.
 */
export function mergeWorkspaceViewStreams(
  primary?: ServiceViewStream[],
  secondary?: ServiceViewStream[],
): ServiceViewStream[] {
  const merged: ServiceViewStream[] = [];
  const seen = new Set<string>();
  for (const stream of [...(primary ?? []), ...(secondary ?? [])]) {
    const identity = workspaceViewStreamKey(stream, 0);
    if (seen.has(identity)) continue;
    seen.add(identity);
    merged.push(stream);
  }
  return merged;
}

/**
 * Returns the stable operator-choice key used to remember one stream for a
 * browser without making a provider URL part of the dashboard route.
 */
export function workspaceViewStreamKey(stream: ServiceViewStream, index = 0): string {
  const id = stream.id?.trim();
  if (id) return `id:${id}`;
  return [
    `provider:${stream.provider?.trim().toLowerCase() || "unknown"}`,
    `route:${stream.routeId?.trim() || stream.connectionId?.trim() || "unrouted"}`,
    `index:${index}`,
  ].join("|");
}

/**
 * Scores the automatic stream fallback. Explicit operator selection is
 * handled separately and always wins while that stream remains available.
 */
export function workspaceViewStreamScore(stream: ServiceViewStream): number {
  const provider = stream.provider?.trim().toLowerCase() ?? "";
  const routeSource = stream.routeSource?.trim().toLowerCase() ?? "";
  const providerMode = stream.providerMode?.trim().toLowerCase() ?? "";
  const displayAllocationId = stream.displayAllocationId?.trim().toLowerCase() ?? "";
  let score = 0;
  if (canOpenViewStream(stream)) score += 80;
  if (provider === "rdp_gateway") score += 20;
  if (canOpenControlViewStream(stream)) score += 15;
  if (stream.routeId || stream.connectionId || stream.connectionName) score += 20;
  if (displayAllocationId) score += 10;
  if (displayAllocationId && !displayAllocationId.includes("shared")) score += 35;
  if (routeSource === "pool" || routeSource === "generated" || routeSource === "discovered") score += 40;
  if (providerMode === "simultaneous_view") score += 20;
  if (providerMode === "single_controller") score += 10;
  if (viewStreamReadinessLabel(stream) === "ready") score += 10;
  return score;
}

/**
 * Orders every reported stream for display while preserving the ability to
 * select a lower-scored source such as CDP when RDP is the default.
 */
export function workspaceViewStreamChoices(streams?: ServiceViewStream[]): ServiceViewStream[] {
  return [...(streams ?? [])].sort((left, right) => workspaceViewStreamScore(right) - workspaceViewStreamScore(left));
}

/** Selects the remembered stream when present, otherwise the best fallback. */
export function selectWorkspaceViewStream(
  streams?: ServiceViewStream[],
  preferredKey?: string | null,
): ServiceViewStream | null {
  const choices = workspaceViewStreamChoices(streams);
  if (choices.length === 0) return null;
  if (preferredKey) {
    const preferred = choices.find((stream, index) => workspaceViewStreamKey(stream, index) === preferredKey);
    if (preferred) return preferred;
  }
  return choices[0] ?? null;
}

/**
 * Chooses the service-owned recovery action for a retained browser. Missing
 * stream metadata is recoverable through reattachment and never implies that
 * the dashboard should launch a duplicate browser process.
 */
export function workspaceViewRecoveryAction({
  browserAttachability,
  streamAttachability,
}: {
  browserAttachability?: unknown;
  streamAttachability?: unknown;
}): "service_remote_view_browser_reattach" | "service_remote_view_route_switch" {
  const recommendedAction = [streamAttachability, browserAttachability]
    .map((value) => value && typeof value === "object" ? (value as Record<string, unknown>).recommendedAction : null)
    .find((value): value is string => typeof value === "string" && value.trim().length > 0);
  return recommendedAction === "service_remote_view_route_switch"
    ? "service_remote_view_route_switch"
    : "service_remote_view_browser_reattach";
}
