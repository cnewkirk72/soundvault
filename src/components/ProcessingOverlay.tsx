import { useMemo } from "react";
import { X, Square } from "lucide-react";
import ProcessingPulse from "./animations/ProcessingPulse";
import { basename, truncateMiddle } from "../lib/utils";
import type { ScanEvent } from "../lib/types";

interface Props {
  events: ScanEvent[];
  current: ScanEvent | null;
  onCancel: () => void;
}

interface DerivedProgress {
  projectsTotal: number;
  projectsFound: number;
  projectsParsed: number;
  samplesFound: number;
  dedupTotal: number;
  dedupProcessed: number;
  copyTotal: number;
  copyCopied: number;
  currentFilename: string | null;
  currentProjectName: string | null;
  parseErrors: number;
  stage: "discovery" | "parse" | "dedup" | "copy" | "idle";
}

function derive(events: ScanEvent[]): DerivedProgress {
  let p: DerivedProgress = {
    projectsTotal: 0,
    projectsFound: 0,
    projectsParsed: 0,
    samplesFound: 0,
    dedupTotal: 0,
    dedupProcessed: 0,
    copyTotal: 0,
    copyCopied: 0,
    currentFilename: null,
    currentProjectName: null,
    parseErrors: 0,
    stage: "idle",
  };
  for (const ev of events) {
    switch (ev.kind) {
      case "discovery_started":
        p.stage = "discovery";
        break;
      case "project_found":
        p.projectsTotal = ev.total;
        p.projectsFound += 1;
        break;
      case "project_parsed":
        p.stage = "parse";
        p.projectsTotal = ev.total;
        p.projectsParsed = ev.index;
        p.samplesFound += ev.samples_found;
        p.currentProjectName = basename(ev.path);
        break;
      case "parse_error":
        p.parseErrors += 1;
        break;
      case "dedup_started":
        p.stage = "dedup";
        p.dedupTotal = ev.total_samples;
        p.dedupProcessed = 0;
        break;
      case "dedup_progress":
        p.dedupTotal = ev.total;
        p.dedupProcessed = ev.processed;
        break;
      case "copy_started":
        p.stage = "copy";
        p.copyTotal = ev.total;
        p.copyCopied = 0;
        break;
      case "copy_progress":
        p.copyTotal = ev.total;
        p.copyCopied = ev.copied;
        p.currentFilename = ev.current_filename;
        break;
      default:
        break;
    }
  }
  return p;
}

export default function ProcessingOverlay({ events, onCancel }: Props) {
  const p = useMemo(() => derive(events), [events]);

  const statusLine = useMemo(() => {
    if (p.stage === "discovery") {
      return `Discovering projects… (${p.projectsFound} found)`;
    }
    if (p.stage === "parse") {
      const projectName = p.currentProjectName ?? "…";
      return `Parsing project ${p.projectsParsed} of ${p.projectsTotal}: ${truncateMiddle(projectName, 36)}`;
    }
    if (p.stage === "dedup") {
      const pct = p.dedupTotal > 0 ? Math.round((p.dedupProcessed / p.dedupTotal) * 100) : 0;
      return `Deduplicating… ${pct}%`;
    }
    if (p.stage === "copy") {
      return `Copying ${p.copyCopied} of ${p.copyTotal}: ${truncateMiddle(p.currentFilename ?? "…", 36)}`;
    }
    return "Starting up…";
  }, [p]);

  const secondaryLine = useMemo(() => {
    if (p.stage === "parse" || p.stage === "dedup" || p.stage === "copy") {
      const projectsLabel = p.projectsTotal > 0 ? `${p.projectsParsed || p.projectsTotal} projects` : "projects";
      return `Found ${p.samplesFound.toLocaleString()} samples across ${projectsLabel}`;
    }
    return null;
  }, [p]);

  const dedupRatio = p.dedupTotal > 0 ? p.dedupProcessed / p.dedupTotal : 0;
  const copyRatio = p.copyTotal > 0 ? p.copyCopied / p.copyTotal : 0;

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center sv-glass-overlay animate-fade-in">
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Scan in progress"
        className="sv-card relative flex w-[480px] max-w-[90vw] flex-col items-center gap-5 px-7 py-7 animate-rise-in"
      >
        <button
          onClick={onCancel}
          aria-label="Cancel scan"
          className="absolute left-3 top-3 flex h-7 w-7 items-center justify-center rounded-md text-ink-300 transition hover:bg-white/[0.05] hover:text-ink-50"
        >
          <X className="h-4 w-4" />
        </button>

        <ProcessingPulse
          totalProjects={p.projectsTotal}
          projectsParsed={p.projectsParsed}
          dedupRatio={dedupRatio}
          copyRatio={copyRatio}
        />

        <div className="flex flex-col items-center gap-1.5 text-center">
          <div className="text-[13.5px] font-medium text-ink-50">{statusLine}</div>
          {secondaryLine && (
            <div className="text-[12px] text-ink-300">{secondaryLine}</div>
          )}
          {p.parseErrors > 0 && (
            <div className="mt-1 text-[11px] text-amber-300/90">
              {p.parseErrors} project{p.parseErrors === 1 ? "" : "s"} skipped due to parse errors
            </div>
          )}
        </div>

        <button
          onClick={onCancel}
          className="sv-button gap-2 h-9 px-4"
        >
          <Square className="h-3.5 w-3.5" />
          Stop
        </button>
      </div>
    </div>
  );
}
