"use client";

import { useEffect, useState, useCallback } from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { HealthSummary } from "./health-summary";
import { NetworkMap } from "./network-map";
import { EventFeed } from "./event-feed";
import { DetailsPanel } from "./details-panel";
import { NodeTable } from "./node-table";
import { ConnectionPanel, StreamMode } from "./connection-panel";
import type { MeshState, MeshEvent } from "@/lib/mesh-types";
import {
  createInitialState,
  processEvent,
  getNodeArray,
} from "@/lib/mesh-state";
import { createMockEventStream } from "@/lib/mock-events";
import { useMeshPolling } from "@/lib/use-mesh-polling";
import { Radio, Wifi, Clock } from "lucide-react";

export function MeshDashboard() {
  const [state, setState] = useState<MeshState>(createInitialState);
  const [selectedNodeId, setSelectedNodeId] = useState<number | null>(null);
  const [currentTime, setCurrentTime] = useState<string>("");
  const [streamMode, setStreamMode] = useState<StreamMode>("mock");
  const [uuid, setUuid] = useState(0);
  const [liveConnecting, setLiveConnecting] = useState(false);
  const [liveConnected, setLiveConnected] = useState(false);

  // Handle incoming events — same handler for both mock and live
  const handleEvent = useCallback((event: MeshEvent) => {
    setState((prev) => processEvent(prev, event));
    setLiveConnecting(false);
    setLiveConnected(true);
  }, []);

  // Mock stream — only runs when mode is "mock"
  useEffect(() => {
    if (streamMode !== "mock") return;
    const cleanup = createMockEventStream(handleEvent, { min: 1000, max: 1800 });
    return cleanup;
  }, [streamMode, handleEvent]);

  // Live polling — only runs when mode is "live"
  // TODO: Replace useMeshPolling with WebSocket when backend supports it:
  // const socket = new WebSocket(`ws://172.10.20.4:8000/ws/${uuid}`)
  // socket.onmessage = (e) => handleEvent(JSON.parse(e.data))
  useMeshPolling({
    uuid,
    onEvent: handleEvent,
    enabled: streamMode === "live",
  });

  // Reset connection state when switching to live or changing UUID
  function handleModeChange(mode: StreamMode) {
    setStreamMode(mode);
    if (mode === "live") {
      setLiveConnected(false);
      setLiveConnecting(true);
    }
  }

  function handleUuidChange(newUuid: number) {
    setUuid(newUuid);
    setLiveConnected(false);
    setLiveConnecting(true);
  }

  // Update clock
  useEffect(() => {
    const updateTime = () => {
      setCurrentTime(
        new Date().toLocaleTimeString("en-US", {
          hour12: false,
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
        })
      );
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  const nodes = getNodeArray(state);
  const selectedNode = selectedNodeId !== null
    ? nodes.find((n) => n.id === selectedNodeId) || null
    : null;
  const lastEventType = state.events[0]?.packet || null;

  return (
    <div className="min-h-screen bg-background p-4 lg:p-6">
      {/* Header */}
      <header className="mb-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-cyan-500/10 p-2 border border-cyan-500/20">
              <Wifi className="h-6 w-6 text-cyan-400" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
                Meshinator
                <Badge variant="secondary" className="text-xs font-normal">
                  v1.0
                </Badge>
              </h1>
              <p className="text-sm text-muted-foreground">
                Real-time Tactical Mesh Awareness Layer
              </p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {/* Stream status badge — changes based on mode */}
            {streamMode === "mock" ? (
              <Badge
                variant="outline"
                className="text-emerald-400 border-emerald-500/30 bg-emerald-500/10 animate-pulse"
              >
                <Radio className="h-3 w-3 mr-1.5" />
                MOCK STREAM ACTIVE
              </Badge>
            ) : liveConnecting ? (
              <Badge
                variant="outline"
                className="text-amber-400 border-amber-500/30 bg-amber-500/10 animate-pulse"
              >
                <Radio className="h-3 w-3 mr-1.5" />
                CONNECTING…
              </Badge>
            ) : liveConnected ? (
              <Badge
                variant="outline"
                className="text-emerald-400 border-emerald-500/30 bg-emerald-500/10 animate-pulse"
              >
                <Radio className="h-3 w-3 mr-1.5" />
                LIVE — UUID {uuid}
              </Badge>
            ) : (
              <Badge
                variant="outline"
                className="text-red-400 border-red-500/30 bg-red-500/10"
              >
                <Radio className="h-3 w-3 mr-1.5" />
                NO SIGNAL
              </Badge>
            )}
            <Badge
              variant="outline"
              className="text-muted-foreground border-border font-mono"
            >
              <Clock className="h-3 w-3 mr-1.5" />
              {currentTime}
            </Badge>
          </div>
        </div>

        {/* Connection panel — sits below the title row */}
        <div className="mt-3">
          <ConnectionPanel
            mode={streamMode}
            uuid={uuid}
            onModeChange={handleModeChange}
            onUuidChange={handleUuidChange}
            connecting={liveConnecting}
            connected={liveConnected}
          />
        </div>
      </header>

      {/* Info Card */}
      <Card className="mb-6 bg-cyan-500/5 border-cyan-500/20">
        <CardContent className="py-3 px-4">
          <p className="text-sm text-muted-foreground">
            <span className="text-cyan-400 font-medium">Mission Brief:</span>{" "}
            This dashboard visualizes live packet events from a tactical radio
            mesh. Each packet updates the shared network state, allowing
            operators to identify active, missing, and down devices in real
            time.
          </p>
        </CardContent>
      </Card>

      {/* Health Summary */}
      <section className="mb-6">
        <HealthSummary state={state} lastEventType={lastEventType} />
      </section>

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 mb-4">
        {/* Network Map - 2 columns */}
        <div className="lg:col-span-2">
          <NetworkMap
            nodes={nodes}
            selectedNodeId={selectedNodeId}
            onSelectNode={setSelectedNodeId}
          />
        </div>

        {/* Details Panel - 1 column */}
        <div>
          <DetailsPanel node={selectedNode} />
        </div>
      </div>

      {/* Bottom Section */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <EventFeed events={state.events} />
        <NodeTable
          nodes={nodes}
          selectedNodeId={selectedNodeId}
          onSelectNode={setSelectedNodeId}
        />
      </div>

      {/* Footer */}
      <footer className="mt-6 text-center text-xs text-muted-foreground">
        <p>MeshOps Tactical Dashboard • Hackathon Demo Build</p>
      </footer>
    </div>
  );
}
