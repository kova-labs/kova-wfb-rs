import type { MeshEvent, MeshNode, MeshState } from "./mesh-types";

export function createInitialState(): MeshState {
  return {
    nodes: new Map(),
    events: [],
    totalPackets: 0,
    packetsLastMinute: [],
  };
}

export function processEvent(state: MeshState, event: MeshEvent): MeshState {
  const newNodes = new Map(state.nodes);
  const now = Date.now();

  // Get or create sender node
  const senderNode: MeshNode = newNodes.get(event.sender) || {
    id: event.sender,
    status: "alive",
    position: event.position,
    lastSeen: now,
    sentCount: 0,
    receivedCount: 0,
    packetCount: 0,
  };

  // Update based on packet type
  switch (event.packet) {
    case "alive-ping":
      senderNode.status = "alive";
      senderNode.position = event.position;
      senderNode.lastSeen = now;
      break;

    case "device-missing":
      // Sender remains alive, but reports missing node
      senderNode.status = "alive";
      senderNode.lastSeen = now;
      
      if (event.missing !== undefined) {
        const missingNode: MeshNode = newNodes.get(event.missing) || {
          id: event.missing,
          status: "missing",
          position: { x: 0, y: 0, z: 0 },
          lastSeen: event.lastSeen || now,
          sentCount: 0,
          receivedCount: 0,
          packetCount: 0,
        };
        missingNode.status = "missing";
        if (event.lastSeen) {
          missingNode.lastSeen = event.lastSeen;
        }
        newNodes.set(event.missing, missingNode);
      }
      break;

    case "device-down":
      senderNode.status = "down";
      break;
  }

  // Update packet counts based on direction
  if (event.direction === "send") {
    senderNode.sentCount++;
  } else {
    senderNode.receivedCount++;
  }
  senderNode.packetCount++;

  newNodes.set(event.sender, senderNode);

  // Add event to feed (keep last 50)
  const newEvents = [event, ...state.events].slice(0, 50);

  // Update packets per minute tracking
  const oneMinuteAgo = now - 60000;
  const newPacketsLastMinute = [...state.packetsLastMinute, now].filter(
    (t) => t > oneMinuteAgo
  );

  return {
    nodes: newNodes,
    events: newEvents,
    totalPackets: state.totalPackets + 1,
    packetsLastMinute: newPacketsLastMinute,
  };
}

export function getNodeArray(state: MeshState): MeshNode[] {
  return Array.from(state.nodes.values()).sort((a, b) => a.id - b.id);
}

export function getStatusCounts(state: MeshState) {
  const nodes = getNodeArray(state);
  return {
    alive: nodes.filter((n) => n.status === "alive").length,
    missing: nodes.filter((n) => n.status === "missing").length,
    down: nodes.filter((n) => n.status === "down").length,
    total: nodes.length,
  };
}
