/** Fetch the service-owned, read-only recent task projection. */
export async function getServiceRuns({ baseUrl, fetchImpl = fetch }) {
  const response = await fetchImpl(`${baseUrl.replace(/\/$/, '')}/runs`);
  const payload = await response.json();
  if (!response.ok || payload?.success !== true) {
    throw new Error(payload?.error || `Service runs request failed with HTTP ${response.status}`);
  }
  return payload.data;
}
