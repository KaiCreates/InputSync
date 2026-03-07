import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppStatus } from "../App";
import "./Panel.css";

interface Props {
  status: AppStatus;
  onStatusChange: () => void;
}

export default function ClientPanel({ status, onStatusChange }: Props) {
  const [serverIp, setServerIp] = useState("");
  const [sessionCode, setSessionCode] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isClient = status.role === "client";
  const isServer = status.role === "server";

  const connect = async () => {
    if (!serverIp.trim()) {
      setError("Server IP is required");
      return;
    }
    if (sessionCode.trim().length !== 6) {
      setError("Session code must be 6 characters");
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await invoke("cmd_connect", {
        serverIp: serverIp.trim(),
        sessionCode: sessionCode.trim().toUpperCase(),
      });
      onStatusChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const disconnect = async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("cmd_disconnect");
      onStatusChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleCodeInput = (v: string) => {
    setSessionCode(v.toUpperCase().replace(/[^A-Z0-9]/g, "").slice(0, 6));
  };

  return (
    <section className="panel">
      <div className="panel-title">
        <span className="panel-icon">◈</span>
        Connect to Server
        {isClient && (
          <span className="panel-badge connected">Connected</span>
        )}
      </div>

      {!isClient ? (
        <div className="client-form">
          <div className="form-group">
            <label className="form-label">Session Code</label>
            <input
              type="text"
              placeholder="ABC123"
              value={sessionCode}
              onChange={(e) => handleCodeInput(e.target.value)}
              disabled={isServer}
              maxLength={6}
              className="code-input"
            />
          </div>
          <div className="form-group">
            <label className="form-label">Server IP Address</label>
            <input
              type="text"
              placeholder="192.168.1.100"
              value={serverIp}
              onChange={(e) => setServerIp(e.target.value)}
              disabled={isServer}
              onKeyDown={(e) => e.key === "Enter" && connect()}
            />
          </div>
          <button
            className="btn-primary"
            onClick={connect}
            disabled={loading || isServer}
            style={{ marginTop: 4 }}
          >
            {loading ? "Connecting..." : "Connect"}
          </button>
        </div>
      ) : (
        <div className="connected-info">
          <div className="info-grid">
            <div className="info-row">
              <span className="info-label">Server</span>
              <span className="info-value mono">{status.server_addr}</span>
            </div>
            <div className="info-row">
              <span className="info-label">Status</span>
              <span className="status-dot connected">Receiving input</span>
            </div>
            {status.latency_ms !== null && (
              <div className="info-row">
                <span className="info-label">Latency</span>
                <span className="info-value">{status.latency_ms.toFixed(1)} ms</span>
              </div>
            )}
          </div>
          <button
            className="btn-danger"
            onClick={disconnect}
            disabled={loading}
            style={{ marginTop: 16 }}
          >
            {loading ? "Disconnecting..." : "Disconnect"}
          </button>
        </div>
      )}

      {error && <div className="panel-error">{error}</div>}
    </section>
  );
}
