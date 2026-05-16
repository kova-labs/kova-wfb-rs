import type { MeshEvent, PacketType, Direction, Position } from "./mesh-types";

// Initialize nodes with fixed positions for consistent visualization
const NODE_POSITIONS: Record<number, Position> = {
  1: { x: 180, y: 120, z: 0 },
  2: { x: 380, y: 120, z: 5 },
  3: { x: 280, y: 260, z: 2 },
};

const NODE_IDS = [1, 2, 3];

let eventCounter = 0;

function generateId(): string {
  return `evt-${Date.now()}-${eventCounter++}`;
}

function randomElement<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

function getPacketType(): PacketType {
  const rand = Math.random();
  // 85% alive-ping, 12% device-missing, 3% device-down
  if (rand < 0.85) return "alive-ping";
  if (rand < 0.97) return "device-missing";
  return "device-down";
}

function getDirection(): Direction {
  return Math.random() < 0.6 ? "send" : "receive";
}

function jitterPosition(pos: Position): Position {
  return {
    x: pos.x + (Math.random() - 0.5) * 20,
    y: pos.y + (Math.random() - 0.5) * 20,
    z: pos.z + (Math.random() - 0.5) * 2,
  };
}

export function generateMockEvent(): MeshEvent {
  const packetType = getPacketType();
  const sender = randomElement(NODE_IDS);
  const direction = getDirection();
  const now = Date.now();

  const baseEvent: MeshEvent = {
    id: generateId(),
    packet: packetType,
    direction,
    sender,
    position: jitterPosition(NODE_POSITIONS[sender]),
    time: now,
  };

  if (packetType === "device-missing") {
    // Pick a different node to be missing
    const otherNodes = NODE_IDS.filter((id) => id !== sender);
    const missingNode = randomElement(otherNodes);
    return {
      ...baseEvent,
      missing: missingNode,
      lastSeen: now - Math.floor(Math.random() * 10000) - 2000,
    };
  }

  return baseEvent;
}

// Hook for real-time mock stream
export function createMockEventStream(
  onEvent: (event: MeshEvent) => void,
  intervalMs: { min: number; max: number } = { min: 1000, max: 1800 }
): () => void {
  let timeoutId: NodeJS.Timeout;
  let isRunning = true;

  function scheduleNext() {
    if (!isRunning) return;
    
    const delay =
      intervalMs.min + Math.random() * (intervalMs.max - intervalMs.min);
    
    timeoutId = setTimeout(() => {
      if (isRunning) {
        onEvent(generateMockEvent());
        scheduleNext();
      }
    }, delay);
  }

  // Start immediately with first event
  onEvent(generateMockEvent());
  scheduleNext();

  // Return cleanup function
  return () => {
    isRunning = false;
    clearTimeout(timeoutId);
  };
}

// TODO: Replace mock event generator with WebSocket:
// export function createWebSocketStream(
//   url: string,
//   onEvent: (event: MeshEvent) => void
// ): () => void {
//   const socket = new WebSocket(url);
//   
//   socket.onmessage = (msg) => {
//     const event = JSON.parse(msg.data) as MeshEvent;
//     onEvent(event);
//   };
//   
//   return () => socket.close();
// }
