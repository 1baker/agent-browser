"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { getServiceRuns, type ServiceRunView, type ServiceRunsResponse } from "@agent-browser/client/service-runs";
import { SERVICE_API_BASE } from "@/lib/dashboard-api";
import { updateDashboardWorkspaceUrlSelection } from "@/lib/workspace-url-selection";
import { Button } from "@/components/ui/button";

const NEXT_ACTION_LABELS: Record<string, string> = {
  wait_for_dispatch: "Waiting to start",
  wait_for_result: "Working",
  verify_task_outcome: "Verify the task result",
  inspect_failure: "Inspect the failed action",
  open_exact_tab_and_recheck: "Open this tab and resolve the prompt",
  start_manual_seeding: "Start manual sign-in",
  finish_sign_in_and_close_browser: "Finish sign-in, then close that browser",
  verify_login_readiness: "Verify sign-in readiness",
  inspect_handoff_failure: "Inspect the sign-in handoff",
};

function runTitle(run: ServiceRunView): string {
  return run.taskName || run.targetServiceId || (run.kind === "human_challenge" ? "Page needs you" : "Browser activity");
}

export function TaskCenter({ onOpenWorkspace }: { onOpenWorkspace: () => void }) {
  const [collection, setCollection] = useState<ServiceRunsResponse | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const data = await getServiceRuns({ baseUrl: SERVICE_API_BASE });
      setCollection(data);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not load browser work.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  const needsHuman = useMemo(() => collection?.runs.filter((run) => run.state === "needs_human") ?? [], [collection]);
  const recent = useMemo(() => collection?.runs.filter((run) => run.state !== "needs_human") ?? [], [collection]);
  const selected = collection?.runs.find((run) => run.id === selectedId) ?? needsHuman[0] ?? recent[0] ?? null;

  const openWorkspace = (run: ServiceRunView) => {
    if (!run.browserId || !run.tabId || !run.targetId) return;
    updateDashboardWorkspaceUrlSelection({
      workspaceId: `browser:${run.browserId}`,
      browserId: run.browserId,
      sessionId: run.sessionName ?? null,
      tabId: run.tabId,
      profileId: run.profileId ?? null,
      jobId: run.jobId ?? null,
    });
    onOpenWorkspace();
  };

  return (
    <div className="h-full overflow-auto p-4 md:p-6" aria-label="Task workbench">
      <div className="mx-auto max-w-6xl space-y-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h1 className="text-xl font-semibold">Browser work</h1>
            <p className="mt-1 text-sm text-muted-foreground">See what needs your help and what the browser recently did.</p>
          </div>
          <Button type="button" variant="outline" onClick={() => void refresh()} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </Button>
        </div>
        {error && <p role="alert" className="rounded-lg border border-destructive/30 p-3 text-sm text-destructive">{error}</p>}
        <p className="text-xs text-muted-foreground">Recent browser actions are not verified task outcomes. Older jobs may no longer appear here.</p>
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(18rem,0.9fr)]">
          <div className="space-y-5">
            <section className="space-y-2" aria-labelledby="needs-you-title">
              <h2 id="needs-you-title" className="text-sm font-semibold">Needs you ({needsHuman.length})</h2>
              {needsHuman.length === 0 && <p className="rounded-lg border p-4 text-sm text-muted-foreground">No recorded human handoffs.</p>}
              {needsHuman.map((run) => (
                <button key={run.id} type="button" onClick={() => setSelectedId(run.id)}
                  className="block w-full rounded-lg border bg-card p-4 text-left hover:border-primary/50 focus-visible:outline-2 focus-visible:outline-primary">
                  <span className="block font-medium">{runTitle(run)}</span>
                  <span className="mt-1 block text-sm text-muted-foreground">{NEXT_ACTION_LABELS[run.nextAction] ?? run.nextAction}</span>
                </button>
              ))}
            </section>
            <section className="space-y-2" aria-labelledby="recent-work-title">
              <h2 id="recent-work-title" className="text-sm font-semibold">Recent activity ({recent.length})</h2>
              {recent.length === 0 && <p className="rounded-lg border p-4 text-sm text-muted-foreground">No labeled recent work.</p>}
              {recent.map((run) => (
                <button key={run.id} type="button" onClick={() => setSelectedId(run.id)}
                  className="block w-full rounded-lg border bg-card p-4 text-left hover:border-primary/50 focus-visible:outline-2 focus-visible:outline-primary">
                  <span className="block font-medium">{runTitle(run)}</span>
                  <span className="mt-1 block text-sm text-muted-foreground">{run.action ?? run.kind} · {NEXT_ACTION_LABELS[run.nextAction] ?? run.nextAction}</span>
                </button>
              ))}
            </section>
          </div>
          <aside className="rounded-lg border bg-card p-4" aria-label="Selected browser work">
            {selected ? (
              <div className="space-y-3">
                <h2 className="font-semibold">{runTitle(selected)}</h2>
                <p className="text-sm">Next: {NEXT_ACTION_LABELS[selected.nextAction] ?? selected.nextAction}</p>
                <dl className="space-y-1 break-all text-xs text-muted-foreground">
                  <div><dt className="inline font-medium">State: </dt><dd className="inline">{selected.state}</dd></div>
                  {selected.profileId && <div><dt className="inline font-medium">Profile: </dt><dd className="inline">{selected.profileId}</dd></div>}
                  {selected.browserId && <div><dt className="inline font-medium">Browser: </dt><dd className="inline">{selected.browserId}</dd></div>}
                  {selected.tabId && <div><dt className="inline font-medium">Tab: </dt><dd className="inline">{selected.tabId}</dd></div>}
                  {selected.url && <div><dt className="inline font-medium">Page: </dt><dd className="inline">{selected.url}</dd></div>}
                </dl>
                {selected.browserId && selected.tabId && selected.targetId && (
                  <Button type="button" onClick={() => openWorkspace(selected)}>Open exact workspace</Button>
                )}
                <p className="text-xs text-muted-foreground">Task outcome: unverified</p>
              </div>
            ) : <p className="text-sm text-muted-foreground">Select a work item to see its evidence and next action.</p>}
          </aside>
        </div>
      </div>
    </div>
  );
}
