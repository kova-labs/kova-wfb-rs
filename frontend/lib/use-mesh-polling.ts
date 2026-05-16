// lib/use-mesh-polling.ts
//
// Polls the backend POST /data/{uuid} endpoint for live MeshEvents.
// Drop this in place of your mock event generator.
//
// TODO: If the backend later adds WebSocket support, replace this hook with:
//   const socket = new WebSocket(`ws://172.10.20.4:8000/ws/${uuid}`)
//   socket.onmessage = (e) => dispatch(normalizeMeshEvent(JSON.parse(e.data)))

import { useEffect, useRef } from "react";
import { MeshEvent } from "./mesh-types";

const BACKEND_BASE = "/api/mesh";  // was "http://172.10.20.4:8000"
const POLL_INTERVAL_MS = 1000;

// Normalizes raw backend payload to our MeshEvent shape.
// The backend uses "last-seen" (hyphen); our type uses "lastSeen" (camelCase).
function normalizeMeshEvent(raw: Record<string, unknown>): MeshEvent {
  return {
    packet: raw["packet"] as MeshEvent["packet"],
    direction: raw["direction"] as MeshEvent["direction"],
    sender: raw["sender"] as number,
    position: raw["position"] as MeshEvent["position"],
    time: raw["time"] as number,
    missing: raw["missing"] as number | undefined,
    lastSeen: (raw["last-seen"] ?? raw["lastSeen"] ?? null) as number | null,
  };
}

interface UseMeshPollingOptions {
  uuid: number;
  onEvent: (event: MeshEvent) => void;
  enabled?: boolean;
}

export function useMeshPolling({
  uuid,
  onEvent,
  enabled = true,
}: UseMeshPollingOptions) {
  // Keep a stable ref to onEvent so we don't re-register the interval
  // every time the parent re-renders.
  const onEventRef = useRef(onEvent);
  useEffect(() => {
    onEventRef.current = onEvent;
  }, [onEvent]);

  useEffect(() => {
    if (!enabled) return;

    let active = true;

    async function poll() {
      try {
        const res = await fetch(`${BACKEND_BASE}/data/${uuid}`, {
          method: "GET",
        });

        if (!res.ok) {
          console.warn(`[MeshPolling] HTTP ${res.status} from /data/${uuid}`);
          return;
        }

        const raw = await res.json();

        if (!active) return;

        // The endpoint may return a single event or an array of events.
        const events: Record<string, unknown>[] = Array.isArray(raw)
          ? raw
          : [raw];

        for (const rawEvent of events) {
          // Skip empty/null entries
          if (!rawEvent || typeof rawEvent !== "object") continue;
          // Basic sanity check — must have at least a packet type
          if (!rawEvent["packet"]) continue;

          onEventRef.current(normalizeMeshEvent(rawEvent));
        }
      } catch (err) {
        // Network errors during a hackathon are expected — log quietly.
        console.warn("[MeshPolling] fetch error:", err);
      }
    }

    // Poll immediately on mount, then on interval
    poll();
    const intervalId = setInterval(poll, POLL_INTERVAL_MS);

    return () => {
      active = false;
      clearInterval(intervalId);
    };
  }, [uuid, enabled]);
}
