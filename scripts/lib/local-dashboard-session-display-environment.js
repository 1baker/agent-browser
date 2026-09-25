import { readFileSync } from 'node:fs';

const DISPLAY_PATTERN = /^:[0-9]+(?:\.[0-9]+)?$/;
const MAX_ENVIRONMENT_BYTES = 256 * 1024;

/** Capture only the display authority needed to resume one exact daemon lane. */
export function readSessionDisplayEnvironment(pid, readEnvironment = readFileSync) {
  if (!Number.isInteger(pid) || pid <= 0) {
    throw new Error('Runtime display environment requires a live daemon PID');
  }
  if (process.platform !== 'linux') return { display: null, osClickIsolatedDisplay: null };
  const bytes = readEnvironment(`/proc/${pid}/environ`);
  if (bytes.length > MAX_ENVIRONMENT_BYTES) {
    throw new Error('Runtime display environment exceeds the bounded read limit');
  }
  const entries = bytes.toString('utf8').split('\0');
  const values = new Map();
  for (const entry of entries) {
    const split = entry.indexOf('=');
    if (split < 0) continue;
    const key = entry.slice(0, split);
    if (key !== 'DISPLAY' && key !== 'AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY') continue;
    if (values.has(key)) throw new Error(`Runtime display environment repeats ${key}`);
    values.set(key, entry.slice(split + 1));
  }
  const display = values.get('DISPLAY') || null;
  const osClickIsolatedDisplay = values.get('AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY') || null;
  if (display && !DISPLAY_PATTERN.test(display)) {
    throw new Error('Runtime display environment has an unsupported DISPLAY');
  }
  if (osClickIsolatedDisplay && osClickIsolatedDisplay !== display) {
    throw new Error('Runtime OS-click opt-in does not match DISPLAY');
  }
  return { display, osClickIsolatedDisplay };
}

/** Never inherit a publisher shell's display authority into another session. */
export function withSessionDisplayEnvironment(baseEnvironment, captured) {
  if (!captured || typeof captured !== 'object') {
    throw new Error('Runtime display environment evidence is missing');
  }
  const { display, osClickIsolatedDisplay } = captured;
  if (display !== null && (typeof display !== 'string' || !DISPLAY_PATTERN.test(display))) {
    throw new Error('Runtime display environment has an unsupported DISPLAY');
  }
  if (osClickIsolatedDisplay !== null && osClickIsolatedDisplay !== display) {
    throw new Error('Runtime OS-click opt-in does not match DISPLAY');
  }
  const next = { ...baseEnvironment };
  delete next.DISPLAY;
  delete next.AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY;
  if (display) next.DISPLAY = display;
  if (osClickIsolatedDisplay) next.AGENT_BROWSER_OS_CLICK_ISOLATED_DISPLAY = osClickIsolatedDisplay;
  return next;
}
