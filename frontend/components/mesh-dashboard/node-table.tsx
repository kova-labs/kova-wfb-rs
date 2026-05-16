"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { MeshNode } from "@/lib/mesh-types";
import { TableIcon, Radio, AlertTriangle, XCircle } from "lucide-react";

interface NodeTableProps {
  nodes: MeshNode[];
  selectedNodeId: number | null;
  onSelectNode: (id: number | null) => void;
}

const STATUS_CONFIG = {
  alive: {
    icon: Radio,
    color: "text-emerald-400",
    bgColor: "bg-emerald-500/10",
    label: "Active",
  },
  missing: {
    icon: AlertTriangle,
    color: "text-amber-400",
    bgColor: "bg-amber-500/10",
    label: "Missing",
  },
  down: {
    icon: XCircle,
    color: "text-red-400",
    bgColor: "bg-red-500/10",
    label: "Down",
  },
};

function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const seconds = Math.floor(diff / 1000);

  if (seconds < 1) return "now";
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h`;
}

export function NodeTable({
  nodes,
  selectedNodeId,
  onSelectNode,
}: NodeTableProps) {
  return (
    <Card className="bg-card/50 backdrop-blur-sm border-border/50 h-full">
      <CardHeader className="pb-2">
        <div className="flex items-center gap-2">
          <TableIcon className="h-4 w-4 text-cyan-400" />
          <CardTitle className="text-sm font-medium">Node Registry</CardTitle>
          <Badge variant="secondary" className="ml-auto text-xs">
            {nodes.length} nodes
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="p-0">
        <div className="overflow-auto max-h-[200px]">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent border-border/50">
                <TableHead className="text-xs w-16">ID</TableHead>
                <TableHead className="text-xs w-24">Status</TableHead>
                <TableHead className="text-xs w-20">Last Seen</TableHead>
                <TableHead className="text-xs w-32">Position</TableHead>
                <TableHead className="text-xs text-right w-16">Sent</TableHead>
                <TableHead className="text-xs text-right w-16">Recv</TableHead>
                <TableHead className="text-xs text-right w-16">Total</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {nodes.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={7}
                    className="text-center text-muted-foreground py-8"
                  >
                    No nodes discovered yet...
                  </TableCell>
                </TableRow>
              ) : (
                nodes.map((node) => {
                  const config = STATUS_CONFIG[node.status];
                  const isSelected = node.id === selectedNodeId;

                  return (
                    <TableRow
                      key={node.id}
                      onClick={() => onSelectNode(node.id)}
                      className={`cursor-pointer transition-colors border-border/30 ${
                        isSelected
                          ? "bg-cyan-500/10 hover:bg-cyan-500/15"
                          : "hover:bg-muted/50"
                      }`}
                    >
                      <TableCell className="font-mono font-bold">
                        {node.id}
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1.5">
                          <config.icon
                            className={`h-3 w-3 ${config.color}`}
                          />
                          <Badge
                            variant="outline"
                            className={`${config.color} border-current/30 text-[10px] px-1.5 py-0`}
                          >
                            {config.label}
                          </Badge>
                        </div>
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground font-mono">
                        {formatRelativeTime(node.lastSeen)}
                      </TableCell>
                      <TableCell className="text-xs font-mono text-muted-foreground">
                        {node.position.x.toFixed(0)}, {node.position.y.toFixed(0)},{" "}
                        {node.position.z.toFixed(0)}
                      </TableCell>
                      <TableCell className="text-right font-mono text-cyan-400">
                        {node.sentCount}
                      </TableCell>
                      <TableCell className="text-right font-mono text-purple-400">
                        {node.receivedCount}
                      </TableCell>
                      <TableCell className="text-right font-mono font-bold">
                        {node.packetCount}
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  );
}
