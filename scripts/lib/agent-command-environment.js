// Side-effect-free helpers for agent-browser subprocess environments.

/**
 * Authorization for the publisher's own pre-install `handoff prepare` (see
 * cli/src/native/handoff_guard.rs). It is passed to that one subprocess only.
 */
export const PUBLISHER_HANDOFF_ENVIRONMENT = Object.freeze({
  AGENT_BROWSER_ALLOW_EXECUTABLE_SIDEGRADE: '1',
});

/**
 * Environment for one subprocess: the base environment unchanged, or a new
 * object with `extraEnvironment` merged in. Neither input is mutated.
 */
export function agentCommandEnvironment(baseEnvironment, extraEnvironment = undefined) {
  return extraEnvironment === undefined ? baseEnvironment : { ...baseEnvironment, ...extraEnvironment };
}
