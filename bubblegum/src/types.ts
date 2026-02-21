// Mirrors Rust structs from managers/mod.rs

export interface Package {
    id: string;
    name: string;
    version: string;
    description?: string;
    manager: string;
    source?: string;
    is_user_installed: boolean;
    icon_name?: string;
    category?: string;
    size_bytes?: number;
}

export interface ManagerInfo {
    id: string;
    name: string;
    available: boolean;
    version?: string;
    color: string;
    emoji: string;
}

export interface Update {
    package_id: string;
    name: string;
    current_version: string;
    new_version: string;
    manager: string;
    source?: string;
}

// ─── Streaming event payloads (mirrors Rust) ─────────────────────────────────
// Every chunk/done event carries the request_id of the stream that produced it.
// The frontend ignores events whose request_id doesn't match the current active
// stream, which prevents stale in-flight threads from showing duplicate results
// after the user switches managers.

export interface PackagesChunk {
    request_id: string;
    manager: string;
    packages: Package[];
}

export interface PackagesDone {
    request_id: string;
}

export interface UpdatesChunk {
    request_id: string;
    manager: string;
    updates: Update[];
}

export interface UpdatesDone {
    request_id: string;
}
