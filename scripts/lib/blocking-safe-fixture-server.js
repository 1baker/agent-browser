import { Worker } from 'node:worker_threads';

const workerUrl = new URL('./blocking-safe-fixture-server-worker.js', import.meta.url);

export function startBlockingSafeFixtureServer({ html, host = '127.0.0.1' }) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(workerUrl, {
      workerData: { html, host },
    });
    let ready = false;
    let closePromise = null;

    const failBeforeReady = (error) => {
      if (ready) return;
      reject(error instanceof Error ? error : new Error(String(error)));
    };

    worker.once('error', failBeforeReady);
    worker.once('exit', (code) => {
      if (!ready && code !== 0) {
        failBeforeReady(new Error(`fixture server worker exited with status ${code}`));
      }
    });
    worker.on('message', (message) => {
      if (message?.type === 'error') {
        failBeforeReady(new Error(message.message || 'fixture server worker failed'));
        return;
      }
      if (message?.type !== 'ready' || ready) return;
      ready = true;
      resolve({
        host: message.host,
        port: message.port,
        close() {
          if (closePromise) return closePromise;
          closePromise = new Promise((closeResolve, closeReject) => {
            const onError = (error) => closeReject(error);
            const onExit = (code) => {
              if (code === 0) closeResolve();
              else closeReject(new Error(`fixture server worker exited with status ${code}`));
            };
            const onMessage = (closeMessage) => {
              if (closeMessage?.type !== 'closed') return;
              worker.off('error', onError);
              worker.off('exit', onExit);
              worker.off('message', onMessage);
              closeResolve();
            };
            worker.once('error', onError);
            worker.once('exit', onExit);
            worker.on('message', onMessage);
            worker.postMessage({ type: 'close' });
          });
          return closePromise;
        },
      });
    });
  });
}
