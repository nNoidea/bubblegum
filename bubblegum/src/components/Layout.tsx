import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { Search } from "lucide-react";
import { useAppStore } from "../store";

export function Layout() {
    const navigate = useNavigate();
    const location = useLocation();
    const { searchQuery, setSearchQuery, userMode, setUserMode, updates } = useAppStore();

    const isPackages = location.pathname === "/";
    const isUpdates = location.pathname === "/updates";
    const updateCount = updates.length;

    return (
        <div
            className="flex flex-col h-screen w-screen overflow-hidden"
            style={{ background: "var(--color-bg)" }}
        >
            {/* Global GNOME-style Header Bar */}
            <header
                className="shrink-0 flex items-center justify-center relative px-4"
                style={{
                    height: "56px",
                    background: "var(--color-surface)",
                    borderBottom: "1px solid var(--color-border)",
                }}
            >
                {/* Window controls / Brand (Left) */}
                <div className="absolute left-4 flex items-center gap-3">
                    <span
                        className="font-bold text-base tracking-wide"
                        style={{ color: "var(--color-text)" }}
                    >
                        Bubblegum
                    </span>
                    <div
                        className="flex rounded-md overflow-hidden"
                        style={{ background: "var(--color-bg)", border: "1px solid var(--color-border)", padding: "2px" }}
                    >
                        {(["User", "System"] as const).map((mode) => {
                            const isActive = (mode === "User") === userMode;
                            return (
                                <button
                                    key={mode}
                                    onClick={() => setUserMode(mode === "User")}
                                    className="px-4 py-1.5 text-sm font-semibold transition-all select-none rounded-sm"
                                    style={isActive ? { background: "var(--color-card)", color: "var(--color-text)", boxShadow: "0 1px 2px rgba(0,0,0,0.2)" } : { color: "var(--color-muted)", background: "transparent" }}
                                >
                                    {mode}
                                </button>
                            );
                        })}
                    </div>
                </div>

                {/* Navigation & Search (Center) */}
                <div className="flex items-center gap-2">
                    {/* View Switcher Controls */}
                    <div className="flex items-center mr-2 rounded-md overflow-hidden" style={{ background: "var(--color-bg)", border: "1px solid var(--color-border)", padding: "2px" }}>
                         <button
                            onClick={() => navigate("/")}
                            className="px-4 py-1.5 text-sm transition-all select-none rounded-sm"
                            style={isPackages ? { background: "var(--color-card)", color: "var(--color-text)", boxShadow: "0 1px 2px rgba(0,0,0,0.2)" } : { color: "var(--color-muted)", background: "transparent" }}
                        >
                            Explore
                        </button>
                        <button
                            onClick={() => navigate("/updates")}
                            className="relative px-4 py-1.5 text-sm transition-all select-none rounded-sm flex items-center gap-1.5"
                            style={isUpdates ? { background: "var(--color-card)", color: "var(--color-text)", boxShadow: "0 1px 2px rgba(0,0,0,0.2)" } : { color: "var(--color-muted)", background: "transparent" }}
                        >
                            Updates
                            {updateCount > 0 && (
                                <span
                                    className="min-w-4 h-4 px-1 rounded-full text-[10px] font-bold flex items-center justify-center"
                                    style={{ background: "var(--color-blue)", color: "white" }}
                                >
                                    {updateCount}
                                </span>
                            )}
                        </button>
                    </div>

                    {/* Search Bar - only prominent when in packages view */}
                    {isPackages && (
                        <div className="relative w-64">
                            <Search
                                size={14}
                                className="absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none"
                                style={{ color: "var(--color-muted)" }}
                            />
                            <input
                                type="text"
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                placeholder="Search apps ( / )"
                                className="w-full rounded-md py-1.5 pl-8 pr-3 text-sm outline-none transition-all placeholder:text-zinc-500"
                                style={{
                                    background: "var(--color-bg)",
                                    border: "1px solid var(--color-border)",
                                    color: "var(--color-text)",
                                }}
                                onFocus={(e) => (e.currentTarget.style.borderColor = "var(--color-blue)")}
                                onBlur={(e) => (e.currentTarget.style.borderColor = "var(--color-border)")}
                            />
                        </div>
                    )}
                </div>
            </header>

            <div className="flex-1 relative overflow-hidden">
                <Outlet />
            </div>
        </div>
    );
}
