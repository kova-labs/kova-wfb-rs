"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { MeshNode } from "@/lib/mesh-types";
import {
  Radio,
  AlertTriangle,
  XCircle,
  MapPin,
  Send,
  Download,
  Box,
  Info,
} from "lucide-react";

interface DetailsPanelProps {
  node: MeshNode | null;
}

const STATUS_CONFIG = {
  alive: {
    icon: Radio,
    color: "text-emerald-400",
    bgColor: "bg-emerald-500/10",
    borderColor: "border-emerald-500/30",
    label: "ACTIVE",
    description:
      "Node is actively participating in the mesh network and responding to health checks.",
  },
  missing: {
    icon: AlertTriangle,
    color: "text-amber-400",
    bgColor: "bg-amber-500/10",
    borderColor: "border-amber-500/30",
    label: "MISSING",
    description:
      "Node has not been observed recently or was reported missing by another device in the mesh.",
  },
  down: {
    icon: XCircle,
    color: "text-red-400",
    bgColor: "bg-red-500/10",
    borderColor: "border-red-500/30",
    label: "DOWN",
    description:
      "Node is considered unavailable and not responding to mesh communications.",
  },
};

function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const seconds = Math.floor(diff / 1000);

  if (seconds < 1) return "just now";
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}

export function DetailsPanel({ node }: DetailsPanelProps) {
  if (!node) {
    return (
      <Card className="bg-card/50 backdrop-blur-sm border-border/50 h-full">
        <CardHeader className="pb-2">
          <div className="flex items-center gap-2">
            <Info className="h-4 w-4 text-cyan-400" />
            <CardTitle className="text-sm font-medium">Node Details</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="flex flex-col items-center justify-center h-[calc(100%-60px)] text-center p-6">
          <div className="rounded-full bg-muted/50 p-4 mb-4">
            <Box className="h-8 w-8 text-muted-foreground" />
          </div>
          <p className="text-sm text-muted-foreground">
            Select a node from the mesh map to inspect telemetry and operational
            status.
          </p>
        </CardContent>
      </Card>
    );
  }

  const config = STATUS_CONFIG[node.status];
  const StatusIcon = config.icon;

  const stats = [
    {
      label: "Sent",
      value: node.sentCount,
      icon: Send,
      color: "text-cyan-400",
    },
    {
      label: "Received",
      value: node.receivedCount,
      icon: Download,
      color: "text-purple-400",
    },
    {
      label: "Total",
      value: node.packetCount,
      icon: Box,
      color: "text-foreground",
    },
  ];

  return (
    <Card className="bg-card/50 backdrop-blur-sm border-border/50 h-full">
      <CardHeader className="pb-2">
        <div className="flex items-center gap-2">
          <Info className="h-4 w-4 text-cyan-400" />
          <CardTitle className="text-sm font-medium">Node Details</CardTitle>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Node Header */}
        <div
          className={`flex items-center gap-3 p-3 rounded-lg ${config.bgColor} border ${config.borderColor}`}
        >
          <div
            className={`rounded-full p-2 ${config.bgColor} border ${config.borderColor}`}
          >
            <StatusIcon className={`h-5 w-5 ${config.color}`} />
          </div>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <span className="font-mono text-lg font-bold">
                Node {node.id}
              </span>
              <Badge
                variant="outline"
                className={`${config.color} border-current/30 text-xs`}
              >
                {config.label}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground">
              Last seen: {formatRelativeTime(node.lastSeen)}
            </p>
          </div>
        </div>

        {/* Position */}
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-xs text-muted-foreground uppercase tracking-wider">
            <MapPin className="h-3 w-3" />
            Position
          </div>
          <div className="grid grid-cols-3 gap-2">
            {(["x", "y", "z"] as const).map((axis) => (
              <div
                key={axis}
                className="bg-secondary/50 rounded-md p-2 text-center"
              >
                <span className="text-xs text-muted-foreground uppercase">
                  {axis}
                </span>
                <p className="font-mono text-sm">
                  {node.position[axis].toFixed(1)}
                </p>
              </div>
            ))}
          </div>
        </div>

        {/* Packet Stats */}
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-xs text-muted-foreground uppercase tracking-wider">
            <Box className="h-3 w-3" />
            Packet Statistics
          </div>
          <div className="grid grid-cols-3 gap-2">
            {stats.map((stat) => (
              <div
                key={stat.label}
                className="bg-secondary/50 rounded-md p-2 text-center"
              >
                <stat.icon className={`h-3 w-3 mx-auto mb-1 ${stat.color}`} />
                <p className={`font-mono text-sm font-bold ${stat.color}`}>
                  {stat.value}
                </p>
                <span className="text-[10px] text-muted-foreground uppercase">
                  {stat.label}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Status Description */}
        <div className="text-xs text-muted-foreground border-t border-border/50 pt-3">
          {config.description}
        </div>
      </CardContent>
    </Card>
  );
}
