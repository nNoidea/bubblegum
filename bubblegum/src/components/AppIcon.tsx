import { useEffect, useState } from "react";
import { Package } from "lucide-react";
import { useAppStore } from "../store";
import clsx from "clsx";

interface AppIconProps {
    iconName?: string;
    name: string;
    color: string;
    size?: number;
}

export function AppIcon({ iconName, name, color, size = 48 }: AppIconProps) {
    const resolveIcon = useAppStore((s) => s.resolveIcon);
    const [src, setSrc] = useState<string | null>(null);

    useEffect(() => {
        if (!iconName) return;
        // resolveIcon now returns a data URL (data:image/...;base64,...) directly
        resolveIcon(iconName).then((dataUrl) => {
            if (dataUrl) setSrc(dataUrl);
        });
    }, [iconName, resolveIcon]);

    const initials = name
        .split(/[\s\-_.]/)
        .slice(0, 2)
        .map((w) => w[0]?.toUpperCase() ?? "")
        .join("");

    if (src) {
        return (
            <img
                src={src}
                alt={name}
                style={{ width: size, height: size }}
                className="rounded-xl object-contain"
                onError={() => setSrc(null)}
            />
        );
    }

    return (
        <div
            className={clsx("rounded-xl flex items-center justify-center font-bold select-none")}
            style={{
                width: size,
                height: size,
                background: `${color}22`,
                border: `1px solid ${color}44`,
                color,
                fontSize: size * 0.35,
            }}
        >
            {initials || <Package size={size * 0.5} />}
        </div>
    );
}
