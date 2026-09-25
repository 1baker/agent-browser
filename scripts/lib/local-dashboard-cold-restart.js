/**
 * Guarded cold restart inside the publication journal/interlock lifecycle.
 *
 * This module never changes retention enforcement. The live adapter must fence
 * admission before closing, bind process handles to the reviewed identities,
 * prove profile idleness, and verify every binary/manifest/profile receipt.
 * A missing or ambiguous observation is a hard failure, never permission to
 * launch a replacement. The journal is write-ahead: recovery rolls back rather
 * than repeating a candidate launch after an uncertain response.
 */
export const COLD_RESTART_SCHEMA = 'agent-browser.cold-restart.v1';

const requiredAdapters = [
  'verifyIntent', 'fenceSource', 'closeSource', 'requireIdle', 'backupProfile',
  'verifyBackups', 'installPair', 'launch', 'qualify', 'fenceCandidate', 'closeCandidate',
  'restoreProfile', 'restoreDashboard', 'verifyFinal',
];

function validate(record, adapters) {
  if (record?.coldRestart?.schemaVersion !== COLD_RESTART_SCHEMA
    || record.coldRestart.intent?.approved !== true) {
    throw new Error('Cold restart requires explicit, journaled restart intent');
  }
  if (record.terminal) throw new Error('Cold restart transaction is already terminal');
  const cold = record.coldRestart;
  for (const flag of ['fenceAdmitted', 'closeAdmitted', 'sourceClosed',
    'candidateInstallAdmitted', 'candidateLaunchAdmitted', 'sourceLaunchAdmitted']) {
    if (cold[flag] !== undefined && typeof cold[flag] !== 'boolean') {
      throw new Error(`Cold restart admission flag must be boolean: ${flag}`);
    }
  }
  if ((cold.closeAdmitted && !cold.fenceAdmitted)
    || (cold.sourceClosed && !cold.closeAdmitted)
    || (cold.profileBackup && !cold.sourceClosed)
    || (cold.candidateInstallAdmitted && (!cold.sourceClosed || !cold.profileBackup))
    || (cold.candidateLaunchAdmitted && !cold.candidateInstallAdmitted)
    || (cold.sourceLaunchAdmitted && !cold.closeAdmitted)) {
    throw new Error('Cold restart admission record has impossible phase bindings');
  }
  for (const name of requiredAdapters) {
    if (typeof adapters[name] !== 'function') {
      throw new Error(`Cold restart adapter is required: ${name}`);
    }
  }
}

/**
 * The caller holds BOTH the publication journal lock and maintenance custody.
 * Adapters return bounded, secret-free receipts, not arbitrary service state.
 * `launch` is intent-bound: on recovery it may reuse a proven matching launch,
 * but must refuse unknown occupants instead of opening a duplicate profile.
 * `closeCandidate` must prove absence or close exactly the admitted launch;
 * a failed launch command does not itself prove that no browser was created.
 */
