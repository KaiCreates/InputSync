import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppStatus } from "../App";
import "./Panel.css";

interface Props {
  status: AppStatus;
  onStatusChange: () => void;
}

export default function ServerPanel({ status, onStatusChange }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const isServer = status.role === "server";
  const isClient = status.role === "client";

  const startServer = async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("cmd_start_server");
      onStatusChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const stopServer = async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("cmd_stop_server");
      onStatusChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const toggleCapture = async () => {
    try {
      await invoke("cmd_toggle_capture");
      onStatusChange();
    } catch (e) {
      setError(String(e));
    }
  };

  const copyCode = async () => {
    if (status.session_code) {
      await navigator.clipboard.writeText(status.session_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  };

  return (
    <section className="panel">
      <div className="panel-title">
        <span className="panel-icon">⬡</span>
        Host / Server
        {isServer && (
          <span className="panel-badge active">Active</span>
        )}
      </div>

      {!isServer ? (
        <button
          className="btn-primary"
          onClick={startServer}
          disabled={loading || isClient}
        >
          {loading ? "Starting..." : "Start Server"}
        </button>
      ) : (
        <div className="server-active">
          <div className="info-grid">
            <div className="info-row">
              <span className="info-label">Session Code</span>
              <div className="code-display">
                <span className="code-value">{status.session_code}</span>
                <button className="btn-copy btn-secondary" onClick={copyCode}>
                  {copied ? "✓" : "Copy"}
                </button>
              </div>
            </div>
            <div className="info-row">
              <span className="info-label">Address</span>
              <span className="info-value mono">{status.local_ip}</span>
            </div>
            <div className="info-row">
              <span className="info-label">Clients</span>
              <span className="info-value">{status.client_count}</span>
            </div>
            <div className="info-row">
              <span className="info-label">Capture</span>
              <div className="capture-toggle">
                <button
                  className={`toggle-btn ${status.capturing ? "active" : ""}`}
                  onClick={toggleCapture}
                >
                  {status.capturing ? "ON" : "OFF"}
                </button>
                <span className="info-muted">
                  {status.capturing ? "Forwarding input" : "Input not captured"}
                </span>
              </div>
            </div>
          </div>

          <button
            className="btn-danger"
            onClick={stopServer}
            disabled={loading}
            style={{ marginTop: 16 }}
          >
            {loading ? "Stopping..." : "Stop Server"}
          </button>
        </div>
      )}

      {error && <div className="panel-error">{error}</div>}
    </section>
  );
}
