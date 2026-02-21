import { MANAGER_COLORS } from "../constants";

// ─── Flat progress bar ────────────────────────────────────────────────────────
export function ProgressBar({ total, done }: { total: number; done: number }) {
    const pct = total === 0 ? 100 : Math.round((done / total) * 100);
    return (
        <div
            className="w-full overflow-hidden shrink-0"
            style={{ height: 3, background: "var(--color-border)" }}
        >
            <div
                className="h-full transition-all duration-500"
                style={{ width: `${pct}%`, background: "var(--color-purple)" }}
            />
        </div>
    );
}

// ─── Per-manager loading pill ─────────────────────────────────────────────────
export function ManagerPill({ id, loading }: { id: string; loading: boolean }) {
    const color = MANAGER_COLORS[id] ?? "#6e6e8e";
    return (
        <span
            className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-semibold transition-all"
            style={{
                background: `${color}1a`,
                border: `1px solid ${color}40`,
                color,
                opacity: loading ? 1 : 0.45,
            }}
        >
            <span>{id}</span>
            {loading && (
                <span
                    className="w-1.5 h-1.5 rounded-full animate-pulse"
                    style={{ background: color }}
                />
            )}
        </span>
    );
}
