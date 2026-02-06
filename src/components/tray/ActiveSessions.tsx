import { motion } from "framer-motion";
import { Circle, MonitorOff } from "lucide-react";
import { formatTokens, formatTimeAgo } from "../../lib/format";
import type { Session, SourceType } from "../../lib/types";

interface Props {
  sessions: Session[];
  sourceType?: SourceType;
}

export function ActiveSessions({ sessions, sourceType }: Props) {
  const isEmpty = sourceType === "api" || sessions.length === 0;

  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-xs font-medium text-text-secondary">Active Sessions</span>
        {!isEmpty && (
          <span className="text-[10px] text-muted tabular-nums">{sessions.length}</span>
        )}
      </div>

      {isEmpty ? (
        <div className="flex-1 flex items-center justify-center gap-2 px-3 h-12 rounded-lg bg-card border border-border border-dashed">
          <MonitorOff size={12} className="text-muted opacity-40 shrink-0" />
          <p className="text-[10px] text-muted">
            {sourceType === "api" ? "Not available via API" : "No active sessions"}
          </p>
        </div>
      ) : (
      <div className={`space-y-1 overflow-y-auto ${sessions.length === 2 ? 'max-h-[76px]' : 'max-h-[150px]'}`}>
        {sessions.map((session, i) => (
          <motion.div
            key={session.id}
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.03 * i }}
            className="flex items-center gap-2.5 px-3 py-2 rounded-lg bg-card border border-border hover:border-border-light transition-colors"
          >
            <Circle
              size={6}
              fill={session.isActive ? "#22c55e" : "#6b6b80"}
              className={session.isActive ? "text-success" : "text-muted"}
            />
            <div className="flex-1 min-w-0">
              <div className="text-xs font-medium text-text truncate">
                {session.project || "Unknown"}
              </div>
              <div className="text-[10px] text-muted">
                {session.model.split("-").slice(-2).join("-")} · {formatTokens(session.tokensUsed)}
              </div>
            </div>
            <span className="text-[10px] text-muted whitespace-nowrap">
              {formatTimeAgo(session.lastActive)}
            </span>
          </motion.div>
        ))}
      </div>
      )}
    </div>
  );
}
