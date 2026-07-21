export const MAX_BODY_BYTES = 16 * 1024;
export const MAX_EVENTS = 500;

export function bodyBytes(value) {
  return new TextEncoder().encode(value).length;
}

export function mergeEvents(current, incoming) {
  const byId = new Map(current.map((event) => [event.id, event]));
  for (const event of incoming) byId.set(event.id, event);
  return [...byId.values()].sort((left, right) => left.id - right.id).slice(-MAX_EVENTS);
}

export function eventsForPane(events, paneId) {
  return events.filter((event) => event.target_pane === paneId);
}

export function latestEventId(events) {
  return events.length ? events.at(-1).id : null;
}
