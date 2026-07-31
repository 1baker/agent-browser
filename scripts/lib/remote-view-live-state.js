export function selectRouteOwnedStream(state, routeId, browserId) {
  const browser = state?.browsers?.[browserId];
  const retainedBrowserStream = browser?.viewStreams?.find(
    (candidate) => candidate?.routeId === routeId,
  );
  if (retainedBrowserStream) return retainedBrowserStream;

  return Object.values(state?.viewStreams || {}).find(
    (candidate) => candidate?.routeId === routeId && candidate?.browserId === browserId,
  ) || null;
}

export function serviceEvaluateValue(response) {
  const result = response?.data?.result;
  return result?.result?.value ?? result?.value ?? result ?? null;
}
