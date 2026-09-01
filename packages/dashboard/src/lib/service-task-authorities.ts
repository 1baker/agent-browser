export type ServiceTaskAuthorityPlanStep = {
  stepId?: string;
  index?: number;
  action?: string;
  requestedUrl?: string | null;
  url?: string | null;
  evidenceBytes?: number;
};

export type ServiceTaskAuthorityOutcomeStep = {
  stepId?: string;
  stepIndex?: number;
  commandId?: string;
  action?: string;
  currentUrl?: string;
  requestedUrl?: string | null;
  reservedEvidenceBytes?: number;
  outcome?: {
    state?: string;
    success?: boolean | null;
    reason?: string | null;
  };
};

export type ServiceTaskAuthorityStatus = {
  id: string;
  sessionId?: string;
  state?: string;
  usable?: boolean;
  issuedAt?: string;
  approvalReference?: string;
  issuer?: { kind?: string; id?: string };
  envelope?: {
    taskName?: string;
    serviceName?: string | null;
    agentName?: string | null;
    targetBinding?: { targetId?: string; initialUrl?: string };
    lineage?: ServiceTaskAuthorityLineage | null;
    expiresAt?: string;
  };
  approvedPlan?: {
    steps?: ServiceTaskAuthorityPlanStep[];
    planSha256?: string;
  };
  revocation?: {
    revokedAt?: string;
    revokedBy?: string;
    reason?: string;
    reconciliation?: ServiceTaskAuthorityReconciliation | null;
  } | null;
  usage?: {
    nextStepIndex?: number;
    remainingSteps?: number;
    completedSteps?: ServiceTaskAuthorityOutcomeStep[];
    failedSteps?: ServiceTaskAuthorityOutcomeStep[];
    indeterminateSteps?: ServiceTaskAuthorityOutcomeStep[];
    outcomeSummary?: { completed?: number; failed?: number; indeterminate?: number };
  };
};

export type ServiceTaskAuthorityLineage = {
  reconciliationId?: string;
  predecessorAuthorityId?: string;
  predecessorStepId?: string;
  predecessorStepIndex?: number;
  predecessorCommandId?: string;
  predecessorOutcomeState?: string;
};

export type ServiceTaskAuthorityReconciliation = ServiceTaskAuthorityLineage & {
  requestSha256?: string;
  replacementAuthorityId?: string;
  state?: string;
  completedAt?: string | null;
};

export type ServiceTaskAuthorityCollection = {
  sessionId?: string;
  count?: number;
  authorities?: ServiceTaskAuthorityStatus[];
  confirmationStatus?: ServiceTaskAuthorityConfirmationCollection;
};

export type ServiceTaskAuthorityConfirmationStatus = {
  confirmationId?: string;
  sessionId?: string;
  action?: string;
  consequenceClass?: string;
  targetBinding?: { targetId?: string; url?: string };
  requestSha256?: string;
  requestedBy?: { kind?: string; id?: string };
  requestedAt?: string;
  expiresAt?: string;
  state?: string;
  executionState?: string;
  decidedBy?: { kind?: string; id?: string } | null;
  decidedAt?: string | null;
  decision?: string | null;
  invalidationReason?: string | null;
};

export type ServiceTaskAuthorityConfirmationCollection = {
  sessionId?: string;
  count?: number;
  confirmations?: ServiceTaskAuthorityConfirmationStatus[];
};

export type ServiceTaskAuthorityLiveTab = {
  id?: string;
  targetId?: string | null;
  lifecycle?: string;
  url?: string | null;
};

export type TaskAuthorityReplacementStep = {
  action: string;
  url?: string;
  evidenceBytes?: number;
};

export type TaskAuthorityRecoveryEligibility = {
  eligible: boolean;
  reason: string;
  indeterminateStep: ServiceTaskAuthorityOutcomeStep | null;
  target: { targetId: string; url: string; tabId: string | null } | null;
  suggestedSteps: TaskAuthorityReplacementStep[];
};

export function taskAuthorityOutcomeCounts(authority: ServiceTaskAuthorityStatus): {
  completed: number;
  failed: number;
  indeterminate: number;
} {
  const summary = authority.usage?.outcomeSummary;
  return {
    completed: summary?.completed ?? authority.usage?.completedSteps?.length ?? 0,
    failed: summary?.failed ?? authority.usage?.failedSteps?.length ?? 0,
    indeterminate: summary?.indeterminate ?? authority.usage?.indeterminateSteps?.length ?? 0,
  };
}

export function taskAuthoritySuggestedReplacementSteps(
  authority: ServiceTaskAuthorityStatus,
): TaskAuthorityReplacementStep[] {
  const steps = authority.approvedPlan?.steps ?? [];
  const nextStepIndex = authority.usage?.nextStepIndex ?? 0;
  return steps.slice(nextStepIndex).flatMap((step) => {
    const action = step.action?.trim();
    if (!action) return [];
    const url = step.requestedUrl ?? step.url ?? undefined;
    return [{
      action,
      ...(url ? { url } : {}),
      ...(Number.isInteger(step.evidenceBytes) && (step.evidenceBytes ?? 0) > 0
        ? { evidenceBytes: step.evidenceBytes }
        : {}),
    }];
  });
}

