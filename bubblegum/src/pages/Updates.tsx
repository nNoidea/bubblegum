import { useEffect } from "react";
import { RefreshCw, ArrowRight, CheckCircle2, ArrowLeft, Cpu } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { useAppStore, ALL_MANAGER_IDS } from "../store";
import { ManagerBadge } from "../components/ManagerBadge";
import { ProgressBar, ManagerPill } from "../components/StatusWidgets";
import { TerminalPanel } from "../components/TerminalPanel";

// ─── Skeleton update row ──────────────────────────────────────────────────────
function SkeletonRow() {
    return (
        <div
            className="flex items-center gap-3 px-4 py-3 skeleton-pulse"
            style={{ borderBottom: "1px solid var(--color-border)" }}
        >
            <div
                className="h-3 rounded w-1/3"
                style={{ background: "var(--color-border)" }}
            />
            <div className="flex-1" />
            <div
                className="h-2.5 rounded w-16"
                style={{ background: "var(--color-border)" }}
            />
            <div
                className="h-2.5 rounded w-16"
                style={{ background: "var(--color-border)" }}
            />
        </div>
    );
}

// ─── Main page ────────────────────────────────────────────────────────────────
export function Updates() {
    const navigate = useNavigate();
    const { updates, updateLoading, updatingManager, loadingUpdateManagers, finishedUpdateManagers, managers, streamUpdates, updateManager, firmwareUpdating, updateFirmware } = useAppStore();

    useEffect(() => {
        streamUpdates();
    }, []);

    // Determine total expected managers from available detected list
    const availableIds = managers.filter((m) => m.available).map((m) => m.id);
    const expectedIds = availableIds.length > 0 ? availableIds : ALL_MANAGER_IDS;
    const totalManagers = expectedIds.length;
    const doneCount = finishedUpdateManagers.length;

    // Group updates by manager
    const grouped = updates.reduce<Record<string, typeof updates>>((acc, u) => {
        (acc[u.manager] ??= []).push(u);
        return acc;
    }, {});

    const managerIds = Object.keys(grouped);

    const isStreaming = updateLoading || loadingUpdateManagers.length > 0;

    // Sequentially update all managers that have pending updates
    async function handleUpdateAll() {
        for (const mgrId of managerIds) {
            await updateManager(mgrId);
        }
        // Refresh the updates list after all done
        streamUpdates();
    }

    return (
        <div
            className="flex flex-col h-screen overflow-hidden"
            style={{ position: "relative" }}
        >
            {/* ── Top bar ─────────────────────────────────────────────────────── */}
            <header
                className="flex items-center gap-4 px-6 py-4 shrink-0"
                style={{ background: "var(--color-bg)", borderBottom: "1px solid var(--color-border)" }}
            >
                <button
                    onClick={() => navigate("/")}
                    className="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors hover:bg-white/5"
                    style={{ border: "1px solid var(--color-border)", color: "var(--color-subtext)" }}
                >
                    <ArrowLeft size={16} />
                    Back
                </button>
                <div
                    className="text-lg font-bold"
                    style={{ color: "var(--color-text)" }}
                >
                    🔄 Updates
                </div>
                {updates.length > 0 && (
                    <span
                        className="px-2 py-0.5 rounded-full text-xs font-bold"
                        style={{ background: "#EF444422", color: "#EF4444" }}
                    >
                        {updates.length}
                    </span>
                )}
                <div className="flex-1" />
                {managerIds.length > 0 && !isStreaming && !updatingManager && (
                    <button
                        onClick={handleUpdateAll}
                        disabled={!!updatingManager || isStreaming}
                        className="flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed mr-2"
                        style={{ background: "var(--color-card)", border: "1px solid var(--color-border)", color: "var(--color-text)" }}
                        title="Update all managers with pending updates"
                    >
                        <RefreshCw size={16} />
                        Update All ({updates.length})
                    </button>
                )}
                <button
                    onClick={updateFirmware}
                    disabled={firmwareUpdating || !!updatingManager}
                    className="flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed mr-2"
                    style={{ background: "var(--color-card)", border: "1px solid var(--color-border)", color: "var(--color-text)" }}
                    title="Check and install firmware updates via fwupd"
                >
                    <Cpu size={16} className={firmwareUpdating ? "animate-spin" : ""} />
                    {firmwareUpdating ? "Updating Firmware…" : "Firmware Update"}
                </button>
                <button
                    onClick={() => streamUpdates()}
                    disabled={updateLoading}
                    className="flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    style={{ background: "var(--color-purple)", color: "white" }}
                >
                    <RefreshCw
                        size={16}
                        className={updateLoading ? "animate-spin" : ""}
                    />
                    Refresh
                </button>
            </header>

            {/* ── Scrollable content ─────────────────────────────────────────── */}
            <div className="flex-1 overflow-y-scroll p-6">
                {/* ── Page header ────────────────────────────────────────────────── */}
                <div className="flex items-center gap-3 mb-4">
                    <h1
                        className="text-2xl font-bold"
                        style={{ color: "var(--color-text)" }}
                    >
                        Updates
                    </h1>
                    <button
                        className="ml-auto flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold transition-colors"
                        style={{
                            background: "var(--color-card)",
                            border: "1px solid var(--color-border)",
                            color: "var(--color-subtext)",
                        }}
                        onClick={streamUpdates}
                        disabled={isStreaming}
                    >
                        <RefreshCw
                            size={16}
                            className={isStreaming ? "animate-spin" : ""}
                        />
                        Refresh
                    </button>
                </div>

                {/* ── Streaming progress ─────────────────────────────────────────── */}
                {isStreaming && (
                    <>
                        <ProgressBar
                            done={doneCount}
                            total={totalManagers}
                        />
                        <div className="flex flex-wrap gap-2 mb-5">
                            {expectedIds.map((id) => (
                                <ManagerPill
                                    key={id}
                                    id={id}
                                    loading={loadingUpdateManagers.includes(id)}
                                />
                            ))}
                        </div>
                    </>
                )}

                {/* ── Results that have arrived so far ───────────────────────────── */}
                {managerIds.length > 0 && (
                    <div className="flex flex-col gap-6 mb-6">
                        {managerIds.map((mgrId) => {
                            const items = grouped[mgrId];
                            return (
                                <div key={mgrId}>
                                    <div className="flex items-center gap-3 mb-3">
                                        <ManagerBadge managerId={mgrId} />
                                        <span
                                            className="text-sm"
                                            style={{ color: "var(--color-muted)" }}
                                        >
                                            {items.length} update{items.length !== 1 ? "s" : ""}
                                        </span>
                                        <button
                                            className="ml-auto flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-semibold transition-colors"
                                            style={{ background: "var(--color-purple)", color: "white" }}
                                            onClick={() => updateManager(mgrId)}
                                        >
                                            <RefreshCw size={14} />
                                            Update all
                                        </button>
                                    </div>
                                    <div
                                        className="rounded-2xl overflow-hidden"
                                        style={{
                                            background: "var(--color-card)",
                                            border: "1px solid var(--color-border)",
                                        }}
                                    >
                                        {items.map((u, i) => (
                                            <div
                                                key={u.package_id}
                                                className="flex items-center gap-3 px-4 py-3"
                                                style={{
                                                    borderBottom: i < items.length - 1 ? "1px solid var(--color-border)" : undefined,
                                                }}
                                            >
                                                <span
                                                    className="font-medium text-sm flex-1 truncate"
                                                    style={{ color: "var(--color-text)" }}
                                                >
                                                    {u.name}
                                                </span>
                                                <span
                                                    className="text-xs font-mono"
                                                    style={{ color: "var(--color-muted)" }}
                                                >
                                                    {u.current_version}
                                                </span>
                                                <ArrowRight
                                                    size={12}
                                                    style={{ color: "var(--color-muted)" }}
                                                />
                                                <span
                                                    className="text-xs font-mono font-semibold"
                                                    style={{ color: "var(--color-teal)" }}
                                                >
                                                    {u.new_version}
                                                </span>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                )}

                {/* ── Skeleton rows for managers still in-flight ─────────────────── */}
                {isStreaming && loadingUpdateManagers.length > 0 && (
                    <div
                        className="rounded-2xl overflow-hidden"
                        style={{
                            background: "var(--color-card)",
                            border: "1px solid var(--color-border)",
                        }}
                    >
                        {Array.from({ length: Math.min(loadingUpdateManagers.length * 2, 8) }).map((_, i) => (
                            <SkeletonRow key={i} />
                        ))}
                    </div>
                )}

                {/* ── All done, nothing found ─────────────────────────────────────── */}
                {!isStreaming && updates.length === 0 && (
                    <div
                        className="flex flex-col items-center justify-center h-64 gap-4"
                        style={{ color: "var(--color-muted)" }}
                    >
                        <CheckCircle2
                            size={48}
                            className="opacity-40"
                            style={{ color: "var(--color-green)" }}
                        />
                        <p className="text-sm font-medium">Everything is up to date!</p>
                    </div>
                )}
            </div>
            {/* end scrollable content */}
            <TerminalPanel />
        </div>
    );
}
