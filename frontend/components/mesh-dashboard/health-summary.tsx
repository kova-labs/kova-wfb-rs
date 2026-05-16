"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { MeshState } from "@/lib/mesh-types";
import { getStatusCounts } from "@/lib/mesh-state";
import {
  Activity,
  Radio,
  AlertTriangle,
  XCircle,
  Zap,
  Clock,
} from "lucide-react";

interface HealthSummaryProps {
  state: MeshState;
  lastEventType: string | null;
}

export function HealthSummary({ state, lastEventType }: HealthSummaryProps) {
  const counts = getStatusCounts(state);
  const packetsPerMinute = state.packetsLastMinute.length;

  const stats = [
    {
      label: "Active Nodes",
      value: counts.alive,
      icon: Radio,
      color: "text-emerald-400",
      bgColor: "bg-emerald-500/10",
      borderColor: "border-emerald-500/20",
    },
    {
      label: "Missing Nodes",
      value: counts.missing,
      icon: AlertTriangle,
      color: "text-amber-400",
      bgColor: "bg-amber-500/10",
      borderColor: "border-amber-500/20",
    },
    {
      label: "Down Nodes",
      value: counts.down,
      icon: XCircle,
      color: "text-red-400",
      bgColor: "bg-red-500/10",
      borderColor: "border-red-500/20",
    },
    {
      label: "Total Packets",
      value: state.totalPackets,
      icon: Activity,
      color: "text-cyan-400",
      bgColor: "bg-cyan-500/10",
      borderColor: "border-cyan-500/20",
    },
    {
      label: "Packets/Min",
      value: packetsPerMinute,
      icon: Zap,
      color: "text-cyan-400",
      bgColor: "bg-cyan-500/10",
      borderColor: "border-cyan-500/20",
    },
    {
      label: "Last Event",
      value: lastEventType || "—",
      icon: Clock,
      color: "text-slate-400",
      bgColor: "bg-slate-500/10",
      borderColor: "border-slate-500/20",
      isText: true,
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
      {stats.map((stat) => (
        <Card
          key={stat.label}
          className={`${stat.bgColor} border ${stat.borderColor} bg-card/50 backdrop-blur-sm`}
        >
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-1 pt-3 px-3">
            <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
              {stat.label}
            </CardTitle>
            <stat.icon className={`h-4 w-4 ${stat.color}`} />
          </CardHeader>
          <CardContent className="px-3 pb-3">
            {stat.isText ? (
              <Badge
                variant="secondary"
                className="text-xs font-mono bg-secondary/50"
              >
                {stat.value}
              </Badge>
            ) : (
              <p className={`text-2xl font-bold font-mono ${stat.color}`}>
                {stat.value}
              </p>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
