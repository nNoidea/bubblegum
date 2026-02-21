import { useMemo } from "react";
import { Search as SearchIcon } from "lucide-react";
import { PackageCard } from "../components/PackageCard";
import { useAppStore } from "../store";

export function Search() {
    const { packages, searchQuery, setSearchQuery, loading } = useAppStore();

    const results = useMemo(() => {
        if (!searchQuery.trim()) return [];
        const q = searchQuery.toLowerCase();
        return packages.filter((p) => p.name.toLowerCase().includes(q) || p.description?.toLowerCase().includes(q) || p.id.toLowerCase().includes(q));
    }, [packages, searchQuery]);

    return (
        <div className="p-6">
            <h1
                className="text-2xl font-bold mb-4"
                style={{ color: "var(--color-text)" }}
            >
                Search
            </h1>

            {/* Search input */}
            <div
                className="flex items-center gap-3 px-4 py-3 rounded-2xl mb-6"
                style={{
                    background: "var(--color-card)",
                    border: "1px solid var(--color-border)",
                }}
            >
                <SearchIcon
                    size={18}
                    style={{ color: "var(--color-muted)" }}
                />
                <input
                    autoFocus
                    className="flex-1 bg-transparent outline-none text-sm"
                    style={{ color: "var(--color-text)" }}
                    placeholder="Search installed packages…"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                />
                {searchQuery && (
                    <button
                        className="text-sm px-3 py-1.5 rounded-lg hover:bg-white/10 transition-colors"
                        style={{ color: "var(--color-muted)" }}
                        onClick={() => setSearchQuery("")}
                    >
                        Clear
                    </button>
                )}
            </div>

            {/* Results */}
            {searchQuery.trim() === "" ? (
                <div
                    className="flex flex-col items-center justify-center h-48 gap-3"
                    style={{ color: "var(--color-muted)" }}
                >
                    <SearchIcon
                        size={40}
                        className="opacity-20"
                    />
                    <p className="text-sm">Start typing to search installed packages</p>
                </div>
            ) : results.length === 0 ? (
                <div
                    className="text-center py-16 text-sm"
                    style={{ color: "var(--color-muted)" }}
                >
                    {loading ? "Loading packages…" : `No results for "${searchQuery}"`}
                </div>
            ) : (
                <>
                    <div
                        className="mb-3 text-sm"
                        style={{ color: "var(--color-muted)" }}
                    >
                        {results.length} result{results.length !== 1 ? "s" : ""}
                    </div>
                    <div
                        className="grid gap-3"
                        style={{ gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))" }}
                    >
                        {results.slice(0, 100).map((pkg) => (
                            <PackageCard
                                key={pkg.id}
                                pkg={pkg}
                            />
                        ))}
                    </div>
                </>
            )}
        </div>
    );
}
