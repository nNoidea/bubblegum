import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Search, RefreshCw, RotateCw, Zap, ArrowDownAZ, ArrowUpAZ, HardDrive } from "lucide-react";
import { PackageCard } from "../components/PackageCard";
import { TerminalPanel } from "../components/TerminalPanel";
import { ProgressBar, ManagerPill } from "../components/StatusWidgets";
import { MANAGER_COLORS, MANAGER_EMOJIS } from "../constants";
import { useAppStore, ALL_MANAGER_IDS } from "../store";

const PAGE_SIZE = 48;

// ─── Source grouping ──────────────────────────────────────────────────────────
const SOURCE_GROUP_LABELS: Record<string, string> = {
    official: "Official",
    community: "Community",
    flathub: "Flathub",
    copr: "COPR",
    rpmfusion: "RPM Fusion",
    "third-party": "Third-party",
    proprietary: "Proprietary",
    restricted: "Restricted",
    local: "Local",
    other: "Other",
};

const SOURCE_GROUP_ORDER = ["official", "flathub", "community", "copr", "rpmfusion", "third-party", "proprietary", "restricted", "local"];

function getSourceGroup(source?: string | null): string {
    if (!source) return "other";
    if (source === "official") return "official";
    if (source === "community") return "community";
    if (source === "proprietary") return "proprietary";
    if (source === "restricted") return "restricted";
    if (source === "locally-installed") return "local";
    if (source.startsWith("copr:")) return "copr";
    if (source.startsWith("rpmfusion")) return "rpmfusion";
    if (source.startsWith("third-party:")) return "third-party";
    if (source.toLowerCase().includes("flathub")) return "flathub";
    return "other";
}

// ─── Skeleton card ────────────────────────────────────────────────────────────
function SkeletonCard() {
    return (
        <div className="flat-card rounded-xl p-4 h-28 skeleton-pulse">
            <div
                className="h-3 rounded w-2/3 mb-3"
                style={{ background: "var(--color-border)" }}
            />
            <div
                className="h-2 rounded w-1/3 mb-2"
                style={{ background: "var(--color-border)" }}
            />
            <div
                className="h-2 rounded w-1/2"
                style={{ background: "var(--color-border)" }}
            />
        </div>
    );
}

// ─── ASCII spinner ───────────────────────────────────────────────────────────
const ASCII_FRAMES = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
function AsciiSpinner({ style }: { style?: React.CSSProperties }) {
    const [frame, setFrame] = useState(0);
    useEffect(() => {
        const id = setInterval(() => setFrame((f) => (f + 1) % ASCII_FRAMES.length), 110);
        return () => clearInterval(id);
    }, []);
    return <span style={{ fontFamily: "monospace", lineHeight: 1, ...style }}>{ASCII_FRAMES[frame]}</span>;
}

