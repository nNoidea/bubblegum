import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Package, ManagerInfo, Update, PackagesChunk, PackagesDone, UpdatesChunk, UpdatesDone } from "./types";

// ─── Terminal entry types ─────────────────────────────────────────────────────
export type TerminalEntryKind = "cmd" | "out" | "err" | "info" | "exit";
export interface TerminalEntry {
    kind: TerminalEntryKind;
    text: string;
}

/** A staged uninstall item — structured so we can batch by manager at execution time. */
export interface StagedUninstall {
    manager: string;
    pkgId: string;
    displayName: string;
}

// Canonical list that mirrors SUPPORTED_MANAGER_IDS in detect.rs.
// Used to pre-populate loadingManagers so the UI shows all spinners immediately.
export const ALL_MANAGER_IDS = ["apt", "dnf", "flatpak", "pacman", "snap", "nix", "cargo", "npm"];

// ─── Stream lifecycle helpers ─────────────────────────────────────────────────
// We maintain module-level listener handles (not Zustand state) so they can be
// cleaned up synchronously before starting a new stream.

let _pkgUnlistenChunk: UnlistenFn | null = null;
let _pkgUnlistenDone: UnlistenFn | null = null;

let _updUnlistenChunk: UnlistenFn | null = null;
let _updUnlistenDone: UnlistenFn | null = null;

let _termUnlistenLine: UnlistenFn | null = null;
let _termUnlistenDone: UnlistenFn | null = null;

function cleanupTerminalListeners() {
    _termUnlistenLine?.();
    _termUnlistenDone?.();
    _termUnlistenLine = null;
    _termUnlistenDone = null;
}

function cleanupPackageListeners() {
    _pkgUnlistenChunk?.();
    _pkgUnlistenDone?.();
    _pkgUnlistenChunk = null;
    _pkgUnlistenDone = null;
}

function cleanupUpdateListeners() {
    _updUnlistenChunk?.();
    _updUnlistenDone?.();
    _updUnlistenChunk = null;
    _updUnlistenDone = null;
}

