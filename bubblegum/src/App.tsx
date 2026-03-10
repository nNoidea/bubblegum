import { useEffect } from "react";
import { HashRouter, Routes, Route } from "react-router-dom";
import { Layout } from "./components/Layout";
import { Overview } from "./pages/Overview";
import { Updates } from "./pages/Updates";
import { useAppStore } from "./store";
import "./styles.css";

function App() {
    const { fetchManagers, streamUpdates } = useAppStore();

    useEffect(() => {
        // Only load managers here; Overview.tsx useEffect handles package streaming
        // when the Overview mounts or selectedManager changes.
        fetchManagers();
        // Pre-warm the updates stream so the sidebar badge shows immediately.
        streamUpdates();
    }, []);

    return (
        <HashRouter>
            <Routes>
                <Route
                    path="/"
                    element={<Layout />}
                >
                    <Route
                        index
                        element={<Overview />}
                    />
                    <Route
                        path="updates"
                        element={<Updates />}
                    />
                </Route>
            </Routes>
        </HashRouter>
    );
}

export default App;
