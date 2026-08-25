#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const missions = [
  {
    id: 'iana-example-domain',
    source: 'IANA example domain',
    url: 'https://example.com/',
    finalUrlPrefixes: ['https://example.com/'],
    titleTerms: ['example domain'],
    evidenceTerms: ['example domain', 'documentation examples'],
  },
  {
    id: 'rfc-http-semantics',
    source: 'RFC Editor HTTP Semantics',
    url: 'https://www.rfc-editor.org/rfc/rfc9110.html',
    finalUrlPrefixes: ['https://www.rfc-editor.org/rfc/rfc9110'],
    titleTerms: ['rfc 9110'],
    evidenceTerms: ['http semantics', 'fielding'],
  },
  {
    id: 'rfc-uri-syntax',
    source: 'RFC Editor URI syntax',
    url: 'https://www.rfc-editor.org/rfc/rfc3986.html',
    finalUrlPrefixes: ['https://www.rfc-editor.org/rfc/rfc3986'],
    titleTerms: ['rfc 3986'],
    evidenceTerms: ['uniform resource identifier', 'generic syntax'],
  },
  {
    id: 'rfc-json-format',
    source: 'RFC Editor JSON format',
    url: 'https://www.rfc-editor.org/rfc/rfc8259.html',
    finalUrlPrefixes: ['https://www.rfc-editor.org/rfc/rfc8259'],
    titleTerms: ['rfc 8259'],
    evidenceTerms: ['javascript object notation', 'data interchange format'],
  },
  {
    id: 'w3c-accessibility-guidelines',
    source: 'W3C WCAG 2.2',
    url: 'https://www.w3.org/TR/WCAG22/',
    finalUrlPrefixes: ['https://www.w3.org/TR/WCAG22/'],
    titleTerms: ['web content accessibility guidelines'],
    evidenceTerms: ['wcag 2.2', 'w3c recommendation'],
  },
  {
    id: 'whatwg-html-standard',
    source: 'WHATWG HTML Standard',
    url: 'https://html.spec.whatwg.org/',
    finalUrlPrefixes: ['https://html.spec.whatwg.org/'],
    titleTerms: ['html standard'],
    evidenceTerms: ['living standard', 'whatwg'],
    evidenceReader: 'network-resource',
  },
  {
    id: 'python-url-parsing',
    source: 'Python urllib.parse documentation',
    url: 'https://docs.python.org/3/library/urllib.parse.html',
    finalUrlPrefixes: ['https://docs.python.org/3/library/urllib.parse.html'],
    titleTerms: ['urllib.parse'],
    evidenceTerms: ['parse urls', 'url parsing'],
  },
  {
    id: 'rust-language-book',
    source: 'The Rust Programming Language',
    url: 'https://doc.rust-lang.org/book/',
    finalUrlPrefixes: ['https://doc.rust-lang.org/book/'],
    titleTerms: ['rust programming language'],
    evidenceTerms: ['rust programming language', 'foreword'],
  },
  {
    id: 'git-reference',
    source: 'Git reference manual',
    url: 'https://git-scm.com/docs/git',
    finalUrlPrefixes: ['https://git-scm.com/docs/git'],
    titleTerms: ['git'],
    evidenceTerms: ['distributed revision control system', 'synopsis'],
  },
  {
    id: 'node-url-api',
    source: 'Node.js URL API',
    url: 'https://nodejs.org/api/url.html',
    finalUrlPrefixes: ['https://nodejs.org/api/url.html'],
    titleTerms: ['url'],
    evidenceTerms: ['node.js', 'whatwg url api'],
  },
];
const selectedMissionIds = new Set(
  String(process.env.AGENT_BROWSER_RESEARCH_MISSION_IDS || '')
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean),
);
const selectedMissions = selectedMissionIds.size
  ? missions.filter((mission) => selectedMissionIds.has(mission.id))
  : missions;
assert(selectedMissions.length > 0, 'AGENT_BROWSER_RESEARCH_MISSION_IDS selected no known missions');
assert(
  selectedMissions.length === selectedMissionIds.size || selectedMissionIds.size === 0,
  'AGENT_BROWSER_RESEARCH_MISSION_IDS contains an unknown mission id',
);

