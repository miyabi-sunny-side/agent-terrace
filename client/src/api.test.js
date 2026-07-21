import { describe, expect, it, vi } from "vitest";

import { ApiError, fetchAgents, fetchScreen } from "./api.js";

describe("API client", () => {
  it("encodes the pane as one path segment", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ pane_id: "%38", content: "ready" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await expect(fetchScreen("%38", fetcher)).resolves.toMatchObject({ content: "ready" });
    expect(fetcher).toHaveBeenCalledWith("/api/agents/%2538/screen", expect.any(Object));
  });

  it("returns structured server errors", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ code: "registry_unavailable", message: "registry down" }), {
        status: 502,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await expect(fetchAgents(fetcher)).rejects.toEqual(
      new ApiError("registry down", "registry_unavailable", 502),
    );
  });
});
