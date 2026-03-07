interface Props {
  size?: number;
}

// Pixel art CRT terminal monitor logo
// ViewBox: 40×30 — each unit = 1 "pixel" in the art
export default function PixelLogo({ size = 44 }: Props) {
  const h = Math.round(size * 30 / 40);
  return (
    <svg
      width={size}
      height={h}
      viewBox="0 0 40 30"
      shapeRendering="crispEdges"
      fill="none"
      aria-label="InputSync logo"
    >
      {/* ── Chassis ──────────────────────────────────────────── */}
      <rect x="0" y="0" width="40" height="24" fill="#4a42c8" />

      {/* ── Title bar (darker strip) ─────────────────────────── */}
      <rect x="0" y="0" width="40" height="6" fill="#3830a8" />

      {/* Traffic-light control dots */}
      <rect x="2" y="2" width="2" height="2" fill="#f04444" />
      <rect x="6" y="2" width="2" height="2" fill="#f59e0b" />
      <rect x="10" y="2" width="2" height="2" fill="#3ecf8e" />

      {/* Tiny "IS" pixel label in title bar */}
      <rect x="19" y="2" width="1" height="3" fill="rgba(255,255,255,0.35)" />
      <rect x="21" y="2" width="1" height="1" fill="rgba(255,255,255,0.35)" />
      <rect x="21" y="3" width="1" height="1" fill="rgba(255,255,255,0.35)" />
      <rect x="21" y="4" width="1" height="1" fill="rgba(255,255,255,0.35)" />
      <rect x="23" y="2" width="2" height="1" fill="rgba(255,255,255,0.35)" />
      <rect x="23" y="3" width="1" height="1" fill="rgba(255,255,255,0.35)" />
      <rect x="23" y="4" width="2" height="1" fill="rgba(255,255,255,0.35)" />

      {/* Title bar bottom separator */}
      <rect x="0" y="6" width="40" height="1" fill="#2c268c" />

      {/* ── CRT Screen ───────────────────────────────────────── */}
      <rect x="2" y="7" width="36" height="16" fill="#060b14" />

      {/* Scanlines — 1px dark every 2 rows */}
      {[7, 9, 11, 13, 15, 17, 19, 21].map((y) => (
        <rect key={y} x="2" y={y} width="36" height="1" fill="rgba(0,0,0,0.22)" />
      ))}

      {/* Phosphor glow overlay */}
      <rect x="2" y="7" width="36" height="16" fill="rgba(62,207,142,0.03)" />

      {/* ── > character — pixel art (each dot = 2×2) ─────────── */}
      {/* row 0 */ }
      <rect x="5" y="9"  width="2" height="2" fill="#3ecf8e" />
      {/* row 1 */}
      <rect x="7" y="11" width="2" height="2" fill="#3ecf8e" />
      {/* row 2 — arrowpoint */}
      <rect x="9" y="13" width="2" height="2" fill="#3ecf8e" />
      {/* row 3 */}
      <rect x="7" y="15" width="2" height="2" fill="#3ecf8e" />
      {/* row 4 */}
      <rect x="5" y="17" width="2" height="2" fill="#3ecf8e" />

      {/* ── _ cursor (6×2, blinking) ──────────────────────────── */}
      <rect x="13" y="17" width="6" height="2" fill="#3ecf8e">
        <animate
          attributeName="opacity"
          values="1;1;0;0;1"
          dur="1.4s"
          repeatCount="indefinite"
        />
      </rect>

      {/* ── Bottom bezel line ─────────────────────────────────── */}
      <rect x="0" y="23" width="40" height="1" fill="#2c268c" />

      {/* ── Stand neck ───────────────────────────────────────── */}
      <rect x="16" y="24" width="8" height="2" fill="#4a42c8" />

      {/* ── Stand base ───────────────────────────────────────── */}
      <rect x="8"  y="26" width="24" height="2" fill="#4a42c8" />
      <rect x="10" y="28" width="20" height="2" fill="#3830a8" />
    </svg>
  );
}
