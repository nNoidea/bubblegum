import { useEffect, useRef } from "react";
import { Play, Trash2, X, Terminal } from "lucide-react";
import { useAppStore, type TerminalEntry } from "../store";
import { MANAGER_EMOJIS } from "../constants";

// ─── Line rendering ───────────────────────────────────────────────────────────

function TerminalLine({ entry }: { entry: TerminalEntry }) {
    const style: React.CSSProperties = {};
    let prefix = "";

    switch (entry.kind) {
        case "cmd":
            style.color = "#7dd3fc"; // sky-300
            style.fontWeight = 600;
            break;
        case "out":
            style.color = "#d4d4d4";
            break;
        case "err":
            style.color = "#f87171"; // red-400
            break;
        case "exit":
            style.color = entry.text.startsWith("✓") ? "#4ade80" : "#f87171";
            style.fontStyle = "italic";
            break;
        case "info":
            style.color = "#a78bfa"; // purple-400
            prefix = "# ";
            break;
    }

    return (
        <div
            className="leading-5 whitespace-pre-wrap break-all"
            style={style}
        >
            {prefix}
            {entry.text}
        </div>
    );
}

// ─── Main terminal panel ─────────────────────────────────────────────────────

export function TerminalPanel() {
    const { terminalStagedCommands, terminalOutput, terminalRunning, unstageCommand, clearTerminalOutput, executeTerminalCommands } = useAppStore();

    const outputRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to bottom when output grows
    useEffect(() => {
        const el = outputRef.current;
        if (el) el.scrollTop = el.scrollHeight;
    }, [terminalOutput]);

    const hasAnything = terminalStagedCommands.length > 0 || terminalOutput.length > 0;

    if (!hasAnything && !terminalRunning) return null;

    return (
        <div
            className="shrink-0 flex flex-col"
            style={{
                height: 280,
                background: "#0d0d14",
                borderTop: "1px solid #1e1e2e",
            }}
        >
            {/* ── Header bar ── */}
            <div
                className="flex items-center gap-2 px-4 py-2 shrink-0"
                style={{
                    background: "#12121f",
                    borderBottom: "1px solid #1e1e2e",
                }}
            >
                <Terminal
                    size={13}
                    style={{ color: "#7dd3fc" }}
                />
                <span
                    className="text-xs font-semibold tracking-widest uppercase"
                    style={{ color: "#7dd3fc" }}
                >
                    Terminal
                </span>
                {terminalRunning && (
                    <span
                        className="text-xs px-2 py-0.5 rounded animate-pulse"
                        style={{ background: "#4ade8020", color: "#4ade80", border: "1px solid #4ade8040" }}
                    >
                        running…
                    </span>
                )}
                <div className="ml-auto flex items-center gap-2">
                    {terminalOutput.length > 0 && !terminalRunning && (
                        <button
                            onClick={clearTerminalOutput}
                            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm transition-colors hover:bg-white/5"
                            style={{ color: "#6e6e8e" }}
                            title="Clear output"
                        >
                            <Trash2 size={14} />
                            Clear
                        </button>
                    )}
                    {/* Execute button */}
                    {terminalStagedCommands.length > 0 && (
                        <button
                            onClick={executeTerminalCommands}
                            disabled={terminalRunning}
                            className="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-semibold transition-all"
                            style={
                                terminalRunning
                                    ? { background: "#1a2e1a", color: "#6e6e8e", opacity: 0.5 }
                                    : { background: "#166534", color: "#4ade80", border: "1px solid #4ade8040", cursor: "pointer" }
                            }
                        >
                            <Play size={14} />
                            Execute ({terminalStagedCommands.length})
                        </button>
                    )}
                </div>
            </div>

            {/* ── Staged commands queue ── */}
            {terminalStagedCommands.length > 0 && (
                <div
                    className="shrink-0 flex flex-col gap-1 px-4 py-2"
                    style={{ background: "#0f0f1a", borderBottom: "1px solid #1e1e2e" }}
                >
                    {terminalStagedCommands.map((item, i) => (
                        <div
                            key={i}
                            className="flex items-center gap-2"
                        >
                            <span
                                className="text-xs font-mono flex-1"
                                style={{ color: "#a78bfa" }}
                            >
                                {MANAGER_EMOJIS[item.manager] ?? "📦"} remove <strong>{item.displayName}</strong>
                                <span style={{ color: "#6e6e8e" }}> ({item.manager})</span>
                            </span>
                            <button
                                onClick={() => unstageCommand(i)}
                                className="p-2 rounded-lg transition-colors hover:bg-red-500/20"
                                style={{ color: "#6e6e8e" }}
                                title="Remove command"
                            >
                                <X size={14} />
                            </button>
                        </div>
                    ))}
                </div>
            )}

            {/* ── Output area ── */}
            <div
                ref={outputRef}
                className="flex-1 overflow-y-auto px-4 py-2 font-mono text-xs"
                style={{ scrollbarWidth: "thin", scrollbarColor: "#2a2a3f #0d0d14" }}
            >
                {terminalOutput.map((entry, i) => (
                    <TerminalLine
                        key={i}
                        entry={entry}
                    />
                ))}
                {terminalRunning && (
                    <div
                        className="animate-pulse mt-1"
                        style={{ color: "#7dd3fc" }}
                    >
                        ▌
                    </div>
                )}
            </div>
        </div>
    );
}