const context = createSmokeContext({
  prefix: 'ab-agentic-research-concierge-',
  sessionPrefix: 'agentic-research-concierge',
});
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD =
  process.env.AGENT_BROWSER_RESEARCH_AGENT_BROWSER_CMD || 'agent-browser';
context.env.AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS = '0';

const missionTimeoutMs = positiveInteger(
  process.env.AGENT_BROWSER_RESEARCH_MISSION_TIMEOUT_MS,
  60000,
  'AGENT_BROWSER_RESEARCH_MISSION_TIMEOUT_MS',
);
const suiteTimeoutMs = positiveInteger(
  process.env.AGENT_BROWSER_RESEARCH_SUITE_TIMEOUT_MS,
  720000,
  'AGENT_BROWSER_RESEARCH_SUITE_TIMEOUT_MS',
);
const suiteStartedAt = Date.now();
const timeout = setTimeout(() => {
  console.error(`Research concierge suite timed out after ${suiteTimeoutMs}ms`);
  process.exitCode = 1;
}, suiteTimeoutMs);

let profilePath;
let cleanupComplete = false;
const results = [];

try {
  const browserExecutable = resolveBrowserExecutable();
  profilePath = createDisposableProfile(browserExecutable);
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = browserExecutable;
  context.env.AGENT_BROWSER_PROFILE = profilePath;

  for (const mission of selectedMissions) {
    const result = await runMission(mission);
    results.push(result);
    console.error(
      `${result.passed ? 'PASS' : 'FAIL'} ${mission.id} ${result.durationMs}ms ${result.finalUrl || result.error || ''}`,
    );
  }

  await cleanup();
  const report = buildReport({ browserExecutable });
  const reportPath = process.env.AGENT_BROWSER_RESEARCH_REPORT;
  if (reportPath) writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.success) process.exitCode = 1;
} catch (err) {
  await cleanup();
  const report = buildReport({ fatalError: err.stack || err.message });
  const reportPath = process.env.AGENT_BROWSER_RESEARCH_REPORT;
  if (reportPath) writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  process.exitCode = 1;
} finally {
  clearTimeout(timeout);
}

