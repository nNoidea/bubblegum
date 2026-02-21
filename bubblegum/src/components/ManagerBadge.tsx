import { useAppStore } from "../store";
import clsx from "clsx";

interface ManagerBadgeProps {
    managerId: string;
    className?: string;
}

export function ManagerBadge({ managerId, className }: ManagerBadgeProps) {
    const managers = useAppStore((s) => s.managers);
    const mgr = managers.find((m) => m.id === managerId);

    const color = mgr?.color ?? "#6B6B8A";
    const emoji = mgr?.emoji ?? "📦";
    const name = mgr?.name ?? managerId;

    return (
        <span
            className={clsx("inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium", className)}
            style={{
                background: `${color}1a`,
                border: `1px solid ${color}40`,
                color,
            }}
        >
            <span>{emoji}</span>
            <span>{name}</span>
        </span>
    );
}
