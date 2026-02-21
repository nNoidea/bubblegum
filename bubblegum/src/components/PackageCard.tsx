import { useState } from "react";
import { Trash2, Copy, Check, ShieldCheck, ShieldOff, RefreshCw } from "lucide-react";
import type { Package } from "../types";
import { AppIcon } from "./AppIcon";
import { ManagerBadge } from "./ManagerBadge";
import { SourceBadge } from "./SourceBadge";
import { useAppStore } from "../store";
import { MANAGER_COLORS } from "../constants";
import clsx from "clsx";

interface PackageCardProps {
    pkg: Package;
}

const AUTO_UPDATE_MANAGERS = new Set(["flatpak", "snap", "nix", "apt", "dnf", "pacman", "brew", "zypper", "yum", "xbps"]);
const MANUAL_UPDATE_MANAGERS = new Set(["cargo", "npm", "pip", "pip3", "gem", "go"]);

function getManagedStatus(manager: string, source?: string | null): { label: string; icon: "auto" | "manual" | "none" } {
    if (source === "locally-installed" || source === "manual") return { label: "Manual install — no auto-updates", icon: "none" };
    if (AUTO_UPDATE_MANAGERS.has(manager)) return { label: `auto-updates ${manager}`, icon: "auto" };
    if (MANUAL_UPDATE_MANAGERS.has(manager)) return { label: `manual updates ${manager}`, icon: "manual" };
    return { label: `Source: ${manager}`, icon: "manual" };
}

export function PackageCard({ pkg }: PackageCardProps) {
    const stageUninstall = useAppStore((s) => s.stageUninstall);
    const addTerminalInfo = useAppStore((s) => s.addTerminalInfo);
    const [staged, setStaged] = useState(false);
    const [copied, setCopied] = useState(false);
 
    const color = MANAGER_COLORS[pkg.manager] ?? "#6B6B8A";
 
    async function handleStageUninstall(e: React.MouseEvent) {
        e.stopPropagation();
        if (pkg.manager === "local") {
            addTerminalInfo("This app is not installed via a package manager, you need to figure out self how to uninstall it.");
            return;
        }
        stageUninstall(pkg.manager, pkg.id, pkg.name);
        setStaged(true);
        setTimeout(() => setStaged(false), 2000);
    }

    async function handleCopyName(e: React.MouseEvent) {
        e.stopPropagation();
        await navigator.clipboard.writeText(pkg.name);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
    }

    function formatSize(bytes?: number) {
        if (!bytes) return null;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
        return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    }

    const status = getManagedStatus(pkg.manager, pkg.source);
    const iconEl =
        status.icon === "auto" ? (
            <ShieldCheck
                size={14}
                style={{ color: "#26a269" }}
            />
        ) : status.icon === "manual" ? (
            <RefreshCw
                size={14}
                style={{ color: "#a86e00" }}
            />
        ) : (
            <ShieldOff
                size={14}
                style={{ color: "var(--color-muted)" }}
            />
        );

    return (
        <div className="flat-card rounded-xl p-4 flex flex-col gap-3 relative">
            <div className="flex items-start justify-between">
                <AppIcon
                    iconName={pkg.icon_name}
                    name={pkg.name}
                    color={color}
                    size={64}
                />
                <div
                    className="flex flex-col shrink-0 rounded-lg overflow-hidden"
                    style={{ border: "1px solid var(--color-border)" }}
                >
                    <button
                        className={clsx("p-2 transition-colors", "hover:bg-blue-500/20 hover:text-blue-400")}
                        style={{ color: "var(--color-muted)", borderBottom: "1px solid var(--color-border)" }}
                        onClick={handleCopyName}
                        title="Copy package name"
                    >
                        {copied ? (
                            <Check
                                size={16}
                                style={{ color: "#34D399" }}
                            />
                        ) : (
                            <Copy size={16} />
                        )}
                    </button>
                    <button
                        className={clsx("p-2 transition-colors", staged ? "bg-red-500/20" : "hover:bg-red-500/20 hover:text-red-400")}
                        style={{ color: staged ? "#f87171" : "var(--color-muted)" }}
                        onClick={handleStageUninstall}
                        title="Stage for removal in terminal"
                    >
                        <Trash2 size={16} />
                    </button>
                </div>
            </div>

            <div className="flex flex-col gap-0.5">
                <span
                    className="font-semibold text-lg truncate max-w-full"
                    style={{ color: "var(--color-text)" }}
                >
                    {pkg.name}
                </span>
                <div
                    className="text-xs flex items-center gap-1.5 overflow-hidden"
                    style={{ color: "var(--color-muted)" }}
                >
                    <span className="truncate min-w-0">{pkg.version}</span>
                    {pkg.size_bytes && <span className="shrink-0">{formatSize(pkg.size_bytes)}</span>}
                </div>
            </div>

            <div className="flex flex-wrap gap-1.5 items-center">
                {pkg.is_user_installed && (
                    <span
                        className="text-xs px-1.5 py-0.5 rounded shrink-0"
                        style={{
                            background: "rgba(145, 65, 172, 0.18)",
                            color: "var(--color-purple)",
                            border: "1px solid rgba(145,65,172,0.35)",
                        }}
                    >
                        yours
                    </span>
                )}
                <ManagerBadge managerId={pkg.manager} />
                {pkg.source && <SourceBadge source={pkg.source} />}
            </div>

            {pkg.category && (
                <div className="flex">
                    <span
                        className="text-xs px-1.5 py-0.5 rounded-md"
                        style={{
                            background: "var(--color-border)",
                            color: "var(--color-subtext)",
                        }}
                    >
                        {pkg.category}
                    </span>
                </div>
            )}

            {pkg.description ? (
                <p
                    className="text-xs leading-relaxed line-clamp-2"
                    style={{ color: "var(--color-subtext)" }}
                >
                    {pkg.description}
                </p>
            ) : (
                <p
                    className="text-xs italic"
                    style={{ color: "var(--color-muted)" }}
                >
                    No description available.
                </p>
            )}

            <div
                className="mt-auto pt-2 flex items-center gap-1.5 text-xs font-medium"
                style={{ color: status.icon === "auto" ? "#26a269" : status.icon === "manual" ? "#a86e00" : "var(--color-muted)" }}
            >
                {iconEl}
                <span>{status.label}</span>
            </div>
        </div>
    );
}