export function taskAuthorityRecoveryEligibility(
  authority: ServiceTaskAuthorityStatus,
  tabs: ServiceTaskAuthorityLiveTab[],
): TaskAuthorityRecoveryEligibility {
  const indeterminate = authority.usage?.indeterminateSteps ?? [];
  const suggestedSteps = taskAuthoritySuggestedReplacementSteps(authority);
  const targetId = authority.envelope?.targetBinding?.targetId?.trim() ?? "";
  const matches = tabs.filter((tab) =>
    tab.targetId === targetId
    && tab.lifecycle !== "closed"
    && typeof tab.url === "string"
    && tab.url.length > 0,
  );
  const target = matches.length === 1
    ? { targetId, url: matches[0].url as string, tabId: matches[0].id ?? null }
    : null;

  if (authority.revocation?.reconciliation) {
    return {
      eligible: false,
      reason: `Reconciliation is already ${authority.revocation.reconciliation.state ?? "recorded"}; inspect its durable lineage.`,
      indeterminateStep: indeterminate[0] ?? null,
      target,
      suggestedSteps,
    };
  }
  if (authority.state === "revoked") {
    return { eligible: false, reason: "The predecessor authority is already revoked.", indeterminateStep: indeterminate[0] ?? null, target, suggestedSteps };
  }
  if (indeterminate.length !== 1) {
    return {
      eligible: false,
      reason: `Recovery requires exactly one indeterminate receipt; this authority has ${indeterminate.length}.`,
      indeterminateStep: indeterminate[0] ?? null,
      target,
      suggestedSteps,
    };
  }
  if (!targetId) {
    return { eligible: false, reason: "The authority has no exact target binding.", indeterminateStep: indeterminate[0], target: null, suggestedSteps };
  }
  if (matches.length !== 1) {
    return {
      eligible: false,
      reason: `Recovery requires one live exact-target tab; found ${matches.length} for ${targetId}.`,
      indeterminateStep: indeterminate[0],
      target: null,
      suggestedSteps,
    };
  }
  if (suggestedSteps.length === 0) {
    return {
      eligible: true,
      reason: "The consumed step is excluded. Add at least one explicit replacement step before staging.",
      indeterminateStep: indeterminate[0],
      target,
      suggestedSteps,
    };
  }
  return {
    eligible: true,
    reason: "One indeterminate receipt and one exact live target are ready for an explicit replacement preview.",
    indeterminateStep: indeterminate[0],
    target,
    suggestedSteps,
  };
}

export function parseTaskAuthorityReplacementPlan(text: string):
  | { success: true; steps: TaskAuthorityReplacementStep[] }
  | { success: false; error: string } {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (error) {
    return { success: false, error: `Replacement plan is not valid JSON: ${error instanceof Error ? error.message : String(error)}` };
  }
  if (!Array.isArray(value) || value.length === 0 || value.length > 100) {
    return { success: false, error: "Replacement plan must contain 1 through 100 steps." };
  }
  const steps: TaskAuthorityReplacementStep[] = [];
  for (const [index, raw] of value.entries()) {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      return { success: false, error: `Replacement step ${index + 1} must be an object.` };
    }
    const record = raw as Record<string, unknown>;
    const unsupported = Object.keys(record).filter((key) => !["action", "url", "evidenceBytes"].includes(key));
    if (unsupported.length > 0) {
      return { success: false, error: `Replacement step ${index + 1} contains unsupported fields: ${unsupported.join(", ")}.` };
    }
    const action = typeof record.action === "string" ? record.action.trim() : "";
    if (!action) return { success: false, error: `Replacement step ${index + 1} requires an action.` };
    const step: TaskAuthorityReplacementStep = { action };
    if (record.url !== undefined) {
      if (typeof record.url !== "string") return { success: false, error: `Replacement step ${index + 1} URL must be a string.` };
      try {
        const url = new URL(record.url);
        if (!["http:", "https:"].includes(url.protocol)) throw new Error("unsupported protocol");
        step.url = url.toString();
      } catch {
        return { success: false, error: `Replacement step ${index + 1} URL must be an absolute HTTP or HTTPS URL.` };
      }
    }
    if (record.evidenceBytes !== undefined) {
      if (!Number.isInteger(record.evidenceBytes) || (record.evidenceBytes as number) < 1) {
        return { success: false, error: `Replacement step ${index + 1} evidenceBytes must be a positive integer.` };
      }
      step.evidenceBytes = record.evidenceBytes as number;
    }
    if (action === "navigate" && !step.url) {
      return { success: false, error: `Replacement step ${index + 1} navigation requires a URL.` };
    }
    steps.push(step);
  }
  return { success: true, steps };
}