async function runMission(mission) {
  const startedAt = Date.now();
  const deadlineAt = startedAt + missionTimeoutMs;
  try {
    let openTimedOut = false;
    try {
      const opened = parseJsonOutput(
        (await runCli(context, [
          '--json',
          '--session', context.session,
          '--profile', profilePath,
          'open', mission.url,
          '--headed',
          '--timeout', String(Math.min(missionTimeoutMs, 20000)),
        ], remainingTime(deadlineAt, 30000))).stdout,
        `${mission.id} open`,
      );
      assert(opened.success === true, `${mission.id} open failed: ${JSON.stringify(opened)}`);
    } catch (err) {
      if (!/operation timed out|timed out/i.test(err.message)) throw err;
      openTimedOut = true;
    }

    let page;
    let evidenceSource = 'renderer-evaluate';
    let evidenceError = null;
    try {
      if (mission.evidenceReader === 'network-resource') {
        page = await readPageResource(mission, deadlineAt);
        evidenceSource = 'network-resource';
      } else {
        const evidence = parseJsonOutput(
          (await runCli(context, [
            '--json',
            '--session', context.session,
            '--job-timeout-ms', String(Math.min(15000, remainingTime(deadlineAt, 15000))),
            'eval',
            `(() => {
          const chunks = [];
          let length = 0;
          const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
          while (length < 50000) {
            const node = walker.nextNode();
            if (!node) break;
            const value = node.nodeValue || '';
            chunks.push(value);
            length += value.length;
          }
          return {
            url: location.href,
            title: document.title,
            heading: document.querySelector('h1')?.textContent || '',
            text: chunks.join(' ').replace(/\\s+/g, ' ').slice(0, 50000),
            linkCount: document.links.length,
          };
          })()`,
          ], remainingTime(deadlineAt, 20000))).stdout,
          `${mission.id} evidence`,
        );
        assert(evidence.success === true, `${mission.id} evidence failed: ${JSON.stringify(evidence)}`);
        page = evidence.data?.result;
      }
    } catch (err) {
      evidenceError = err.message;
      if (mission.evidenceReader === 'network-resource') throw err;
      page = await readPageResource(mission, deadlineAt);
      evidenceSource = 'network-resource-after-renderer-failure';
    }
    assert(page && typeof page === 'object', `${mission.id} returned no evidence object`);

    const normalizedIdentity = normalize(`${page.title} ${page.heading} ${page.text}`);
    const normalizedText = normalize(`${page.heading} ${page.text}`);
    const urlMatched = mission.finalUrlPrefixes.some((prefix) => page.url?.startsWith(prefix));
    const identityMatched = mission.titleTerms.every((term) => normalizedIdentity.includes(normalize(term)));
    const evidenceMatched = mission.evidenceTerms.every((term) => normalizedText.includes(normalize(term)));
    const missingIdentityTerms = mission.titleTerms.filter((term) => !normalizedIdentity.includes(normalize(term)));
    const missingEvidenceTerms = mission.evidenceTerms.filter((term) => !normalizedText.includes(normalize(term)));
    const checks = {
      canonicalUrl: urlMatched,
      pageIdentity: identityMatched,
      evidence: evidenceMatched,
      bounded: Date.now() - startedAt <= missionTimeoutMs,
    };

    return {
      id: mission.id,
      source: mission.source,
      requestedUrl: mission.url,
      finalUrl: page.url,
      title: page.title,
      heading: page.heading,
      linkCount: page.linkCount,
      titleAvailable: Boolean(page.title),
      evidenceSource,
      activeTargetUrl: page.activeTargetUrl || null,
      openTimedOut,
      recoveredAfterOpenTimeout: openTimedOut && urlMatched && identityMatched && evidenceMatched,
      evidenceTerms: mission.evidenceTerms,
      missingIdentityTerms,
      missingEvidenceTerms,
      textSample: String(page.text || '').replace(/\s+/g, ' ').trim().slice(0, 500),
      checks,
      passed: Object.values(checks).every(Boolean),
      durationMs: Date.now() - startedAt,
      ...(evidenceError ? { error: evidenceError } : {}),
    };
  } catch (err) {
    return {
      id: mission.id,
      source: mission.source,
      requestedUrl: mission.url,
      finalUrl: null,
      title: null,
      heading: null,
      linkCount: null,
      evidenceTerms: mission.evidenceTerms,
      checks: {
        canonicalUrl: false,
        pageIdentity: false,
        evidence: false,
        bounded: Date.now() - startedAt <= missionTimeoutMs,
      },
      passed: false,
      durationMs: Date.now() - startedAt,
      error: err.message,
    };
  }
}

async function readPageResource(mission, deadlineAt) {
  const timeoutMs = remainingTime(deadlineAt, 15000);
  const response = parseJsonOutput(
    (await runCli(context, [
      '--json',
      '--session', context.session,
      '--job-timeout-ms', String(Math.max(1, timeoutMs - 250)),
      'get', 'page',
      '--url', mission.url,
      '--max-bytes', '50000',
      '--timeout', String(Math.max(1, timeoutMs - 500)),
    ], timeoutMs)).stdout,
    `${mission.id} bounded page reader`,
  );
  assert(response.success === true, `${mission.id} page reader failed: ${JSON.stringify(response)}`);
  const data = response.data || {};
  assert(data.source === 'Network.loadNetworkResource', `${mission.id} used an unexpected page reader`);
  assert(data.includeCredentials === false, `${mission.id} page reader unexpectedly included credentials`);
  assert(data.bytesReturned <= 50000, `${mission.id} page reader exceeded its byte bound`);
  const html = String(data.text || '');
  return {
    url: data.url,
    activeTargetUrl: data.activeTargetUrl,
    title: decodeHtml(extractTagText(html, 'title')),
    heading: decodeHtml(extractTagText(html, 'h1')),
    text: htmlToText(html),
    linkCount: (html.match(/<a\b/gi) || []).length,
  };
}

