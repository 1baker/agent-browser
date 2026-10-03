export interface ServiceRunView {
  id: string;
  kind: "recent_job" | "human_challenge" | "manual_sign_in";
  state: string;
  nextAction: string;
  taskOutcomeVerified: false;
  evidenceScope: string;
  taskName?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  jobId?: string | null;
  action?: string | null;
  challengeId?: string | null;
  handoffId?: string | null;
  browserId?: string | null;
  tabId?: string | null;
  targetId?: string | null;
  serviceTabHandle?: Record<string, unknown> | null;
  gatePolicyDecision?: string | null;
  sessionName?: string | null;
  profileId?: string | null;
  targetServiceId?: string | null;
  url?: string | null;
  updatedAt?: string | null;
}

export interface ServiceRunsResponse {
  runs: ServiceRunView[];
  count: number;
  coverage: "recent_jobs_and_retained_human_gates";
  durableTaskHistory: false;
}

export declare function getServiceRuns(options: {
  baseUrl: string;
  fetchImpl?: typeof fetch;
}): Promise<ServiceRunsResponse>;
