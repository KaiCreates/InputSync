import { AppStatus } from "../App";
import "./StatusBar.css";

interface Props {
  status: AppStatus;
}

export default function StatusBar({ status }: Props) {
  const getStatusText = () => {
    switch (status.role) {
      case "server":
        return `Server active · ${status.client_count} client${status.client_count !== 1 ? "s" : ""} connected`;
      case "client":
        return `Connected to ${status.server_addr}`;
      default:
        return "Ready — Start server or connect to one";
    }
  };

  return (
    <footer className="status-bar">
      <div className="status-indicator">
        <span className={`dot dot-${status.role}`} />
        <span className="status-text">{getStatusText()}</span>
      </div>
      <span className="status-version">v0.1.0</span>
    </footer>
  );
}
