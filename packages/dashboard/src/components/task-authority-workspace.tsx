"use client";

import { useMemo, useState } from "react";
import { AlertTriangle, CheckCircle2, Database, FileWarning, GitBranch, Loader2, RefreshCw, ShieldCheck } from "lucide-react";
import { SERVICE_API_BASE } from "@/lib/dashboard-api";
import {
  parseTaskAuthorityReplacementPlan,
  taskAuthorityOutcomeCounts,
  taskAuthorityRecoveryEligibility,
  type ServiceTaskAuthorityCollection,
  type ServiceTaskAuthorityLiveTab,
  type ServiceTaskAuthorityStatus,
  type TaskAuthorityRecoveryEligibility,
} from "@/lib/service-task-authorities";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

type ApiResponse = {
  success?: boolean;
  data?: Record<string, unknown>;
  error?: string | null;
};

type ConfirmationEvidence = {
  confirmationId: string;
  targetId: string;
  url: string;
  expiresInMs: number | null;
};

type ConfirmationCleanupEvidence = {
  apply?: boolean;
  reviewSha256?: string;
  policy?: { retainCount?: number; minAgeSeconds?: number };
  requestedBy?: { kind?: string; id?: string };
  pendingPreserved?: boolean;
  indeterminatePreservedCount?: number;
  terminalReceiptCount?: number;
  candidateCount?: number;
  candidates?: Array<{
    confirmationId?: string;
    state?: string;
    executionState?: string;
    requestedAt?: string;
    receiptSha256?: string;
  }>;
  removedCount?: number;
  tombstoneLedger?: {
    schema?: string;
    integrityState?: string;
    legacyPendingMigration?: boolean;
    segmentCapacity?: number;
    segmentCount?: number;
    activeCount?: number;
    activeSha256?: string;
    confirmationCount?: number;
    headSha256?: string | null;
  };
};

type TaskAuthorityWorkspaceProps = {
  collection: ServiceTaskAuthorityCollection | null;
  tabs: ServiceTaskAuthorityLiveTab[];
  sessionName: string;
  loading: boolean;
  error: string;
  onRefresh: () => void;
};

function valueString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function authorityStateLabel(authority: ServiceTaskAuthorityStatus): string {
  const reconciliation = authority.revocation?.reconciliation;
  if (reconciliation) return `reconciliation ${reconciliation.state ?? "recorded"}`;
  if (authority.envelope?.lineage) return "replacement authority";
  return authority.state ?? "unknown";
}

function AuthoritySummary({ authority }: { authority: ServiceTaskAuthorityStatus }) {
  const counts = taskAuthorityOutcomeCounts(authority);
  const lineage = authority.envelope?.lineage;
  const reconciliation = authority.revocation?.reconciliation;
  return (
    <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-2 xl:grid-cols-4">
      <span><strong className="text-foreground">Caller:</strong> {authority.envelope?.serviceName ?? "service not recorded"} / {authority.envelope?.agentName ?? "agent not recorded"}</span>
      <span><strong className="text-foreground">Progress:</strong> {counts.completed} completed, {counts.failed} failed, {counts.indeterminate} indeterminate</span>
      <span><strong className="text-foreground">Target:</strong> {authority.envelope?.targetBinding?.targetId ?? "missing"}</span>
      <span><strong className="text-foreground">Approval:</strong> {authority.approvalReference ?? "missing"}</span>
      {reconciliation && (
        <span className="sm:col-span-2 xl:col-span-4">
          <strong className="text-foreground">Reconciliation:</strong> {reconciliation.reconciliationId ?? "missing ID"} is {reconciliation.state ?? "recorded"}; replacement {reconciliation.replacementAuthorityId ?? "missing"}
        </span>
      )}
      {lineage && (
        <span className="sm:col-span-2 xl:col-span-4">
          <strong className="text-foreground">Lineage:</strong> predecessor {lineage.predecessorAuthorityId ?? "missing"}, step {lineage.predecessorStepId ?? "missing"}, command {lineage.predecessorCommandId ?? "missing"}
        </span>
      )}
    </div>
  );
}

