import { Outlet } from "react-router-dom";

export function Layout() {
    return (
        <div
            className="flex flex-col h-screen w-screen overflow-hidden"
            style={{ background: "var(--color-bg)" }}
        >
            <Outlet />
        </div>
    );
}
