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
            style.color = "var(--color-blue)";
            style.fontWeight = 600;
            break;
        case "out":
            style.color = "#d4d4d4";
            break;
        case "err":
            style.color = "var(--color-red)";
            break;
        case "exit":
            style.color = entry.text.startsWith("✓") ? "var(--color-green)" : "var(--color-red)";
            style.fontStyle = "italic";
            break;
        case "info":
            style.color = "var(--color-purple)";
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
                background: "#1e1e1e",
                borderTop: "1px solid var(--color-border)",
            }}
        >
            {/* ── Header bar ── */}
            <div
                className="flex items-center gap-2 px-4 py-2 shrink-0 element-border"
                style={{
                    background: "var(--color-surface)",
                    borderBottom: "1px solid var(--color-border)",
                }}
            >
                <Terminal
                    size={14}
                    style={{ color: "var(--color-muted)" }}
                />
                <span
                    className="text-sm font-semibold"
                    style={{ color: "var(--color-text)" }}
                >
                    Terminal Console
                </span>
                {terminalRunning && (
                    <span
                        className="text-xs px-2 py-0.5 rounded-md animate-pulse font-medium"
                        style={{ background: "rgba(38,162,105,0.2)", color: "var(--color-green)" }}
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
                                    ? { background: "var(--color-border)", color: "var(--color-muted)", opacity: 0.5 }
                                    : { background: "var(--color-blue)", color: "white", border: "1px solid rgba(0,0,0,0.2)", cursor: "pointer" }
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
                    style={{ background: "var(--color-bg)", borderBottom: "1px solid var(--color-border)" }}
                >
                    {terminalStagedCommands.map((item, i) => (
                        <div
                            key={i}
                            className="flex items-center gap-2"
                        >
                            <span
                                className="text-xs font-mono flex-1"
                                style={{ color: "var(--color-subtext)" }}
                            >
                                {MANAGER_EMOJIS[item.manager] ?? "📦"} remove <strong>{item.displayName}</strong>
                                <span style={{ color: "#6e6e8e" }}> ({item.manager})</span>
                            </span>
                            <button
                                onClick={() => unstageCommand(i)}
                                className="p-2 rounded-lg transition-colors hover:bg-[var(--color-red)]/20"
                                style={{ color: "var(--color-muted)" }}
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
                style={{ scrollbarWidth: "thin", scrollbarColor: "var(--color-border) #1e1e1e" }}
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
                        style={{ color: "var(--color-blue)" }}
                    >
                        ▌
                    </div>
                )}
            </div>
        </div>
    );
}
