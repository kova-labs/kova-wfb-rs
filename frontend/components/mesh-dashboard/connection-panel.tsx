// components/mesh-dashboard/connection-panel.tsx
//
// Renders a small toolbar the operator can use during the demo to:
//   - Switch between mock data and the live backend
//   - Set the UUID on the fly (no code changes needed)
//   - See the current connection status

"use client";

import { useState } from "react";
import { Wifi, WifiOff, FlaskConical } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export type StreamMode = "mock" | "live";

interface ConnectionPanelProps {
  mode: StreamMode;
  uuid: number;
  onModeChange: (mode: StreamMode) => void;
  onUuidChange: (uuid: number) => void;
  /** Pass true while waiting for the first response from the backend */
  connecting?: boolean;
  /** Pass true once at least one event has been received */
  connected?: boolean;
}

export function ConnectionPanel({
  mode,
  uuid,
  onModeChange,
  onUuidChange,
  connecting = false,
  connected = false,
}: ConnectionPanelProps) {
  const [uuidInput, setUuidInput] = useState(String(uuid));

  function handleUuidSubmit() {
    const parsed = parseInt(uuidInput, 10);
    if (!isNaN(parsed)) onUuidChange(parsed);
  }

  const statusBadge = () => {
    if (mode === "mock") {
      return (
        <Badge variant="secondary" className="gap-1">
          <FlaskConical className="h-3 w-3" />
          MOCK STREAM ACTIVE
        </Badge>
      );
    }
    if (connecting) {
      return (
        <Badge variant="outline" className="gap-1 animate-pulse">
          <Wifi className="h-3 w-3" />
          CONNECTING…
        </Badge>
      );
    }
    if (connected) {
      return (
        <Badge className="gap-1 bg-emerald-600 text-white">
          <Wifi className="h-3 w-3" />
          LIVE — UUID {uuid}
        </Badge>
      );
    }
    return (
      <Badge variant="destructive" className="gap-1">
        <WifiOff className="h-3 w-3" />
        NO SIGNAL
      </Badge>
    );
  };

  return (
    <div className="flex flex-wrap items-center gap-4 rounded-lg border border-border/50 bg-muted/30 px-4 py-2">
      {/* Status badge */}
      <div className="flex items-center gap-2">{statusBadge()}</div>

      {/* Mode toggle */}
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant={mode === "mock" ? "default" : "outline"}
          onClick={() => onModeChange("mock")}
        >
          Mock
        </Button>
        <Button
          size="sm"
          variant={mode === "live" ? "default" : "outline"}
          onClick={() => onModeChange("live")}
        >
          Live
        </Button>
      </div>

      {/* UUID input — only relevant in live mode */}
      {mode === "live" && (
        <div className="flex items-center gap-2">
          <Label htmlFor="uuid-input" className="text-xs text-muted-foreground">
            UUID
          </Label>
          <Input
            id="uuid-input"
            className="h-7 w-20 text-xs"
            value={uuidInput}
            onChange={(e) => setUuidInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleUuidSubmit()}
          />
          <Button size="sm" variant="outline" onClick={handleUuidSubmit}>
            Set
          </Button>
        </div>
      )}
    </div>
  );
}
