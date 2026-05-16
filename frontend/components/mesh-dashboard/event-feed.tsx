"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { MeshEvent } from "@/lib/mesh-types";
import {
  Activity,
  AlertTriangle,
  XCircle,
  ArrowUpRight,
  ArrowDownLeft,
  List,
} from "lucide-react";

interface EventFeedProps {
  events: MeshEvent[];
}

const PACKET_CONFIG = {
  "alive-ping": {
    icon: Activity,
    color: "text-emerald-400",
    bgColor: "bg-emerald-500/10",
    label: "PING",
  },
  "device-missing": {
    icon: AlertTriangle,
    color: "text-amber-400",
    bgColor: "bg-amber-500/10",
    label: "MISSING",
  },
  "device-down": {
    icon: XCircle,
    color: "text-red-400",
    bgColor: "bg-red-500/10",
    label: "DOWN",
  },
};

function formatTime(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleTimeString("en-US", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function getEventMessage(event: MeshEvent): string {
  const directionWord = event.direction === "send" ? "sent" : "received";

  switch (event.packet) {
    case "alive-ping":
      return `Node ${event.sender} ${directionWord} alive ping`;
    case "device-missing":
      return `Node ${event.sender} reported node ${event.missing} missing`;
    case "device-down":
      return `Node ${event.sender} marked as down`;
    default:
      return `Node ${event.sender} packet`;
  }
}

export function EventFeed({ events }: EventFeedProps) {
  const displayEvents = events.slice(0, 20);

  return (
    <Card className="bg-card/50 backdrop-blur-sm border-border/50 h-full">
      <CardHeader className="pb-2">
        <div className="flex items-center gap-2">
          <List className="h-4 w-4 text-cyan-400" />
          <CardTitle className="text-sm font-medium">Event Feed</CardTitle>
          <Badge variant="secondary" className="ml-auto text-xs">
            {events.length} events
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="p-0">
        <ScrollArea className="h-[200px]">
          <div className="space-y-1 p-3 pt-0">
            {displayEvents.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-4">
                Waiting for events...
              </p>
            ) : (
              displayEvents.map((event, idx) => {
                const config = PACKET_CONFIG[event.packet];
                const DirectionIcon =
                  event.direction === "send" ? ArrowUpRight : ArrowDownLeft;

                return (
                  <div
                    key={`${event.time}-${event.sender}-${idx}`}
                    className={`flex items-center gap-2 text-xs p-2 rounded-md ${config.bgColor} border border-transparent hover:border-border/50 transition-all animate-in fade-in-0 slide-in-from-top-1 duration-300`}
                    style={{ animationDelay: `${idx * 50}ms` }}
                  >
                    <span className="text-muted-foreground font-mono w-16 shrink-0">
                      {formatTime(event.time)}
                    </span>
                    <config.icon className={`h-3.5 w-3.5 ${config.color} shrink-0`} />
                    <Badge
                      variant="outline"
                      className={`${config.color} border-current/30 text-[10px] px-1.5 py-0 shrink-0`}
                    >
                      {config.label}
                    </Badge>
                    <DirectionIcon
                      className={`h-3 w-3 shrink-0 ${
                        event.direction === "send"
                          ? "text-cyan-400"
                          : "text-purple-400"
                      }`}
                    />
                    <span className="text-foreground/80 truncate">
                      {getEventMessage(event)}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