function extractTagText(html, tag) {
  return html.match(new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)<\\/${tag}>`, 'i'))?.[1] || '';
}

function htmlToText(html) {
  return decodeHtml(
    String(html)
      .replace(/<!--[\s\S]*?-->/g, ' ')
      .replace(/<(script|style|template)\b[^>]*>[\s\S]*?<\/\1>/gi, ' ')
      .replace(/<[^>]+>/g, ' '),
  ).replace(/\s+/g, ' ').trim();
}

function decodeHtml(value) {
  return String(value)
    .replace(/&nbsp;|&#160;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#39;|&apos;/gi, "'")
    .replace(/&#(\d+);/g, (_match, code) => String.fromCodePoint(Number(code)))
    .replace(/&#x([0-9a-f]+);/gi, (_match, code) => String.fromCodePoint(Number.parseInt(code, 16)));
}

async function readBrowserField(field, deadlineAt) {
  try {
    const timeoutMs = remainingTime(deadlineAt, 5000);
    const response = parseJsonOutput(
      (await runCli(context, [
        '--json',
        '--session', context.session,
        '--job-timeout-ms', String(Math.max(1, timeoutMs - 250)),
        'get', field,
      ], timeoutMs)).stdout,
      `browser ${field} fallback`,
    );
    return response.data?.[field] || '';
  } catch {
    return '';
  }
}

function remainingTime(deadlineAt, capMs) {
  return Math.max(1, Math.min(capMs, deadlineAt - Date.now()));
}

function resolveBrowserExecutable() {
  const explicit = process.env.AGENT_BROWSER_RESEARCH_BROWSER_EXECUTABLE;
  if (explicit) {
    assert(existsSync(explicit), `Configured research browser executable is missing: ${explicit}`);
    return explicit;
  }
  const doctor = spawnSync(
    context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,
    ['install', 'doctor', '--json'],
    { encoding: 'utf8', env: process.env },
  );
  const payload = parseJsonOutput(doctor.stdout, 'agent-browser install doctor');
  const executable = payload.data?.launchConfig?.executablePath;
  assert(executable && existsSync(executable), `Install doctor returned no usable browser executable: ${doctor.stdout}`);
  return executable;
}

function createDisposableProfile(browserExecutable) {
  const explicitRoot = process.env.AGENT_BROWSER_RESEARCH_PROFILE_ROOT;
  if (explicitRoot) return mkdtempSync(join(explicitRoot, 'agent-browser-research-concierge-'));
  if (/^\/mnt\/[a-z]\//i.test(browserExecutable)) {
    const windowsTemp = execFileSync(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-Command', '[Console]::Out.Write([IO.Path]::GetTempPath())'],
      { encoding: 'utf8' },
    ).trim();
    const wslTemp = execFileSync('wslpath', ['-u', windowsTemp], { encoding: 'utf8' }).trim();
    return mkdtempSync(join(wslTemp, 'agent-browser-research-concierge-'));
  }
  return mkdtempSync(join(context.tempHome, 'agent-browser-research-concierge-profile-'));
}

async function cleanup() {
  if (cleanupComplete) return;
  cleanupComplete = true;
  await closeSession(context);
  if (profilePath) rmSync(profilePath, { recursive: true, force: true });
  context.cleanupTempHome();
}

function buildReport(extra = {}) {
  const passed = results.filter((result) => result.passed).length;
  const failed = results.length - passed;
  const recoveredAfterOpenTimeout = results.filter((result) => result.recoveredAfterOpenTimeout).length;
  return {
    schema: 'agent-browser.agentic-research-concierge.v1',
    generatedAt: new Date().toISOString(),
    posture: {
      publicOnly: true,
      authenticatedProfilesUsed: false,
      mutationsAllowed: false,
      promptSubmissionAllowed: false,
      retriesPerMission: 0,
      disposableSession: context.session,
      cleanupComplete,
    },
    score: {
      total: selectedMissions.length,
      attempted: results.length,
      passed,
      failed,
      percent: selectedMissions.length ? Math.round((passed / selectedMissions.length) * 100) : 0,
      recoveredAfterOpenTimeout,
    },
    success: results.length === selectedMissions.length && failed === 0 && cleanupComplete,
    durationMs: Date.now() - suiteStartedAt,
    results,
    ...extra,
  };
}

function normalize(value) {
  return String(value || '').replace(/\s+/g, ' ').trim().toLowerCase();
}

function positiveInteger(raw, fallback, label) {
  if (raw === undefined || raw === '') return fallback;
  const value = Number(raw);
  assert(Number.isInteger(value) && value > 0, `${label} must be a positive integer`);
  return value;
}
