import { useMemo } from "react";

/**
 * Constellation-style progress visual. Each "node" represents a project.
 * As projects finish parsing, their node lights up; the most recently lit
 * node pulses. When everything is parsed the whole constellation softly
 * glows. At the dedup/copy stages the inner ring fills as a separate ring.
 *
 * Tied to real progress — not a stock spinner.
 */
interface Props {
  totalProjects: number;
  projectsParsed: number;
  dedupRatio: number;   // 0..1
  copyRatio: number;    // 0..1
  done?: boolean;
}

export default function ProcessingPulse({
  totalProjects,
  projectsParsed,
  dedupRatio,
  copyRatio,
  done,
}: Props) {
  const safeTotal = Math.max(totalProjects, 1);
  const nodes = useMemo(() => {
    const count = Math.min(Math.max(safeTotal, 8), 36);
    const radius = 62;
    return Array.from({ length: count }, (_, i) => {
      const angle = (i / count) * Math.PI * 2 - Math.PI / 2;
      return {
        i,
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
      };
    });
  }, [safeTotal]);

  // Map "projects parsed" to a fraction of nodes lit. We always animate the
  // ring proportionally to projectsParsed/totalProjects, regardless of the
  // visual node count.
  const litFraction = done ? 1 : Math.min(projectsParsed / safeTotal, 1);

  const outerRingProgress = litFraction;
  // Inner rings show dedup + copy as nested arcs.
  return (
    <svg
      width="180"
      height="180"
      viewBox="-90 -90 180 180"
      role="presentation"
      aria-hidden
      className="drop-shadow-[0_0_40px_rgba(122,131,245,0.35)]"
    >
      <defs>
        <radialGradient id="sv-pulse-glow" cx="0" cy="0" r="80" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="#7a83f5" stopOpacity="0.45" />
          <stop offset="60%" stopColor="#7a83f5" stopOpacity="0.07" />
          <stop offset="100%" stopColor="#7a83f5" stopOpacity="0" />
        </radialGradient>
        <linearGradient id="sv-arc" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#9ba4ff" />
          <stop offset="100%" stopColor="#6470e8" />
        </linearGradient>
      </defs>

      <circle r="84" fill="url(#sv-pulse-glow)" />

      {/* Background ring */}
      <circle
        r="62"
        fill="none"
        stroke="rgba(155,164,255,0.10)"
        strokeWidth="1"
      />

      {/* Project nodes */}
      {nodes.map((n) => {
        const isLit = (n.i / nodes.length) < litFraction;
        const isHead =
          isLit && Math.abs(n.i / nodes.length - litFraction) < 1 / nodes.length;
        return (
          <circle
            key={n.i}
            cx={n.x}
            cy={n.y}
            r={isHead ? 3.4 : isLit ? 2.4 : 1.4}
            fill={isLit ? "#9ba4ff" : "rgba(155,164,255,0.20)"}
            className={isHead && !done ? "animate-pulse-soft" : undefined}
          />
        );
      })}

      {/* Outer arc — overall progress (parsing) */}
      <ProgressArc r={48} progress={outerRingProgress} strokeWidth={3} />

      {/* Dedup arc */}
      {dedupRatio > 0 && (
        <ProgressArc r={36} progress={dedupRatio} strokeWidth={2.5} hue="cyan" />
      )}

      {/* Copy arc */}
      {copyRatio > 0 && (
        <ProgressArc r={26} progress={copyRatio} strokeWidth={2.5} hue="accent" />
      )}

      {done && (
        <g>
          <circle r="22" fill="rgba(155,164,255,0.18)" />
          <path
            d="M -7 0 L -2 6 L 9 -6"
            stroke="#dde1ff"
            strokeWidth="2.6"
            strokeLinecap="round"
            strokeLinejoin="round"
            fill="none"
          />
        </g>
      )}
    </svg>
  );
}

function ProgressArc({
  r,
  progress,
  strokeWidth,
  hue = "accent",
}: {
  r: number;
  progress: number;
  strokeWidth: number;
  hue?: "accent" | "cyan";
}) {
  const c = 2 * Math.PI * r;
  const p = Math.min(Math.max(progress, 0), 1);
  return (
    <circle
      r={r}
      fill="none"
      stroke={hue === "cyan" ? "#7cd5e6" : "url(#sv-arc)"}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeDasharray={`${c * p} ${c * (1 - p)}`}
      transform="rotate(-90)"
      style={{ transition: "stroke-dasharray 220ms ease-out" }}
    />
  );
}
