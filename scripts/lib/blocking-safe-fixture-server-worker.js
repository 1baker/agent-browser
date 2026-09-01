import { createServer } from 'node:http';
import { parentPort, workerData } from 'node:worker_threads';

if (!parentPort) {
  throw new Error('blocking-safe fixture server requires a worker parent port');
}

const host = workerData?.host || '127.0.0.1';
const html = String(workerData?.html || '');
const server = createServer((_req, res) => {
  res.writeHead(200, {
    'content-type': 'text/html; charset=utf-8',
    'cache-control': 'no-store',
  });
  res.end(html);
});

server.once('error', (error) => {
  parentPort.postMessage({
    type: 'error',
    message: error.message,
  });
});

server.listen(0, host, () => {
  const address = server.address();
  parentPort.postMessage({
    type: 'ready',
    host,
    port: address.port,
  });
});

parentPort.on('message', (message) => {
  if (message?.type !== 'close') return;
  server.close(() => {
    parentPort.postMessage({ type: 'closed' });
    parentPort.close();
  });
});
