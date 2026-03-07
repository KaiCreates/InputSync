import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ServerPanel from "./components/ServerPanel";
import ClientPanel from "./components/ClientPanel";
import StatusBar from "./components/StatusBar";
import "./App.css";

export interface AppStatus {
  role: "idle" | "server" | "client";
  session_code: string | null;
  local_ip: string | null;
  server_addr: string | null;
  client_count: number;
  capturing: boolean;
  latency_ms: number | null;
}

export default function App() {
  const [status, setStatus] = useState<AppStatus>({
    role: "idle",
    session_code: null,
    local_ip: null,
    server_addr: null,
    client_count: 0,
    capturing: false,
    latency_ms: null,
  });

  const refreshStatus = async () => {
    try {
      const s = await invoke<AppStatus>("cmd_get_status");
      setStatus(s);
    } catch (e) {
      console.error("Status fetch failed:", e);
    }
  };

  useEffect(() => {
    refreshStatus();
    const interval = setInterval(refreshStatus, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="app">
      <header className="app-header">
        <div className="logo">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <rect x="2" y="6" width="8" height="12" rx="2" fill="var(--accent)" opacity="0.9"/>
            <rect x="14" y="6" width="8" height="12" rx="2" fill="var(--accent)" opacity="0.5"/>
            <path d="M10 12h4M12 10l2 2-2 2" stroke="var(--accent)" strokeWidth="1.5" strokeLinecap="round"/>
          </svg>
          <span>InputSync</span>
        </div>
        <div className="header-status">
          <span className={`role-badge role-${status.role}`}>
            {status.role.toUpperCase()}
          </span>
        </div>
      </header>

      <main className="app-main">
        <ServerPanel status={status} onStatusChange={refreshStatus} />
        <div className="divider" />
        <ClientPanel status={status} onStatusChange={refreshStatus} />
      </main>

      <StatusBar status={status} />
    </div>
  );
}
