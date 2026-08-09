import type { ServiceViewStream } from "./service-view-streams.ts";

export type WorkspaceBrowserSelection = {
  workspaceId?: string | null;
  browserId?: string | null;
  sessionId?: string | null;
};

export type WorkspaceBrowserSelectionCandidate = {
  id: string;
  activeSessionIds?: string[];
};

export type WorkspaceSelectedContextCandidate = {
  node?: { id: string } | null;
  stream?: WorkspaceSelectedContextStreamCandidate | null;
};

export type WorkspaceSelectedContextStreamCandidate = {
  provider?: string | null;
  url?: string | null;
  routeId?: string | null;
  displayAllocationId?: string | null;
  connectionId?: string | null;
  connectionName?: string | null;
  routeSource?: string | null;
  providerMode?: string | null;
  viewerLeaseIds?: string[];
  controllerLeaseId?: string | null;
  readOnly?: boolean;
  controlInput?: string | null;
  embeddable?: boolean;
  controllable?: boolean;
  operatorVisibleState?: string | null;
  operatorVisibleReason?: string | null;
  routeSummary?: string | null;
};

export type WorkspaceSelectedContextRouteCandidate = ServiceViewStream & {
  id?: string | null;
};

/**
 * Returns whether navigator-owned context can supply the viewport directly.
 * This includes detected manual-runtime RDP streams that intentionally have
 * no matching service-owned browser record.
 */
export function selectedWorkspaceContextCanRenderViewport(
  context?: WorkspaceSelectedContextCandidate | null,
): boolean {
  return Boolean(context?.node && context.stream?.url?.trim() && context.stream.embeddable);
}

/**
 * Restores provider route URLs that are intentionally not duplicated onto a
 * navigator node. Public dashboards need the route's external URL while local
 * dashboards continue to use the loopback frame URL.
 */
export function serviceViewStreamForSelectedWorkspaceContext(
  nodeId: string,
  stream: WorkspaceSelectedContextStreamCandidate,
  route?: WorkspaceSelectedContextRouteCandidate | null,
): ServiceViewStream | null {
  if (!stream.url?.trim() || !stream.embeddable) return null;
  return {
    id: `selected:${nodeId}:${stream.provider ?? "stream"}`,
    provider: stream.provider ?? route?.provider ?? undefined,
    controlInput: stream.controlInput ?? route?.controlInput ?? null,
    url: stream.url,
    frameUrl: route?.frameUrl ?? stream.url,
    externalUrl: route?.externalUrl ?? stream.url,
    routeDescriptor: route?.routeDescriptor ?? null,
    routeId: stream.routeId ?? route?.routeId ?? route?.id ?? null,
    displayAllocationId: stream.displayAllocationId ?? route?.displayAllocationId ?? null,
    connectionId: stream.connectionId ?? route?.connectionId ?? null,
    connectionName: stream.connectionName ?? route?.connectionName ?? null,
    routeSource: stream.routeSource ?? route?.routeSource ?? null,
    providerMode: stream.providerMode ?? route?.providerMode ?? null,
    viewerLeaseIds: stream.viewerLeaseIds ?? route?.viewerLeaseIds,
    controllerLeaseId: stream.controllerLeaseId ?? route?.controllerLeaseId ?? null,
    readOnly: stream.readOnly ?? route?.readOnly,
    readiness: {
      state: stream.operatorVisibleState,
      reason: stream.operatorVisibleReason ?? stream.routeSummary ?? undefined,
    },
  };
}

export function serviceBrowserForWorkspaceSelection<
  T extends WorkspaceBrowserSelectionCandidate,
>(
  browsers: T[],
  selection: WorkspaceBrowserSelection,
): T | null {
  const explicitBrowserId = selection.browserId?.trim()
    || browserIdFromWorkspaceId(selection.workspaceId);
  if (explicitBrowserId) {
    const exactBrowser = browsers.find((browser) => browser.id === explicitBrowserId);
    if (exactBrowser) return exactBrowser;
  }

  const sessionId = selection.sessionId?.trim()
    || sessionIdFromWorkspaceId(selection.workspaceId);
  if (!sessionId) return null;
  return browsers.find((browser) => browser.activeSessionIds?.includes(sessionId)) ?? null;
}

function browserIdFromWorkspaceId(workspaceId?: string | null): string | null {
  const prefix = "browser:";
  return workspaceId?.startsWith(prefix) ? workspaceId.slice(prefix.length) : null;
}

function sessionIdFromWorkspaceId(workspaceId?: string | null): string | null {
  const prefix = "daemon-session:";
  return workspaceId?.startsWith(prefix) ? workspaceId.slice(prefix.length) : null;
}
