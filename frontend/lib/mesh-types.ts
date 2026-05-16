// Packet types from backend
export type PacketType = "alive-ping" | "device-missing" | "device-down";

export type Direction = "send" | "receive";

export type Position = {
  x: number;
  y: number;
  z: number;
};

export type MeshEvent = {
  id: string;
  packet: PacketType;
  direction: Direction;
  sender: number;
  position: Position;
  time: number;
  missing?: number;
  lastSeen?: number | null;
};

// Frontend node state
export type NodeStatus = "alive" | "missing" | "down";

export type MeshNode = {
  id: number;
  status: NodeStatus;
  position: Position;
  lastSeen: number;
  sentCount: number;
  receivedCount: number;
  packetCount: number;
};

// Dashboard state
export type MeshState = {
  nodes: Map<number, MeshNode>;
  events: MeshEvent[];
  totalPackets: number;
  packetsLastMinute: number[];
};
