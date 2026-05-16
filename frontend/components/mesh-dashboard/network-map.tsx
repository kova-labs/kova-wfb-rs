"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { MeshNode } from "@/lib/mesh-types";
import { Radio, AlertTriangle, XCircle, MapPin } from "lucide-react";

interface NetworkMapProps {
  nodes: MeshNode[];
  selectedNodeId: number | null;
  onSelectNode: (id: number | null) => void;
}

const STATUS_CONFIG = {
  alive: {
    color: "fill-emerald-500",
    stroke: "stroke-emerald-400",
    label: "Active",
  },
  missing: {
    color: "fill-amber-500",
    stroke: "stroke-amber-400",
    label: "Missing",
  },
  down: {
    color: "fill-red-500",
    stroke: "stroke-red-400",
    label: "Down",
  },
};

export function NetworkMap({
  nodes,
  selectedNodeId,
  onSelectNode,
}: NetworkMapProps) {
  const SVG_W = 560;
  const SVG_H = 360;
  const PADDING = 60; // space around the outermost nodes
  const NODE_R = 14;

  // ── 1. Map real-world positions into SVG coordinates ──────────────────
  // Find bounding box of all node positions
  const xs = nodes.map((n) => n.position.x);
  const ys = nodes.map((n) => n.position.y);

  const minX = nodes.length ? Math.min(...xs) : 0;
  const maxX = nodes.length ? Math.max(...xs) : 1;
  const minY = nodes.length ? Math.min(...ys) : 0;
  const maxY = nodes.length ? Math.max(...ys) : 1;

  const rangeX = maxX - minX || 1; // avoid div/0 when all nodes overlap
  const rangeY = maxY - minY || 1;

  // Scale real positions into the padded SVG canvas
  function toSvg(nx: number, ny: number) {
    return {
      x: PADDING + ((nx - minX) / rangeX) * (SVG_W - PADDING * 2),
      y: PADDING + ((ny - minY) / rangeY) * (SVG_H - PADDING * 2),
    };
  }

  // ── 2. Collision resolution ────────────────────────────────────────────
  const MIN_DIST = 40;
  const positions = nodes.map((n) => {
    const p = toSvg(n.position.x, n.position.y);
    return { id: n.id, x: p.x, y: p.y };
  });

  for (let iter = 0; iter < 10; iter++) {
    for (let i = 0; i < positions.length; i++) {
      for (let j = i + 1; j < positions.length; j++) {
        const dx = positions[j].x - positions[i].x;
        const dy = positions[j].y - positions[i].y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        if (dist < MIN_DIST) {
          const overlap = (MIN_DIST - dist) / 2;
          const nx = (dx / dist) * overlap;
          const ny = (dy / dist) * overlap;
          positions[i].x -= nx;
          positions[i].y -= ny;
          positions[j].x += nx;
          positions[j].y += ny;
        }
      }
    }
  }

  // Clamp resolved positions so they stay inside the SVG after repulsion
  for (const p of positions) {
    p.x = Math.max(PADDING, Math.min(SVG_W - PADDING, p.x));
    p.y = Math.max(PADDING, Math.min(SVG_H - PADDING, p.y));
  }

  const resolvedPositions = new Map(positions.map((p) => [p.id, p]));

  // ── 3. Mesh connections between alive nodes ────────────────────────────
  const aliveNodes = nodes.filter((n) => n.status === "alive");
  const connections: { x1: number; y1: number; x2: number; y2: number }[] = [];

  for (let i = 0; i < aliveNodes.length; i++) {
    for (let j = i + 1; j < aliveNodes.length; j++) {
      const a = resolvedPositions.get(aliveNodes[i].id)!;
      const b = resolvedPositions.get(aliveNodes[j].id)!;
      // Connect all alive nodes — they're already scaled to fit so distance
      // threshold would be arbitrary; just connect all pairs
      connections.push({ x1: a.x, y1: a.y, x2: b.x, y2: b.y });
    }
  }

  return (
    <Card className="bg-card/50 backdrop-blur-sm border-border/50 h-full">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <MapPin className="h-4 w-4 text-cyan-400" />
            <span className="text-sm font-medium">Mesh Topology</span>
          </div>
          <div className="flex items-center gap-3 text-xs">
            {Object.entries(STATUS_CONFIG).map(([status, config]) => (
              <div key={status} className="flex items-center gap-1.5">
                <span
                  className={`h-2.5 w-2.5 rounded-full ${config.color.replace("fill-", "bg-")}`}
                />
                <span className="text-muted-foreground">{config.label}</span>
              </div>
            ))}
          </div>
        </div>
      </CardHeader>

      <CardContent className="p-2">
        <svg
          viewBox={`0 0 ${SVG_W} ${SVG_H}`}
          className="w-full h-auto"
          style={{ maxHeight: "340px" }}
        >
          <defs>
            {/* Grid pattern */}
            <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
              <path
                d="M 40 0 L 0 0 0 40"
                fill="none"
                stroke="currentColor"
                strokeWidth="0.5"
                className="text-border/30"
              />
            </pattern>
            {/* Glow filters */}
            {Object.keys(STATUS_CONFIG).map((status) => (
              <filter key={status} id={`glow-${status}`}>
                <feGaussianBlur stdDeviation="3" result="coloredBlur" />
                <feMerge>
                  <feMergeNode in="coloredBlur" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
            ))}
          </defs>

          {/* Background grid */}
          <rect width={SVG_W} height={SVG_H} fill="url(#grid)" />

          {/* Connection lines */}
          {connections.map((c, idx) => (
            <line
              key={`conn-${idx}`}
              x1={c.x1} y1={c.y1}
              x2={c.x2} y2={c.y2}
              stroke="#06b6d4"
              strokeWidth="1"
              strokeOpacity="0.3"
              strokeDasharray="4 2"
            />
          ))}

          {/* Nodes */}
          {nodes.map((node) => {
            const config = STATUS_CONFIG[node.status];
            const isSelected = node.id === selectedNodeId;
            const pos = resolvedPositions.get(node.id);
            if (!pos) return null;
            const { x, y } = pos;

            return (
              <g
                key={node.id}
                className="cursor-pointer"
                onClick={() => onSelectNode(node.id)}
              >
                {/* Pulse ring for alive nodes */}
                {node.status === "alive" && (
                  <circle
                    cx={x}
                    cy={y}
                    r="18"
                    className="fill-none stroke-emerald-400/40 mesh-pulse-ring"
                    strokeWidth="2"
                  />
                )}

                {/* Selection ring */}
              {isSelected && (
                  <circle
                    cx={x} cy={y} r="22"
                    fill="none"
                    stroke="#06b6d4"
                    strokeWidth="2"
                    strokeOpacity="0.8"
                  />
                )}

                {/* Node circle */}
                <circle
                  cx={x} cy={y} r={NODE_R}
                  className={`${config.color} ${config.stroke} stroke-2`}
                  filter={`url(#glow-${node.status})`}
                />

                {/* Node ID label */}
                <text
                  x={x} y={y + 1}
                  textAnchor="middle"
                  dominantBaseline="middle"
                  className="fill-white font-mono text-xs font-bold"
                  style={{ fontSize: "11px", fontWeight: "bold" }}
                >
                  {node.id}
                </text>

                {/* Small status dot */}
                {/*
                <circle
                  cx={x + 12} cy={y - 12} r="5"
                  className={`${config.color} ${node.status !== "alive" ? "animate-pulse" : ""}`}
                />
                */}

                {/* Node label below */}
                <text
                  x={x} y={y + NODE_R + 12}
                  textAnchor="middle"
                  className="fill-muted-foreground"
                  style={{ fontSize: "9px" }}
                >
                  {`${node.position.x.toFixed(1)}, ${node.position.y.toFixed(1)}`}
                </text>
              </g>
            );
          })}

          {/* Empty state */}
          {nodes.length === 0 && (
            <text
              x={SVG_W / 2} y={SVG_H / 2}
              textAnchor="middle"
              dominantBaseline="middle"
              className="fill-muted-foreground"
              style={{ fontSize: "13px" }}
            >
              Waiting for nodes…
            </text>
          )}
        </svg>
      </CardContent>
    </Card>
  );
}