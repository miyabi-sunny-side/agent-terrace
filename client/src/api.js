export class ApiError extends Error {
  constructor(message, code, status) {
    super(message);
    this.name = "ApiError";
    this.code = code;
    this.status = status;
  }
}

async function getJson(path, fetcher = fetch) {
  const response = await fetcher(path, { headers: { Accept: "application/json" } });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new ApiError(
      body.message ?? "サーバーと通信できませんでした",
      body.code,
      response.status,
    );
  }
  return body;
}

export async function fetchAgents(fetcher = fetch) {
  const body = await getJson("/api/agents", fetcher);
  return body.agents;
}

export async function fetchScreen(paneId, fetcher = fetch) {
  return getJson(`/api/agents/${encodeURIComponent(paneId)}/screen`, fetcher);
}