// ─── Manager tab button ───────────────────────────────────────────────────────
function ManagerTab({
    label,
    emoji,
    color,
    count,
    active,
    loading,
    onClick,
    onSync,
}: {
    label: string;
    emoji?: string;
    color?: string;
    count?: number;
    active: boolean;
    loading?: boolean;
    onClick: () => void;
    onSync?: () => void;
}) {
    const [hovered, setHovered] = useState(false);
    const [syncSpin, setSyncSpin] = useState(false);
    const accentColor = color ?? "#9141ac";

    function handleSync(e: React.MouseEvent) {
        e.stopPropagation();
        setSyncSpin(true);
        setTimeout(() => setSyncSpin(false), 900);
        onSync?.();
    }

    const containerStyle: React.CSSProperties = active
        ? {
              background: `${accentColor}1a`,
              border: `1px solid ${accentColor}40`,
              color: accentColor,
          }
        : {
              background: hovered ? "var(--color-card)" : "transparent",
              border: `1px solid ${hovered ? "var(--color-border)" : "transparent"}`,
              color: hovered ? "var(--color-text)" : "var(--color-muted)",
          };

    return (
        <div
            className="flex items-stretch shrink-0 rounded-lg overflow-hidden transition-all duration-150"
            style={containerStyle}
            onMouseEnter={() => setHovered(true)}
            onMouseLeave={() => setHovered(false)}
        >
            {/* Main select button */}
            <button
                onClick={onClick}
                className="flex items-center gap-2 pl-4 py-2.5 pr-3 text-sm font-semibold whitespace-nowrap bg-transparent border-0 outline-none cursor-pointer"
                style={{ color: "inherit" }}
            >
                {emoji && <span className="shrink-0 text-base leading-none">{emoji}</span>}
                <span
                    className="truncate"
                    style={{ maxWidth: 80 }}
                >
                    {label}
                </span>
                {loading ? (
                    <AsciiSpinner style={{ color: accentColor, fontSize: "0.9em" }} />
                ) : (
                    count !== undefined && (
                        <span
                            className="px-1.5 py-0.5 rounded text-xs font-mono shrink-0"
                            style={{
                                background: active ? `${accentColor}20` : "var(--color-border)",
                                color: active ? accentColor : "var(--color-subtext)",
                            }}
                        >
                            {count > 9999 ? `${Math.round(count / 1000)}k` : count.toLocaleString()}
                        </span>
                    )
                )}
            </button>

            {/* Sync section — thin divider + icon, always present, opacity-driven */}
            {onSync && !loading && (
                <>
                    <div
                        className="my-2"
                        style={{
                            width: 1,
                            background: "currentColor",
                            opacity: hovered ? 0.2 : 0.08,
                            transition: "opacity 0.2s",
                        }}
                    />
                    <button
                        onClick={handleSync}
                        title={`Refresh ${label}`}
                        className="flex items-center justify-center px-3 py-2.5 bg-transparent border-0 outline-none cursor-pointer transition-all duration-200"
                        style={{
                            opacity: hovered ? 0.8 : 0.2,
                            color: "inherit",
                        }}
                    >
                        <RotateCw
                            size={14}
                            className={syncSpin ? "animate-spin" : ""}
                        />
                    </button>
                </>
            )}
        </div>
    );
}

// ─── Source tab button ────────────────────────────────────────────────────────
const SOURCE_COLORS: Record<string, string> = {
    official: "#26a269",
    community: "#9141ac",
    flathub: "#2190a4",
    copr: "#3584e4",
    rpmfusion: "#c7561e",
    "third-party": "#a86e00",
    proprietary: "#c4402e",
    restricted: "#e01b24",
    local: "#6e6e8e",
    other: "#6e6e8e",
};

function SourceTab({ group, count, active, onClick }: { group: string; count: number; active: boolean; onClick: () => void }) {
    const label = SOURCE_GROUP_LABELS[group] ?? group;
    const color = SOURCE_COLORS[group] ?? "#6e6e8e";
    return (
        <button
            onClick={onClick}
            className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-150 cursor-pointer"
            style={active ? { background: `${color}1a`, border: `1px solid ${color}40`, color } : { background: "transparent", border: "1px solid transparent", color: "var(--color-muted)" }}
        >
            <span>{label}</span>
            <span
                className="px-1.5 py-0.5 rounded text-xs"
                style={{
                    background: active ? `${color}1a` : "var(--color-border)",
                    color: active ? color : "var(--color-subtext)",
                }}
            >
                {count > 9999 ? `${Math.round(count / 1000)}k` : count.toLocaleString()}
            </span>
        </button>
    );
}