export async function runColdRestart({ journal, record, adapters, recover = false }) {
  validate(record, adapters);
  let current = record;
  const commit = (phase, coldPatch = {}, patch = {}) => {
    current = journal.commit(current, phase, {
      ...patch,
      coldRestart: { ...current.coldRestart, ...coldPatch },
    });
    return current;
  };
  const call = async (name, ...args) => {
    const result = await adapters[name](current, ...args);
    const proof = {
      verifyIntent: 'verified', verifyBackups: 'verified', fenceSource: 'verified',
      fenceCandidate: 'verified',
      requireIdle: 'idle', installPair: 'verified', qualify: 'verified',
      restoreProfile: 'restored', restoreSourceFence: 'restored',
    }[name];
    if (proof && result !== true && result?.[proof] !== true) {
      throw new Error(`Cold restart ${name} did not return positive ${proof} proof`);
    }
    return result;
  };
  const blocked = (error) => {
    // Do not overwrite a newer revision if a write-ahead commit itself failed.
    try { commit('recovery_blocked', { blocked: true, failure: error.message }); }
    catch { /* The last durable admitted phase remains the recovery authority. */ }
    throw error;
  };
  const finish = async (selection, launched) => {
    const qualified = await call('qualify', selection, launched);
    commit(`cold_${selection}_qualified`, { [`${selection}Qualified`]: qualified });
    await call('restoreDashboard');
    const verified = await call('verifyFinal', selection);
    if (verified?.verified !== true) throw new Error('Cold restart final verification did not pass');
    return commit(selection === 'candidate' ? 'ready' : 'rolled_back', {
      result: selection === 'candidate' ? 'candidate_verified' : 'source_restored',
      final: verified,
      blocked: false,
    }, current.retainedBrowserExpectation ? {
      retainedBrowserExpectation: { ...current.retainedBrowserExpectation, final: verified.retained ?? null },
    } : {});
  };
  const rollback = async () => {
    // This revalidates immutable plan/artifact identities, not stale live PIDs.
    await call('verifyIntent');
    if (!current.coldRestart.closeAdmitted) {
      // A fence may have changed admission or retired a controller. Its inverse
      // must be explicit and verified; do not guess from an adapter exception.
      if (current.coldRestart.fenceAdmitted) {
        if (typeof adapters.restoreSourceFence !== 'function') {
          throw new Error('Source fence outcome needs explicit verified restoration');
        }
        await call('restoreSourceFence');
      }
      await call('restoreDashboard');
      const verified = await call('verifyFinal', 'source');
      if (verified?.verified !== true) throw new Error('Unchanged source verification did not pass');
      return commit('rolled_back', { result: 'source_unchanged', final: verified, blocked: false },
        current.retainedBrowserExpectation ? {
          retainedBrowserExpectation: { ...current.retainedBrowserExpectation, final: verified.retained ?? null },
        } : {});
    }

    // A prior rollback may already have launched the old generation. The live
    // adapter must identify that exact launch without another launch or close.
    if (current.coldRestart.sourceLaunchAdmitted) {
      await call('verifyBackups');
      const launched = await call('launch', 'source');
      commit('cold_source_launched', { sourceLaunch: launched });
      return finish('source', launched);
    }

    if (current.coldRestart.candidateLaunchAdmitted) {
      // Forward qualification may have already restarted the dashboard.
      // Re-fence candidate admission before any rollback shutdown request.
      commit('cold_candidate_fence_admitted');
      await call('fenceCandidate');
      commit('cold_candidate_close_admitted');
      await call('closeCandidate');
    }
    // Includes original close failures: never replace files while that close
    // is uncertain, or while an unknown process occupies the profile.
    await call('requireIdle');
    commit('cold_rollback_idle', { sourceClosed: true });
    await call('verifyBackups');
    if (current.coldRestart.profileBackup) {
      commit('cold_profile_restore_admitted');
      const restoration = await call('restoreProfile');
      commit('cold_profile_restored', { profileRestoration: restoration });
    } else if (current.coldRestart.candidateInstallAdmitted) {
      throw new Error('Candidate replacement cannot recover without a verified cold profile backup');
    }
    await call('requireIdle');
    commit('cold_source_install_admitted');
    await call('installPair', 'source');
    await call('requireIdle');
    commit('cold_source_launch_admitted', { sourceLaunchAdmitted: true });
    const launched = await call('launch', 'source');
    commit('cold_source_launched', { sourceLaunch: launched });
    return finish('source', launched);
  };

  if (recover) {
    try { return await rollback(); }
    catch (error) { return blocked(error); }
  }
  if (current.coldRestart.fenceAdmitted || current.coldRestart.closeAdmitted
    || current.coldRestart.candidateInstallAdmitted) {
    throw new Error('Partially admitted cold restart requires explicit recovery');
  }
  try {
    await call('verifyIntent');
    // Binary/config backups are verified BEFORE any close. Profile backup is
    // captured only once the browser is stopped and its locks are released.
    await call('verifyBackups');
    commit('cold_fence_admitted', { fenceAdmitted: true });
    const fence = await call('fenceSource');
    commit('cold_fenced', { fence });
    await call('verifyIntent');
    commit('cold_close_admitted', { closeAdmitted: true });
    await call('closeSource');
    await call('requireIdle');
    commit('cold_source_closed', { sourceClosed: true });
    const profileBackup = await call('backupProfile');
    if (!profileBackup) throw new Error('Cold profile backup receipt is missing');
    commit('cold_profile_backed_up', { profileBackup });
    await call('verifyBackups');
    await call('requireIdle');
    commit('cold_candidate_install_admitted', { candidateInstallAdmitted: true });
    await call('installPair', 'candidate');
    commit('cold_candidate_installed');
    await call('requireIdle');
    commit('cold_candidate_launch_admitted', { candidateLaunchAdmitted: true });
    const launched = await call('launch', 'candidate');
    commit('cold_candidate_launched', { candidateLaunch: launched });
    return await finish('candidate', launched);
  } catch (error) {
    try {
      commit('cold_forward_failed', { failure: error.message });
      return await rollback();
    } catch (rollbackError) {
      return blocked(new AggregateError([error, rollbackError],
        `Cold restart did not complete and rollback is blocked: ${rollbackError.message}`));
    }
  }
}