export function TaskAuthorityWorkspace({
  collection,
  tabs,
  sessionName,
  loading,
  error,
  onRefresh,
}: TaskAuthorityWorkspaceProps) {
  const authorities = useMemo(
    () => [...(collection?.authorities ?? [])].sort((left, right) => (right.issuedAt ?? "").localeCompare(left.issuedAt ?? "")),
    [collection],
  );
  const confirmationReceipts = useMemo(
    () => [...(collection?.confirmationStatus?.confirmations ?? [])].sort((left, right) => (right.requestedAt ?? "").localeCompare(left.requestedAt ?? "")),
    [collection],
  );
  const [selected, setSelected] = useState<ServiceTaskAuthorityStatus | null>(null);
  const [recovery, setRecovery] = useState<{ authority: ServiceTaskAuthorityStatus; eligibility: TaskAuthorityRecoveryEligibility } | null>(null);
  const [planText, setPlanText] = useState("[]");
  const [approvalReference, setApprovalReference] = useState("");
  const [expiresInSeconds, setExpiresInSeconds] = useState(300);
  const [reconciliationId, setReconciliationId] = useState("");
  const [confirmation, setConfirmation] = useState<ConfirmationEvidence | null>(null);
  const [actionState, setActionState] = useState<"staging" | "confirming" | "denying" | null>(null);
  const [actionError, setActionError] = useState("");
  const [actionResult, setActionResult] = useState("");
  const [cleanupRetainCount, setCleanupRetainCount] = useState(1000);
  const [cleanupMinAgeSeconds, setCleanupMinAgeSeconds] = useState(30 * 24 * 60 * 60);
  const [cleanupPreview, setCleanupPreview] = useState<ConfirmationCleanupEvidence | null>(null);
  const [cleanupState, setCleanupState] = useState<"previewing" | "applying" | null>(null);
  const [cleanupError, setCleanupError] = useState("");
  const [cleanupResult, setCleanupResult] = useState("");

  const requestCleanup = async (apply: boolean) => {
    if (!Number.isInteger(cleanupRetainCount) || cleanupRetainCount < 0) {
      setCleanupError("Receipt retention count must be a nonnegative whole number.");
      return;
    }
    if (!Number.isInteger(cleanupMinAgeSeconds) || cleanupMinAgeSeconds < 0) {
      setCleanupError("Minimum receipt age must be a nonnegative whole number of seconds.");
      return;
    }
    const reviewSha256 = cleanupPreview?.reviewSha256;
    if (apply && (!reviewSha256 || !/^[0-9a-f]{64}$/i.test(reviewSha256))) {
      setCleanupError("Run and review a current cleanup preview before applying it.");
      return;
    }
    setCleanupState(apply ? "applying" : "previewing");
    setCleanupError("");
    setCleanupResult("");
    try {
      const response = await fetch(`${SERVICE_API_BASE}/task-authorities/confirmations/cleanup`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          sessionName,
          retainCount: cleanupRetainCount,
          minAgeSeconds: cleanupMinAgeSeconds,
          apply,
          ...(apply ? { reviewSha256 } : {}),
        }),
      });
      const payload = (await response.json().catch(() => ({}))) as ApiResponse;
      if (!response.ok || payload.success !== true) {
        throw new Error(payload.error || `Confirmation cleanup ${apply ? "apply" : "preview"} failed with HTTP ${response.status}`);
      }
      const data = (payload.data?.result ?? payload.data) as ConfirmationCleanupEvidence | undefined;
      if (!data || typeof data.reviewSha256 !== "string" || !/^[0-9a-f]{64}$/i.test(data.reviewSha256)) {
        throw new Error("The broker did not return a valid cleanup review digest.");
      }
      if (data.tombstoneLedger?.integrityState !== "verified") {
        throw new Error("The retired-ID ledger was not verified; cleanup failed closed.");
      }
      if (apply) {
        setCleanupResult(`Cleanup applied with the reviewed digest. Removed ${data.removedCount ?? 0} terminal receipts.`);
        setCleanupPreview(null);
        onRefresh();
      } else {
        setCleanupPreview(data);
      }
    } catch (caught) {
      setCleanupError(caught instanceof Error ? caught.message : "Confirmation cleanup failed.");
    } finally {
      setCleanupState(null);
    }
  };

  const invalidateCleanupPreview = () => {
    setCleanupPreview(null);
    setCleanupError("");
    setCleanupResult("");
  };

  const openRecovery = (authority: ServiceTaskAuthorityStatus) => {
    const eligibility = taskAuthorityRecoveryEligibility(authority, tabs);
    if (!eligibility.eligible) return;
    setRecovery({ authority, eligibility });
    setPlanText(JSON.stringify(eligibility.suggestedSteps, null, 2));
    setApprovalReference(`dashboard recovery for ${authority.id}`);
    setExpiresInSeconds(300);
    setReconciliationId(`dashboard-${authority.id}-${crypto.randomUUID()}`);
    setConfirmation(null);
    setActionError("");
    setActionResult("");
  };

  const decideConfirmation = async (decision: "confirm" | "deny") => {
    if (!confirmation) return false;
    setActionState(decision === "confirm" ? "confirming" : "denying");
    setActionError("");
    try {
      const response = await fetch(`${SERVICE_API_BASE}/task-authorities/confirmation`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sessionName, confirmationId: confirmation.confirmationId, expectedAction: "task_authority_reconcile", decision }),
      });
      const payload = (await response.json().catch(() => ({}))) as ApiResponse;
      if (!response.ok || payload.success !== true) {
        throw new Error(payload.error || `Confirmation ${decision} failed with HTTP ${response.status}`);
      }
      if (decision === "confirm") {
        const commandResult = payload.data?.result as Record<string, unknown> | undefined;
        if (commandResult?.success !== true) {
          throw new Error(valueString(commandResult?.error) || "The broker rejected the confirmed reconciliation.");
        }
        setActionResult("Reconciliation completed. The predecessor is revoked and the replacement authority is durable.");
        onRefresh();
      } else {
        setActionResult("Reconciliation was denied. No authority record changed.");
      }
      setConfirmation(null);
      return true;
    } catch (caught) {
      setActionError(caught instanceof Error ? caught.message : `Confirmation ${decision} failed.`);
      return false;
    } finally {
      setActionState(null);
    }
  };

  const closeRecovery = async () => {
    if (confirmation) {
      const denied = await decideConfirmation("deny");
      if (!denied) return;
    }
    setRecovery(null);
    setActionError("");
    setActionResult("");
  };

  const stageRecovery = async () => {
    const indeterminateStep = recovery?.eligibility.indeterminateStep;
    const target = recovery?.eligibility.target;
    if (!recovery || !indeterminateStep || !target) return;
    const parsed = parseTaskAuthorityReplacementPlan(planText);
    if (!parsed.success) {
      setActionError(parsed.error);
      return;
    }
    if (!approvalReference.trim()) {
      setActionError("Approval reference is required.");
      return;
    }
    if (!Number.isInteger(expiresInSeconds) || expiresInSeconds < 1 || expiresInSeconds > 3600) {
      setActionError("Replacement lifetime must be a whole number from 1 through 3600 seconds.");
      return;
    }
    setActionState("staging");
    setActionError("");
    setActionResult("");
    const { authority } = recovery;
    try {
      const response = await fetch(`${SERVICE_API_BASE}/task-authorities/${encodeURIComponent(authority.id)}/reconcile`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          sessionName,
          reconciliationId,
          unresolvedStepId: indeterminateStep.stepId,
          taskName: authority.envelope?.taskName,
          serviceName: authority.envelope?.serviceName,
          agentName: authority.envelope?.agentName,
          expectedTargetId: target.targetId,
          expectedUrl: target.url,
          approvalReference: approvalReference.trim(),
          expiresInSeconds,
          steps: parsed.steps,
        }),
      });
      const payload = (await response.json().catch(() => ({}))) as ApiResponse;
      if (!response.ok || payload.success !== true) {
        throw new Error(payload.error || `Reconciliation staging failed with HTTP ${response.status}`);
      }
      const data = payload.data ?? {};
      const confirmationId = valueString(data.confirmationId) || valueString(data.confirmation_id);
      const binding = data.expectedTargetBinding as Record<string, unknown> | undefined;
      const targetId = valueString(binding?.targetId);
      const url = valueString(binding?.url);
      if (data.confirmation_required !== true || !confirmationId || targetId !== target.targetId || url !== target.url) {
        if (confirmationId) {
          await fetch(`${SERVICE_API_BASE}/task-authorities/confirmation`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ sessionName, confirmationId, expectedAction: "task_authority_reconcile", decision: "deny" }),
          }).catch(() => null);
        }
        throw new Error("Broker confirmation evidence did not match the reviewed exact target and URL; reconciliation was not confirmed.");
      }
      setConfirmation({
        confirmationId,
        targetId,
        url,
        expiresInMs: typeof data.expiresInMs === "number" ? data.expiresInMs : null,
      });
    } catch (caught) {
      setActionError(caught instanceof Error ? caught.message : "Reconciliation staging failed.");
    } finally {
      setActionState(null);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p className="text-sm font-black text-foreground">Task authorities</p>
          <p className="text-xs text-muted-foreground">No-launch broker status for session <code>{sessionName || "default"}</code></p>
        </div>
        <Button size="sm" variant="outline" onClick={onRefresh} disabled={loading}>
          {loading ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
          Refresh
        </Button>
      </div>

      {error && (
        <div className="flex items-start gap-2 rounded-2xl border border-destructive/25 bg-destructive/10 px-3 py-3 text-xs text-destructive" role="alert">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {!error && loading && collection === null && (
        <div className="flex items-center justify-center gap-2 rounded-2xl border border-dashed border-border px-4 py-8 text-xs text-muted-foreground" role="status">
          <Loader2 className="size-4 animate-spin" />
          Loading task authority evidence
        </div>
      )}

      {confirmationReceipts.length > 0 && (
        <section className="rounded-2xl border border-border bg-card/70 p-3" aria-label="Task authority confirmation receipts">
          <p className="text-xs font-black text-foreground">Confirmation receipts</p>
          <div className="mt-2 space-y-2">
            {confirmationReceipts.map((receipt) => (
              <div key={receipt.confirmationId} className="rounded-xl bg-muted px-3 py-2 text-xs text-muted-foreground">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="outline">{receipt.state ?? "unknown"}</Badge>
                  <Badge variant={receipt.executionState === "indeterminate" ? "destructive" : "outline"}>{receipt.executionState ?? "unknown execution"}</Badge>
                  <code className="break-all text-foreground">{receipt.confirmationId}</code>
                </div>
                <p className="mt-1 break-all">{receipt.action ?? "unknown action"} on {receipt.targetBinding?.targetId ?? "missing target"} at {receipt.targetBinding?.url ?? "missing URL"}</p>
                <p className="mt-1 break-all">Request digest {receipt.requestSha256 ?? "missing"}; requested by {receipt.requestedBy?.kind ?? "unknown"}:{receipt.requestedBy?.id ?? "unknown"}; decided by {receipt.decidedBy ? `${receipt.decidedBy.kind ?? "unknown"}:${receipt.decidedBy.id ?? "unknown"}` : "not decided"}.</p>
                {receipt.invalidationReason && <p className="mt-1 text-destructive">{receipt.invalidationReason}</p>}
              </div>
            ))}
          </div>
        </section>
      )}

      <section className="rounded-2xl border border-border bg-card/70 p-3" aria-label="Confirmation receipt retention review">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="flex items-center gap-2 text-xs font-black text-foreground"><Database className="size-3.5" /> Receipt retention review</p>
            <p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">Preview is non-mutating. Apply is bound to the exact digest, candidate receipts, authenticated requester, policy, and verified retired-ID ledger state.</p>
          </div>
          <Button size="sm" variant="outline" onClick={() => void requestCleanup(false)} disabled={cleanupState !== null}>
            {cleanupState === "previewing" && <Loader2 className="size-3.5 animate-spin" />}
            Preview cleanup
          </Button>
        </div>
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          <label className="text-xs">
            <span className="font-black text-foreground">Keep newest terminal receipts</span>
            <input type="number" min={0} className="mt-1 w-full rounded-xl border border-input bg-background px-3 py-2" value={cleanupRetainCount} onChange={(event) => { invalidateCleanupPreview(); setCleanupRetainCount(Number(event.target.value)); }} disabled={cleanupState !== null} />
          </label>
          <label className="text-xs">
            <span className="font-black text-foreground">Minimum age in seconds</span>
            <input type="number" min={0} className="mt-1 w-full rounded-xl border border-input bg-background px-3 py-2" value={cleanupMinAgeSeconds} onChange={(event) => { invalidateCleanupPreview(); setCleanupMinAgeSeconds(Number(event.target.value)); }} disabled={cleanupState !== null} />
          </label>
        </div>
        {cleanupPreview && (
          <div className="mt-3 space-y-3 rounded-2xl border border-border bg-muted/50 p-3 text-xs">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="outline">{cleanupPreview.candidateCount ?? 0} candidates</Badge>
              <Badge variant="outline">{cleanupPreview.indeterminatePreservedCount ?? 0} indeterminate preserved</Badge>
              <Badge variant={cleanupPreview.tombstoneLedger?.integrityState === "verified" ? "outline" : "destructive"}>ledger {cleanupPreview.tombstoneLedger?.integrityState ?? "unknown"}</Badge>
              {cleanupPreview.tombstoneLedger?.legacyPendingMigration && <Badge variant="outline">legacy migration pending</Badge>}
            </div>
            <dl className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
              <div><dt className="text-muted-foreground">Requested by</dt><dd>{cleanupPreview.requestedBy?.kind ?? "unknown"}:{cleanupPreview.requestedBy?.id ?? "unknown"}</dd></div>
              <div><dt className="text-muted-foreground">Ledger entries</dt><dd>{cleanupPreview.tombstoneLedger?.confirmationCount ?? 0}</dd></div>
              <div><dt className="text-muted-foreground">Segments</dt><dd>{cleanupPreview.tombstoneLedger?.segmentCount ?? 0} at capacity {cleanupPreview.tombstoneLedger?.segmentCapacity ?? "unknown"}</dd></div>
              <div><dt className="text-muted-foreground">Active IDs</dt><dd>{cleanupPreview.tombstoneLedger?.activeCount ?? 0}</dd></div>
            </dl>
            <p className="break-all text-muted-foreground">Ledger head {cleanupPreview.tombstoneLedger?.headSha256 ?? "no immutable segments"}; active digest {cleanupPreview.tombstoneLedger?.activeSha256 ?? "missing"}.</p>
            <div>
              <p className="font-black text-foreground">Exact review digest</p>
              <code className="mt-1 block break-all rounded-xl bg-background px-3 py-2">{cleanupPreview.reviewSha256}</code>
            </div>
            {(cleanupPreview.candidates?.length ?? 0) > 0 && (
              <div>
                <p className="font-black text-foreground">Candidate evidence</p>
                <div className="mt-1 max-h-44 space-y-1 overflow-y-auto">
                  {cleanupPreview.candidates?.map((candidate) => (
                    <div key={candidate.confirmationId} className="rounded-xl bg-background px-3 py-2">
                      <code className="break-all">{candidate.confirmationId}</code>
                      <p className="mt-1 break-all text-muted-foreground">{candidate.state} / {candidate.executionState}; receipt {candidate.receiptSha256}</p>
                    </div>
                  ))}
                </div>
              </div>
            )}
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button size="sm" variant="destructive" disabled={cleanupState !== null || ((cleanupPreview.candidateCount ?? 0) === 0 && !cleanupPreview.tombstoneLedger?.legacyPendingMigration)}>
                  Apply reviewed cleanup
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Apply this exact receipt cleanup?</AlertDialogTitle>
                  <AlertDialogDescription>
                    The broker will recompute the candidate set and verified ledger state. Any change invalidates this digest. Pending and indeterminate receipts remain preserved.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Keep reviewing</AlertDialogCancel>
                  <AlertDialogAction onClick={() => void requestCleanup(true)}>Apply exact digest</AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        )}
        {cleanupError && <p className="mt-3 rounded-xl bg-destructive/10 px-3 py-2 text-xs text-destructive" role="alert">{cleanupError}</p>}
        {cleanupResult && <p className="mt-3 flex items-center gap-2 rounded-xl bg-emerald-500/10 px-3 py-2 text-xs text-foreground" role="status"><CheckCircle2 className="size-4" />{cleanupResult}</p>}
      </section>

      {!error && !loading && authorities.length === 0 && (
        <div className="rounded-2xl border border-dashed border-border px-4 py-8 text-center">
          <ShieldCheck className="mx-auto size-6 text-muted-foreground" />
          <p className="mt-2 text-sm font-black text-foreground">No authority records</p>
          <p className="mt-1 text-xs text-muted-foreground">This selected daemon session has not issued durable task authority.</p>
        </div>
      )}

      <div className="space-y-2">
        {authorities.map((authority) => {
          const eligibility = taskAuthorityRecoveryEligibility(authority, tabs);
          const counts = taskAuthorityOutcomeCounts(authority);
          return (
            <article key={authority.id} className="rounded-2xl border border-border bg-card/70 p-3">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">{authorityStateLabel(authority)}</Badge>
                    {counts.indeterminate > 0 && <Badge variant="destructive">{counts.indeterminate} indeterminate</Badge>}
                    <p className="break-all text-xs font-black text-foreground">{authority.id}</p>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">{authority.envelope?.taskName ?? "task not recorded"}</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button size="sm" variant="outline" onClick={() => setSelected(authority)}>Inspect</Button>
                  <Button size="sm" onClick={() => openRecovery(authority)} disabled={!eligibility.eligible} title={eligibility.reason}>
                    <FileWarning className="size-3.5" />
                    Prepare recovery
                  </Button>
                </div>
              </div>
              <div className="mt-3"><AuthoritySummary authority={authority} /></div>
              {counts.indeterminate > 0 && (
                <p className="mt-3 rounded-xl bg-amber-500/10 px-3 py-2 text-xs leading-5 text-foreground">
                  <strong>Recovery gate:</strong> {eligibility.reason}
                </p>
              )}
            </article>
          );
        })}
      </div>

      <Dialog open={selected !== null} onOpenChange={(open) => { if (!open) setSelected(null); }}>
        <DialogContent className="max-h-[85vh] max-w-3xl overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Task authority evidence</DialogTitle>
            <DialogDescription>Immutable broker status, outcomes, plan, revocation, and lineage.</DialogDescription>
          </DialogHeader>
          {selected && (
            <div className="space-y-4 text-xs">
              <AuthoritySummary authority={selected} />
              <section>
                <p className="font-black text-foreground">Approved plan</p>
                <pre className="mt-1 max-h-56 overflow-auto rounded-xl bg-muted p-3 text-[11px] leading-5">{JSON.stringify(selected.approvedPlan, null, 2)}</pre>
              </section>
              <section>
                <p className="font-black text-foreground">Durable outcomes</p>
                <pre className="mt-1 max-h-56 overflow-auto rounded-xl bg-muted p-3 text-[11px] leading-5">{JSON.stringify(selected.usage, null, 2)}</pre>
              </section>
              {(selected.revocation || selected.envelope?.lineage) && (
                <section>
                  <p className="font-black text-foreground">Reconciliation lineage</p>
                  <pre className="mt-1 max-h-56 overflow-auto rounded-xl bg-muted p-3 text-[11px] leading-5">{JSON.stringify({ revocation: selected.revocation, lineage: selected.envelope?.lineage }, null, 2)}</pre>
                </section>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>

      <Dialog open={recovery !== null} onOpenChange={(open) => { if (!open) void closeRecovery(); }}>
        <DialogContent className="max-h-[90vh] max-w-3xl overflow-y-auto" onEscapeKeyDown={(event) => { if (confirmation || actionState) event.preventDefault(); }}>
          <DialogHeader>
            <DialogTitle>Review replacement authority</DialogTitle>
            <DialogDescription>The consumed indeterminate step is evidence only. It is never copied into this replacement plan.</DialogDescription>
          </DialogHeader>
          {recovery && (
            <div className="space-y-4 text-xs">
              <div className="rounded-2xl border border-border p-3">
                <p className="font-black text-foreground">Predecessor receipt</p>
                <dl className="mt-2 grid gap-2 sm:grid-cols-2">
                  <div><dt className="text-muted-foreground">Authority</dt><dd className="break-all font-mono">{recovery.authority.id}</dd></div>
                  <div><dt className="text-muted-foreground">Step</dt><dd className="break-all font-mono">{recovery.eligibility.indeterminateStep?.stepId}</dd></div>
                  <div><dt className="text-muted-foreground">Command</dt><dd className="break-all font-mono">{recovery.eligibility.indeterminateStep?.commandId}</dd></div>
                  <div><dt className="text-muted-foreground">Outcome</dt><dd>indeterminate and consumed</dd></div>
                  <div className="sm:col-span-2"><dt className="text-muted-foreground">Exact target</dt><dd className="break-all font-mono">{recovery.eligibility.target?.targetId} at {recovery.eligibility.target?.url}</dd></div>
                </dl>
              </div>

              <label className="block">
                <span className="font-black text-foreground">Replacement plan JSON</span>
                <span className="mt-1 block leading-5 text-muted-foreground">Only action, URL, and evidenceBytes are accepted. Consumed step IDs and command IDs are rejected.</span>
                <textarea className="mt-2 min-h-44 w-full rounded-xl border border-input bg-background p-3 font-mono text-[11px] leading-5" value={planText} onChange={(event) => setPlanText(event.target.value)} disabled={confirmation !== null || actionState !== null} />
              </label>
              <div className="grid gap-3 sm:grid-cols-2">
                <label>
                  <span className="font-black text-foreground">Approval reference</span>
                  <input className="mt-1 w-full rounded-xl border border-input bg-background px-3 py-2" value={approvalReference} onChange={(event) => setApprovalReference(event.target.value)} disabled={confirmation !== null || actionState !== null} />
                </label>
                <label>
                  <span className="font-black text-foreground">Replacement lifetime in seconds</span>
                  <input type="number" min={1} max={3600} className="mt-1 w-full rounded-xl border border-input bg-background px-3 py-2" value={expiresInSeconds} onChange={(event) => setExpiresInSeconds(Number(event.target.value))} disabled={confirmation !== null || actionState !== null} />
                </label>
              </div>
              <div className="rounded-xl bg-muted px-3 py-2">
                <span className="font-black text-foreground">Stable reconciliation ID:</span> <code className="break-all">{reconciliationId}</code>
              </div>

              {confirmation && (
                <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 p-3" role="status">
                  <div className="flex items-center gap-2 font-black text-foreground"><GitBranch className="size-4" /> Exact-target confirmation pending</div>
                  <p className="mt-2 break-all">Confirmation <code>{confirmation.confirmationId}</code></p>
                  <p className="mt-1 break-all">Target <code>{confirmation.targetId}</code></p>
                  <p className="mt-1 break-all">URL <code>{confirmation.url}</code></p>
                  <p className="mt-1">Expires in {confirmation.expiresInMs === null ? "the broker's bounded window" : `${Math.round(confirmation.expiresInMs / 1000)} seconds`}.</p>
                </div>
              )}
              {actionError && <p className="rounded-xl bg-destructive/10 px-3 py-2 text-destructive" role="alert">{actionError}</p>}
              {actionResult && <p className="flex items-center gap-2 rounded-xl bg-emerald-500/10 px-3 py-2 text-foreground" role="status"><CheckCircle2 className="size-4" />{actionResult}</p>}
            </div>
          )}
          <DialogFooter className="gap-2 sm:gap-2">
            <Button variant="outline" onClick={() => void closeRecovery()} disabled={actionState !== null}>
              {confirmation ? "Deny and close" : "Close"}
            </Button>
            {!confirmation && !actionResult && (
              <Button onClick={() => void stageRecovery()} disabled={actionState !== null}>
                {actionState === "staging" && <Loader2 className="size-3.5 animate-spin" />}
                Request exact-target confirmation
              </Button>
            )}
            {confirmation && (
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button disabled={actionState !== null}>Confirm reconciliation</Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Confirm this replacement authority?</AlertDialogTitle>
                    <AlertDialogDescription>
                      This durably revokes the predecessor and creates one lineage-bound replacement for the exact target and URL shown in the preview. The indeterminate step remains consumed.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>Review again</AlertDialogCancel>
                    <AlertDialogAction onClick={() => void decideConfirmation("confirm")}>Confirm exact target</AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