function newRequestId() {
    return `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

interface AppState {
    // Data
    managers: ManagerInfo[];
    packages: Package[];
    updates: Update[];

    // Package stream loading state
    loadingManagers: string[]; // manager IDs currently in-flight
    finishedManagers: string[]; // manager IDs that have completed

    // Updates stream loading state
    loadingUpdateManagers: string[];
    finishedUpdateManagers: string[];

    // UI flags
    userMode: boolean;
    selectedManager: string;
    searchQuery: string;
    loading: boolean;
    updateLoading: boolean;
    updatingManager: string | null;
    firmwareUpdating: boolean;

    // Package cache: key = "${managerId}_u" or "${managerId}_s"
    // Populated per-manager as chunks arrive; checked before streaming.
    packageCache: Record<string, Package[]>;

    // Actions
    setUserMode: (v: boolean) => void;
    setSelectedManager: (id: string) => void;
    setSearchQuery: (q: string) => void;

    fetchManagers: () => Promise<void>;
    /** Stream packages — checks cache first, only fetches if uncached (or force=true). */
    streamPackages: (force?: boolean) => Promise<void>;
    /**
     * Force-refresh a single manager without clearing other managers' packages.
     * Clears that manager's cache and re-streams just it, merging results back.
     */
    refreshSingleManager: (managerId: string) => Promise<void>;
    /** Stream updates in parallel — returns immediately, UI gets live updates. */
    streamUpdates: () => Promise<void>;
    updateManager: (managerId: string) => Promise<void>;
    /** Run firmware updates via fwupdmgr (refresh + update), streamed to terminal. */
    updateFirmware: () => Promise<void>;

    // Icon cache
    iconCache: Record<string, string | null>;
    resolveIcon: (name: string) => Promise<string | null>;

    // Terminal
    terminalStagedCommands: StagedUninstall[];
    terminalOutput: TerminalEntry[];
    terminalRunning: boolean;
    stageUninstall: (manager: string, pkgId: string, displayName: string) => void;
    unstageCommand: (index: number) => void;
    clearTerminalOutput: () => void;
    executeTerminalCommands: () => Promise<void>;
}

export const useAppStore = create<AppState>((set, get) => ({
    managers: [],
    packages: [],
    updates: [],

    loadingManagers: [],
    finishedManagers: [],
    loadingUpdateManagers: [],
    finishedUpdateManagers: [],

    userMode: true,
    selectedManager: typeof localStorage !== "undefined" ? (localStorage.getItem("selectedManager") ?? "all") : "all",
    searchQuery: "",
    loading: false,
    updateLoading: false,
    updatingManager: null,
    firmwareUpdating: false,

    packageCache: {},

    iconCache: {},

    terminalStagedCommands: [],
    terminalOutput: [],
    terminalRunning: false,

    setUserMode: (v) => {
        set({ userMode: v });
        // Overview.tsx useEffect([selectedManager, userMode]) handles re-streaming
    },

    setSelectedManager: (id) => {
        // Don't call streamPackages() here — Overview.tsx useEffect([selectedManager])
        // fires on every change and is the single trigger for package loading.
        try {
            localStorage.setItem("selectedManager", id);
        } catch (_) {}
        set({ selectedManager: id });
    },

    setSearchQuery: (q) => set({ searchQuery: q }),

    fetchManagers: async () => {
        try {
            const managers = await invoke<ManagerInfo[]>("get_managers");
            set({ managers });
        } catch (e) {
            console.error("Failed to fetch managers:", e);
        }
    },

    // ─── Package streaming ───────────────────────────────────────────────────
    streamPackages: async (force = false) => {
        const { selectedManager, userMode, managers, packageCache } = get();
        const mode = userMode ? "u" : "s";

        // ── Cache check ──────────────────────────────────────────────────────
        if (!force) {
            if (selectedManager !== "all") {
                const cached = packageCache[`${selectedManager}_${mode}`];
                if (cached) {
                    set({
                        packages: cached,
                        loading: false,
                        loadingManagers: [],
                        finishedManagers: [selectedManager],
                    });
                    return;
                }
            } else {
                // "all": aggregate if every available manager is cached
                const availIds = managers.filter((m) => m.available).map((m) => m.id);
                const checkIds = availIds.length > 0 ? availIds : ALL_MANAGER_IDS;
                const allCached = checkIds.every((id) => `${id}_${mode}` in packageCache);
                if (allCached) {
                    const map = new Map<string, Package>();
                    checkIds.forEach((id) => {
                        (packageCache[`${id}_${mode}`] ?? []).forEach((p) => map.set(p.id, p));
                    });
                    set({
                        packages: Array.from(map.values()),
                        loading: false,
                        loadingManagers: [],
                        finishedManagers: [...checkIds],
                    });
                    return;
                }
            }
        }

        // ── Clear cache entries for managers we're about to re-fetch ─────────
        if (force) {
            const availIds = managers.filter((m) => m.available).map((m) => m.id);
            const clearIds = selectedManager === "all" ? (availIds.length > 0 ? availIds : ALL_MANAGER_IDS) : [selectedManager];
            const newCache = { ...packageCache };
            clearIds.forEach((id) => delete newCache[`${id}_${mode}`]);
            set({ packageCache: newCache });
        }

        // Tear down previous stream listeners before starting a new one.
        cleanupPackageListeners();

        const requestId = newRequestId();

        const { managers: mgrs } = get();
        const availableIds = mgrs.filter((m) => m.available).map((m) => m.id);
        const expectedIds = selectedManager === "all" ? (availableIds.length > 0 ? availableIds : ALL_MANAGER_IDS) : [selectedManager];

        set({
            packages: [],
            loadingManagers: [...expectedIds],
            finishedManagers: [],
            loading: true,
        });

        try {
            const myRequestId = requestId;
            _pkgUnlistenChunk = await listen<PackagesChunk>("packages::chunk", (event) => {
                if (event.payload.request_id !== myRequestId) return;

                const { manager, packages: newPkgs } = event.payload;
                const managerCacheKey = `${manager}_${mode}`;

                set((state) => {
                    // Merge into packages list (dedup by id)
                    const map = new Map(state.packages.map((p) => [p.id, p]));
                    newPkgs.forEach((p) => map.set(p.id, p));

                    // Accumulate per-manager cache
                    const existingForManager = state.packageCache[managerCacheKey] ?? [];
                    const managerMap = new Map(existingForManager.map((p) => [p.id, p]));
                    newPkgs.forEach((p) => managerMap.set(p.id, p));

                    return {
                        packages: Array.from(map.values()),
                        packageCache: {
                            ...state.packageCache,
                            [managerCacheKey]: Array.from(managerMap.values()),
                        },
                        loadingManagers: state.loadingManagers.filter((id) => id !== manager),
                        finishedManagers: [...state.finishedManagers, manager],
                    };
                });
            });

            _pkgUnlistenDone = await listen<PackagesDone>("packages::done", (event) => {
                if (event.payload.request_id !== myRequestId) return;
                set({ loading: false, loadingManagers: [] });
            });

            await invoke("stream_packages", {
                requestId,
                managerId: selectedManager,
                userMode,
            });
        } catch (e) {
            console.error("Failed to stream packages:", e);
            set({ loading: false, loadingManagers: [] });
            cleanupPackageListeners();
        }
    },

    // ─── Refresh a single manager without clearing other managers' data ──────
    refreshSingleManager: async (managerId: string) => {
        const { selectedManager, userMode, packageCache } = get();
        const mode = userMode ? "u" : "s";

        // Clear that manager's cache
        const newCache = { ...packageCache };
        delete newCache[`${managerId}_u`];
        delete newCache[`${managerId}_s`];
        set({ packageCache: newCache });

        // If the current view IS this manager, full re-stream is fine
        if (selectedManager === managerId) {
            get().streamPackages(true);
            return;
        }

        // We're viewing "all" (or another manager): stream just this one manager,
        // replace its packages in the current list, don't clear everything else.
        cleanupPackageListeners();
        const requestId = newRequestId();
        const myMode = mode;

        // Remove this manager's packages from current list while we reload
        set((state) => ({
            packages: state.packages.filter((p) => p.manager !== managerId),
            loadingManagers: [...state.loadingManagers, managerId],
            loading: true,
        }));

        try {
            const myRequestId = requestId;
            _pkgUnlistenChunk = await listen<PackagesChunk>("packages::chunk", (event) => {
                if (event.payload.request_id !== myRequestId) return;
                const { manager, packages: newPkgs } = event.payload;
                if (manager !== managerId) return;

                const managerCacheKey = `${manager}_${myMode}`;
                set((state) => {
                    const map = new Map(state.packages.map((p) => [p.id, p]));
                    newPkgs.forEach((p) => map.set(p.id, p));

                    const managerMap = new Map<string, Package>();
                    newPkgs.forEach((p) => managerMap.set(p.id, p));

                    return {
                        packages: Array.from(map.values()),
                        packageCache: {
                            ...state.packageCache,
                            [managerCacheKey]: Array.from(managerMap.values()),
                        },
                    };
                });
            });

            _pkgUnlistenDone = await listen<PackagesDone>("packages::done", (event) => {
                if (event.payload.request_id !== myRequestId) return;
                set((state) => ({
                    loading: state.loadingManagers.filter((id) => id !== managerId).length > 0,
                    loadingManagers: state.loadingManagers.filter((id) => id !== managerId),
                    finishedManagers: [...state.finishedManagers, managerId],
                }));
            });

            await invoke("stream_packages", {
                requestId,
                managerId,
                userMode,
            });
        } catch (e) {
            console.error("Failed to refresh manager:", e);
            set((state) => ({
                loading: false,
                loadingManagers: state.loadingManagers.filter((id) => id !== managerId),
            }));
            cleanupPackageListeners();
        }
    },

    // ─── Updates streaming ───────────────────────────────────────────────────
    streamUpdates: async () => {
        cleanupUpdateListeners();

        const requestId = newRequestId();
        // _currentUpdRequestId removed – listeners close over local `myUpdRequestId`.

        // Derive expected IDs from available managers.
        const { managers } = get();
        const availableIds = managers.filter((m) => m.available).map((m) => m.id);
        const expectedIds = availableIds.length > 0 ? availableIds : ALL_MANAGER_IDS;

        set({
            updates: [],
            loadingUpdateManagers: [...expectedIds],
            finishedUpdateManagers: [],
            updateLoading: true,
        });

        try {
            const myUpdRequestId = requestId;
            _updUnlistenChunk = await listen<UpdatesChunk>("updates::chunk", (event) => {
                if (event.payload.request_id !== myUpdRequestId) return;

                const { manager, updates: newUpdates } = event.payload;
                set((state) => {
                    // Deduplicate by package_id.
                    const map = new Map(state.updates.map((u) => [u.package_id + u.manager, u]));
                    newUpdates.forEach((u) => map.set(u.package_id + u.manager, u));
                    return {
                        updates: Array.from(map.values()),
                        loadingUpdateManagers: state.loadingUpdateManagers.filter((id) => id !== manager),
                        finishedUpdateManagers: [...state.finishedUpdateManagers, manager],
                    };
                });
            });

            _updUnlistenDone = await listen<UpdatesDone>("updates::done", (event) => {
                if (event.payload.request_id !== myUpdRequestId) return;
                // Don't call cleanupUpdateListeners() here for the same reason.
                set({ updateLoading: false, loadingUpdateManagers: [] });
            });

            await invoke("stream_updates", { requestId });
        } catch (e) {
            console.error("Failed to stream updates:", e);
            set({ updateLoading: false, loadingUpdateManagers: [] });
            cleanupUpdateListeners();
        }
    },

    updateManager: async (managerId) => {
        const { terminalRunning } = get();
        if (terminalRunning) return;

        set({ updatingManager: managerId, terminalRunning: true });
        cleanupTerminalListeners();

        try {
            const displayCmd = await invoke<string>("get_update_command", { managerId });
            const requestId = newRequestId();

            set((state) => ({
                terminalOutput: [...state.terminalOutput, { kind: "cmd", text: `$ ${displayCmd}` }],
            }));

            await new Promise<void>(async (resolve) => {
                const myId = requestId;

                _termUnlistenLine = await listen<{ request_id: string; text: string; is_stderr: boolean }>("terminal::line", (event) => {
                    if (event.payload.request_id !== myId) return;
                    set((state) => ({
                        terminalOutput: [...state.terminalOutput, { kind: event.payload.is_stderr ? "err" : "out", text: event.payload.text }],
                    }));
                });

                _termUnlistenDone = await listen<{ request_id: string; exit_code: number }>("terminal::done", (event) => {
                    if (event.payload.request_id !== myId) return;
                    const code = event.payload.exit_code;
                    set((state) => ({
                        terminalOutput: [...state.terminalOutput, { kind: "exit", text: code === 0 ? "✓ Done (exit 0)" : `✗ Exit code ${code}` }],
                    }));
                    cleanupTerminalListeners();
                    resolve();
                });

                try {
                    // Safe execution: backend builds command with proper args, no shell
                    await invoke("execute_update", { requestId, managerId });
                } catch (e) {
                    set((state) => ({
                        terminalOutput: [...state.terminalOutput, { kind: "err", text: String(e) }, { kind: "exit", text: "✗ Failed to invoke" }],
                    }));
                    cleanupTerminalListeners();
                    resolve();
                }
            });
        } finally {
            set({ updatingManager: null, terminalRunning: false });
            get().streamUpdates();
        }
    },

    updateFirmware: async () => {
        const { firmwareUpdating } = get();
        if (firmwareUpdating) return;

        set({ firmwareUpdating: true });
        cleanupTerminalListeners();

        const requestId = newRequestId();

        set((state) => ({
            terminalOutput: [...state.terminalOutput, { kind: "cmd", text: "$ pkexec fwupdmgr refresh && pkexec fwupdmgr update" }],
        }));

        await new Promise<void>(async (resolve) => {
            const myId = requestId;

            _termUnlistenLine = await listen<{ request_id: string; text: string; is_stderr: boolean }>("terminal::line", (event) => {
                if (event.payload.request_id !== myId) return;
                set((state) => ({
                    terminalOutput: [...state.terminalOutput, { kind: event.payload.is_stderr ? "err" : "out", text: event.payload.text }],
                }));
            });

            _termUnlistenDone = await listen<{ request_id: string; exit_code: number }>("terminal::done", (event) => {
                if (event.payload.request_id !== myId) return;
                const code = event.payload.exit_code;
                set((state) => ({
                    terminalOutput: [...state.terminalOutput, { kind: "exit", text: code === 0 ? "\u2713 Firmware update complete (exit 0)" : `\u2717 Firmware update failed (exit ${code})` }],
                }));
                cleanupTerminalListeners();
                resolve();
            });

            try {
                await invoke("update_firmware", { requestId });
            } catch (e) {
                set((state) => ({
                    terminalOutput: [...state.terminalOutput, { kind: "err", text: String(e) }, { kind: "exit", text: "\u2717 Failed to invoke firmware update" }],
                }));
                cleanupTerminalListeners();
                resolve();
            }
        });

        set({ firmwareUpdating: false });
    },

    resolveIcon: async (name: string) => {
        const cache = get().iconCache;
        if (name in cache) return cache[name];

        try {
            const path = await invoke<string | null>("find_icon", { name });
            set((state) => ({ iconCache: { ...state.iconCache, [name]: path } }));
            return path;
        } catch {
            set((state) => ({ iconCache: { ...state.iconCache, [name]: null } }));
            return null;
        }
    },

    stageUninstall: (manager: string, pkgId: string, displayName: string) => {
        set((state) => {
            // Don't stage the same package twice
            if (state.terminalStagedCommands.some((s) => s.pkgId === pkgId)) return state;
            return { terminalStagedCommands: [...state.terminalStagedCommands, { manager, pkgId, displayName }] };
        });
    },

    unstageCommand: (index: number) => {
        set((state) => ({
            terminalStagedCommands: state.terminalStagedCommands.filter((_, i) => i !== index),
        }));
    },

    clearTerminalOutput: () => {
        set({ terminalOutput: [] });
    },

    executeTerminalCommands: async () => {
        const { terminalStagedCommands, terminalRunning } = get();
        if (terminalRunning || terminalStagedCommands.length === 0) return;

        // Confirmation dialog — prevents accidental removal of critical packages
        const names = terminalStagedCommands.map((s) => s.displayName).join(", ");
        if (!window.confirm(`Remove ${terminalStagedCommands.length} package(s)?\n\n${names}`)) {
            return;
        }

        const staged = [...terminalStagedCommands];
        set({ terminalStagedCommands: [], terminalRunning: true });

        cleanupTerminalListeners();

        // Group by manager so we can batch into one command per manager
        const byManager = new Map<string, string[]>();
        for (const item of staged) {
            const list = byManager.get(item.manager) ?? [];
            list.push(item.pkgId);
            byManager.set(item.manager, list);
        }

        // Execute safely per manager
        for (const [managerId, pkgIds] of byManager) {
            const requestId = newRequestId();

            // Display-only: get human-readable command for the terminal panel
            try {
                const displayCmd = await invoke<string>("get_batch_uninstall_command", { managerId, pkgIds });
                set((state) => ({
                    terminalOutput: [...state.terminalOutput, { kind: "cmd", text: `$ ${displayCmd}` }],
                }));
            } catch {
                set((state) => ({
                    terminalOutput: [...state.terminalOutput, { kind: "cmd", text: `$ [uninstall ${pkgIds.length} from ${managerId}]` }],
                }));
            }

            await new Promise<void>(async (resolve) => {
                const myId = requestId;

                _termUnlistenLine = await listen<{ request_id: string; text: string; is_stderr: boolean }>("terminal::line", (event) => {
                    if (event.payload.request_id !== myId) return;
                    set((state) => ({
                        terminalOutput: [...state.terminalOutput, { kind: event.payload.is_stderr ? "err" : "out", text: event.payload.text }],
                    }));
                });

                _termUnlistenDone = await listen<{ request_id: string; exit_code: number }>("terminal::done", (event) => {
                    if (event.payload.request_id !== myId) return;
                    const code = event.payload.exit_code;
                    set((state) => ({
                        terminalOutput: [...state.terminalOutput, { kind: "exit", text: code === 0 ? "✓ Done (exit 0)" : `✗ Exit code ${code}` }],
                    }));
                    cleanupTerminalListeners();
                    resolve();
                });

                try {
                    // Safe execution: backend builds command with proper args, no shell
                    await invoke("execute_batch_uninstall", { requestId, managerId, pkgIds });
                } catch (e) {
                    set((state) => ({
                        terminalOutput: [...state.terminalOutput, { kind: "err", text: String(e) }, { kind: "exit", text: "✗ Failed to invoke" }],
                    }));
                    cleanupTerminalListeners();
                    resolve();
                }
            });
        }

        set({ terminalRunning: false });

        // Refresh packages after terminal commands (like uninstalls) finish
        get().streamPackages(true);
    },
}));
