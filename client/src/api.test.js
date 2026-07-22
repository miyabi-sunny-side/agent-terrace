import { describe, expect, it, vi } from "vitest";

import {
  ApiError,
  fetchAgents,
  fetchLetters,
  fetchScreen,
  fetchSkills,
  sendLetter,
} from "./api.js";

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

  it("requests skills for one encoded pane", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ skills: ["bump-tag", "deliver"] }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await expect(fetchSkills("%38", fetcher)).resolves.toEqual(["bump-tag", "deliver"]);
    expect(fetcher).toHaveBeenCalledWith("/api/agents/%2538/skills", expect.any(Object));
  });

  it("requests mailbox deltas with explicit bounds", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ version: 1, mailbox: "mobile", events: [] }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await fetchLetters({ after: 41, limit: 500 }, fetcher);
    expect(fetcher).toHaveBeenCalledWith(
      "/api/letters?limit=500&after=41",
      expect.objectContaining({ headers: { Accept: "application/json" } }),
    );
  });

  it("keeps skill separate from the letter body", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ id: 42, status: "sent" }), {
        status: 201,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await sendLetter({ agent: "%38", skill: "deliver", body: "fix the parser" }, fetcher);
    expect(fetcher).toHaveBeenCalledWith("/api/letters", {
      method: "POST",
      headers: { Accept: "application/json", "Content-Type": "application/json" },
      body: JSON.stringify({ agent: "%38", skill: "deliver", body: "fix the parser" }),
    });
  });
});
