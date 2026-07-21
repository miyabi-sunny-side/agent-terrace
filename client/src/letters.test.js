import { describe, expect, it } from "vitest";

import { bodyBytes, eventsForPane, latestEventId, MAX_EVENTS, mergeEvents } from "./letters.js";

const event = (id, pane = "%1") => ({ id, target_pane: pane, body: `body-${id}` });

describe("letter timeline helpers", () => {
  it("merges polling overlap without duplicates and preserves id order", () => {
    const merged = mergeEvents([event(2), event(4)], [event(3), { ...event(4), body: "updated" }]);
    expect(merged.map(({ id }) => id)).toEqual([2, 3, 4]);
    expect(merged.at(-1).body).toBe("updated");
    expect(latestEventId(merged)).toBe(4);
  });

  it("filters a shared mailbox to the selected pane", () => {
    expect(eventsForPane([event(1, "%1"), event(2, "%2"), event(3, "%1")], "%1")).toEqual([
      event(1, "%1"),
      event(3, "%1"),
    ]);
  });

  it("counts UTF-8 bytes instead of JavaScript code units", () => {
    expect(bodyBytes("abc")).toBe(3);
    expect(bodyBytes("手紙")).toBe(6);
  });

  it("keeps only the mailbox retention window", () => {
    const events = Array.from({ length: MAX_EVENTS + 2 }, (_, id) => ({ id }));
    const merged = mergeEvents([], events);
    expect(merged).toHaveLength(MAX_EVENTS);
    expect(merged[0].id).toBe(2);
  });
});
