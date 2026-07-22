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

async function postJson(path, payload, fetcher = fetch) {
  const response = await fetcher(path, {
    method: "POST",
    headers: { Accept: "application/json", "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
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

export async function fetchSkills(paneId, fetcher = fetch) {
  const body = await getJson(`/api/agents/${encodeURIComponent(paneId)}/skills`, fetcher);
  return body.skills;
}

export async function fetchLetters({ after, limit = 500 } = {}, fetcher = fetch) {
  const query = new URLSearchParams({ limit: String(limit) });
  if (after !== undefined && after !== null) query.set("after", String(after));
  return getJson(`/api/letters?${query}`, fetcher);
}

export async function sendLetter({ agent, skill, body }, fetcher = fetch) {
  return postJson("/api/letters", { agent, skill, body }, fetcher);
}
