export type WorkspaceBrowserSelection = {
  workspaceId?: string | null;
  browserId?: string | null;
  sessionId?: string | null;
};

export type WorkspaceBrowserSelectionCandidate = {
  id: string;
  activeSessionIds?: string[];
};

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