// ─── Main Overview page ───────────────────────────────────────────────────────
export function Overview() {
    const navigate = useNavigate();

    const {
        packages,
        loading,
        loadingManagers,
        finishedManagers,
        selectedManager,
        setSelectedManager,
        managers,
        userMode,
        setUserMode,
        searchQuery,
        setSearchQuery,
        updates,
        streamPackages,
        refreshSingleManager,
        packageCache,
    } = useAppStore();

    // Local state: source filter resets when manager/mode changes
    const [selectedSource, setSelectedSource] = useState("all");
    const [page, setPage] = useState(1);
    const [sortBy, setSortBy] = useState<"name-asc" | "name-desc" | "size-desc" | "manager">("name-asc");

    // Ref for keyboard shortcut focus
    const searchInputRef = useRef<HTMLInputElement>(null);

    // Ref for infinite scroll sentinel
    const sentinelRef = useRef<HTMLDivElement>(null);

    // ─── Keyboard shortcuts ───────────────────────────────────────────────────
    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            const tag = (document.activeElement as HTMLElement)?.tagName;
            const isInput = tag === "INPUT" || tag === "TEXTAREA";
            // "/" or Ctrl+F → focus search
            if (!isInput && e.key === "/" && !e.ctrlKey && !e.metaKey) {
                e.preventDefault();
                searchInputRef.current?.focus();
            }
            if (e.ctrlKey && e.key === "f") {
                e.preventDefault();
                searchInputRef.current?.focus();
            }
            // Escape → clear search or blur
            if (e.key === "Escape") {
                if (searchQuery) setSearchQuery("");
                else (document.activeElement as HTMLElement)?.blur();
            }
            // Ctrl+R → force refresh packages
            if ((e.ctrlKey || e.metaKey) && e.key === "r") {
                e.preventDefault();
                streamPackages(true);
            }
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, [searchQuery]);

    // Re-stream whenever the active manager or user-mode changes (cache-aware)
    useEffect(() => {
        setPage(1);
        setSelectedSource("all");
        streamPackages();
    }, [selectedManager, userMode]);

    // ─── Infinite scroll ──────────────────────────────────────────────────────
    // The sentinel div only renders when hasMore && !loading, so if the observer
    // fires, we can safely page forward without rechecking those flags here.
    // Re-attach when the sentinel appears/disappears (driven by hasMore/loading).
    useEffect(() => {
        const el = sentinelRef.current;
        if (!el) return;
        const obs = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting) {
                    setPage((p) => p + 1);
                }
            },
            { rootMargin: "300px" },
        );
        obs.observe(el);
        return () => obs.disconnect();
    });

    // ─── Derived values ───────────────────────────────────────────────────────
    const availableManagers = managers.filter((m) => m.available);
    const cacheMode = userMode ? "u" : "s";
    // Helper: is a specific manager's data already in cache?
    function isManagerCached(id: string) {
        return `${id}_${cacheMode}` in packageCache;
    }

    // Package counts per manager — prefer cache so counts stay visible even
    // when switching views (packages state may only hold the current tab's data)
    const managerCounts = useMemo(() => {
        const counts: Record<string, number> = {};
        // Authoritative: read from cache for each manager
        availableManagers.forEach((m) => {
            const cached = packageCache[`${m.id}_${cacheMode}`];
            if (cached) counts[m.id] = cached.length;
        });
        // Fill in from live packages for managers still streaming (not yet cached)
        packages.forEach((p) => {
            if (!(p.manager in counts)) {
                counts[p.manager] = (counts[p.manager] ?? 0) + 1;
            }
        });
        return counts;
    }, [packages, packageCache, cacheMode, availableManagers]);

    // Total count across all managers (stable across tab switches)
    const allCount = useMemo(() => {
        const partialSum = availableManagers.reduce((sum, m) => {
            return sum + (packageCache[`${m.id}_${cacheMode}`]?.length ?? 0);
        }, 0);
        return partialSum > 0 ? partialSum : packages.length;
    }, [packages, packageCache, cacheMode, availableManagers]);

    // Progress tracking for the current stream
    const expectedIds = selectedManager === "all" ? ALL_MANAGER_IDS : [selectedManager];
    const doneCount = finishedManagers.filter((id) => expectedIds.includes(id)).length;
    const totalManagers = expectedIds.length;

    // Source groups — only show source tabs per manager (not when "all" is selected)
    const sourceGroups = useMemo(() => {
        if (selectedManager === "all") return new Map<string, number>();
        const map = new Map<string, number>();
        packages.forEach((p) => {
            const g = getSourceGroup(p.source);
            map.set(g, (map.get(g) ?? 0) + 1);
        });
        return map;
    }, [packages, selectedManager]);

    // Ordered source group list for tabs
    const sortedGroups = useMemo(() => {
        const withPackages = new Set(sourceGroups.keys());
        withPackages.delete("other");
        const ordered = SOURCE_GROUP_ORDER.filter((g) => withPackages.has(g));
        const extra = [...withPackages].filter((g) => !SOURCE_GROUP_ORDER.includes(g));
        return [...ordered, ...extra];
    }, [sourceGroups]);

    // Source-filtered packages
    const filteredBySource = useMemo(() => {
        if (selectedSource === "all") return packages;
        return packages.filter((p) => getSourceGroup(p.source) === selectedSource);
    }, [packages, selectedSource]);

    // Search-filtered packages
    const filteredBySearch = useMemo(() => {
        const q = searchQuery.trim().toLowerCase();
        if (!q) return filteredBySource;
        return filteredBySource.filter((p) => p.name.toLowerCase().includes(q) || (p.description ?? "").toLowerCase().includes(q));
    }, [filteredBySource, searchQuery]);

    // Sorted packages
    const sortedPackages = useMemo(() => {
        const arr = [...filteredBySearch];
        switch (sortBy) {
            case "name-asc":
                return arr.sort((a, b) => a.name.localeCompare(b.name));
            case "name-desc":
                return arr.sort((a, b) => b.name.localeCompare(a.name));
            case "size-desc":
                return arr.sort((a, b) => (b.size_bytes ?? 0) - (a.size_bytes ?? 0));
            case "manager":
                return arr.sort((a, b) => a.manager.localeCompare(b.manager) || a.name.localeCompare(b.name));
            default:
                return arr;
        }
    }, [filteredBySearch, sortBy]);

    // Total displayed size
    const totalSize = useMemo(() => {
        const bytes = filteredBySearch.reduce((sum, p) => sum + (p.size_bytes ?? 0), 0);
        if (!bytes) return null;
        if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
        return `${Math.round(bytes / 1024 / 1024)} MB`;
    }, [filteredBySearch]);

    // Paginated slice
    const paged = useMemo(() => sortedPackages.slice(0, page * PAGE_SIZE), [sortedPackages, page]);
    const hasMore = paged.length < sortedPackages.length;

    const updateCount = updates.length;

    return (
        <div className="flex flex-col h-screen overflow-hidden">
            {/* ════ TOP TOOLBAR ════ */}
            <header
                className="shrink-0 flex items-center gap-4 px-6 py-3"
                style={{ background: "var(--color-surface)", borderBottom: "1px solid var(--color-border)" }}
            >
                {/* Left: brand + User/System toggle stacked */}
                <div className="flex flex-col gap-2 shrink-0">
                    <span
                        className="text-2xl font-black select-none leading-tight tracking-tight"
                        style={{ color: "var(--color-text)" }}
                    >
                        🛇 Bubblegum
                    </span>

                    <div
                        className="flex rounded-lg overflow-hidden self-start"
                        style={{ background: "var(--color-card)", border: "1px solid var(--color-border)", padding: "2px" }}
                    >
                        {(["User", "System"] as const).map((mode) => {
                            const isActive = (mode === "User") === userMode;
                            return (
                                <button
                                    key={mode}
                                    onClick={() => setUserMode(mode === "User")}
                                    className="px-4 py-1.5 text-sm font-medium transition-all select-none rounded-lg"
                                    style={isActive ? { background: "var(--color-purple)", color: "white" } : { color: "var(--color-muted)" }}
                                >
                                    {mode === "User" ? "👤 User" : "⚙️ System"}
                                </button>
                            );
                        })}
                    </div>
                </div>

                {/* Center: big search bar */}
                <div className="flex-1 relative">
                    <Search
                        size={15}
                        className="absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none"
                        style={{ color: "var(--color-muted)" }}
                    />
                    <input
                        type="text"
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        placeholder={packages.length > 0 ? `Search ${packages.length.toLocaleString()} packages… (/)` : "Search packages… (/)"}
                        ref={searchInputRef}
                        className="w-full rounded-2xl py-2.5 pl-10 pr-4 text-sm outline-none transition-all"
                        style={{
                            background: "var(--color-card)",
                            border: "1.5px solid var(--color-border)",
                            color: "var(--color-text)",
                        }}
                        onFocus={(e) => (e.currentTarget.style.borderColor = "var(--color-purple)")}
                        onBlur={(e) => (e.currentTarget.style.borderColor = "var(--color-border)")}
                    />
                </div>

                {/* Right: Updates button */}
                <button
                    onClick={() => navigate("/updates")}
                    className="relative flex items-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium transition-colors shrink-0"
                    style={{ background: "var(--color-purple)", color: "white" }}
                >
                    <RefreshCw size={14} />
                    <span>Updates</span>
                    {updateCount > 0 && (
                        <span
                            className="absolute -top-1.5 -right-1.5 min-w-5 h-5 px-1 rounded-full text-xs font-bold flex items-center justify-center"
                            style={{ background: "var(--color-red)", color: "white" }}
                        >
                            {updateCount > 99 ? "99+" : updateCount}
                        </span>
                    )}
                </button>
            </header>

            {/* ════ MANAGER TABS ════ */}
            <div
                className="shrink-0 flex items-center gap-1.5 px-4 py-3 overflow-x-auto scrollbar-none"
                style={{ background: "var(--color-surface)", borderBottom: "1px solid var(--color-border)" }}
            >
                {/* "All" tab */}
                <ManagerTab
                    label="All"
                    active={selectedManager === "all"}
                    count={allCount}
                    loading={loading && selectedManager === "all" && loadingManagers.length > 0}
                    onClick={() => setSelectedManager("all")}
                    onSync={() => streamPackages(true)}
                />

                {/* Divider */}
                <div
                    className="w-px h-5 shrink-0"
                    style={{ background: "var(--color-border)" }}
                />

                {availableManagers.map((m) => (
                    <ManagerTab
                        key={m.id}
                        label={m.name}
                        emoji={MANAGER_EMOJIS[m.id] ?? "📦"}
                        color={MANAGER_COLORS[m.id]}
                        active={selectedManager === m.id}
                        count={managerCounts[m.id]}
                        loading={loading && loadingManagers.includes(m.id)}
                        onClick={() => setSelectedManager(m.id)}
                        onSync={() => refreshSingleManager(m.id)}
                    />
                ))}

                {/* In-flight manager pills when streaming "all" */}
                {loading && selectedManager === "all" && loadingManagers.length > 0 && (
                    <div
                        className="flex items-center gap-1 ml-2 pl-2"
                        style={{ borderLeft: "1px solid var(--color-border)" }}
                    >
                        {loadingManagers.map((id) => (
                            <ManagerPill
                                key={id}
                                id={id}
                                loading={true}
                            />
                        ))}
                    </div>
                )}
            </div>

            {/* ════ SOURCE TABS ════ (visible once data arrives) */}
            {sortedGroups.length > 1 && (
                <div
                    className="shrink-0 flex items-center gap-1 px-4 py-2 overflow-x-auto scrollbar-none"
                    style={{ background: "var(--color-surface)", borderBottom: "1px solid var(--color-border)" }}
                >
                    {/* "All Sources" */}
                    <SourceTab
                        group="all"
                        count={packages.length}
                        active={selectedSource === "all"}
                        onClick={() => setSelectedSource("all")}
                    />

                    {sortedGroups.map((g) => (
                        <SourceTab
                            key={g}
                            group={g}
                            count={sourceGroups.get(g) ?? 0}
                            active={selectedSource === g}
                            onClick={() => setSelectedSource(g)}
                        />
                    ))}
                </div>
            )}

            {/* ════ PROGRESS BAR ════ */}
            {loading && (
                <ProgressBar
                    total={totalManagers}
                    done={doneCount}
                />
            )}

            {/* ════ PACKAGE GRID ════ */}
            <div className="flex-1 overflow-y-scroll px-6 py-4">
                {/* Status line + Sort controls */}
                {(loading || filteredBySearch.length > 0) && (
                    <div className="mb-3 flex items-center gap-2 flex-wrap">
                        <p
                            className="text-xs flex items-center gap-2"
                            style={{ color: "var(--color-muted)" }}
                        >
                            {loading && (
                                <span
                                    className="flex items-center gap-1.5 font-bold"
                                    style={{ color: "var(--color-purple)" }}
                                >
                                    <AsciiSpinner />
                                    <span>Loading…</span>
                                </span>
                            )}
                            {!loading && isManagerCached(selectedManager) && (
                                <span
                                    className="flex items-center gap-1"
                                    style={{ color: "#00DDB8" }}
                                >
                                    <Zap size={10} />
                                    <span>cached</span>
                                </span>
                            )}
                            {packages.length > 0 &&
                                (filteredBySearch.length !== packages.length
                                    ? `${filteredBySearch.length.toLocaleString()} / ${packages.length.toLocaleString()} packages`
                                    : `${packages.length.toLocaleString()} packages`)}
                            {totalSize && (
                                <span
                                    className="flex items-center gap-1"
                                    style={{ color: "var(--color-muted)" }}
                                >
                                    <HardDrive size={10} />
                                    <span>{totalSize}</span>
                                </span>
                            )}
                        </p>

                        {/* Sort controls */}
                        <div className="flex items-center gap-1 ml-auto">
                            {(
                                [
                                    { key: "name-asc", label: "A→Z", icon: <ArrowDownAZ size={11} /> },
                                    { key: "name-desc", label: "Z→A", icon: <ArrowUpAZ size={11} /> },
                                    { key: "size-desc", label: "Largest", icon: <HardDrive size={11} /> },
                                    { key: "manager", label: "Manager", icon: null },
                                ] as const
                            ).map(({ key, label, icon }) => (
                                <button
                                    key={key}
                                    onClick={() => setSortBy(key)}
                                    title={`Sort by ${label}`}
                                    className="flex items-center gap-1 px-2 py-1 rounded-lg text-xs font-medium transition-all"
                                    style={{
                                        background: sortBy === key ? "var(--color-card)" : "transparent",
                                        border: `1px solid ${sortBy === key ? "var(--color-border)" : "transparent"}`,
                                        color: sortBy === key ? "var(--color-text)" : "var(--color-muted)",
                                        cursor: "pointer",
                                    }}
                                >
                                    {icon}
                                    {label}
                                </button>
                            ))}

                        </div>
                    </div>
                )}

                {/* Skeleton grid while nothing loaded yet */}
                {loading && packages.length === 0 && (
                    <div
                        className="grid gap-3"
                        style={{ gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))" }}
                    >
                        {Array.from({ length: 24 }).map((_, i) => (
                            <SkeletonCard key={i} />
                        ))}
                    </div>
                )}

                {/* Empty state */}
                {!loading && filteredBySearch.length === 0 && packages.length >= 0 && (
                    <div
                        className="flex flex-col items-center justify-center h-64 gap-4"
                        style={{ color: "var(--color-muted)" }}
                    >
                        <div className="text-6xl select-none animate-bounce">🫧</div>
                        <p className="text-sm font-medium">{searchQuery.trim() ? `No packages match "${searchQuery}"` : "No packages found."}</p>
                        {searchQuery.trim() && (
                            <button
                                onClick={() => setSearchQuery("")}
                                className="text-xs px-4 py-2 rounded-lg font-medium transition-colors"
                                style={{ background: "var(--color-purple)", color: "white" }}
                            >
                                Clear search
                            </button>
                        )}
                    </div>
                )}

                {/* Cards */}
                {paged.length > 0 && (
                    <div
                        className="grid gap-3"
                        style={{ gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))" }}
                    >
                        {paged.map((pkg) => (
                            <PackageCard
                                key={pkg.id}
                                pkg={pkg}
                            />
                        ))}
                        {loading && Array.from({ length: 6 }).map((_, i) => <SkeletonCard key={`skel-tail-${i}`} />)}
                    </div>
                )}

                {/* Infinite scroll sentinel */}
                {hasMore && !loading && (
                    <div
                        ref={sentinelRef}
                        className="h-8"
                    />
                )}
                {hasMore && loading && (
                    <div
                        className="mt-4 text-center text-xs"
                        style={{ color: "var(--color-muted)" }}
                    >
                        Loading more…
                    </div>
                )}
            </div>

            {/* ════ TERMINAL PANEL ════ */}
            <TerminalPanel />
        </div>
    );
}
