import { useMemo, useState } from "react";
import { X, FolderOpen, ChevronDown, ChevronRight, CheckCircle2, Sparkles, AlertTriangle } from "lucide-react";
import { cn, truncateMiddle } from "../lib/utils";
import { revealPath } from "../lib/tauri";
import type { AnalysisReport, UniqueSample } from "../lib/types";
import ProcessingPulse from "./animations/ProcessingPulse";

interface Props {
  report: AnalysisReport;
  onClose: () => void;
}

export default function CompletionReport({ report, onClose }: Props) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [errorsOpen, setErrorsOpen] = useState(false);

  const totalSaved = useMemo(
    () => report.categories.reduce((a, c) => a + c.samples.length, 0),
    [report],
  );

  function toggle(catPath: string) {
    const next = new Set(expanded);
    if (next.has(catPath)) next.delete(catPath);
    else next.add(catPath);
    setExpanded(next);
  }

  async function openOutput() {
    try { await revealPath(report.output_root); } catch { /* soft-fail */ }
  }

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center sv-glass-overlay animate-fade-in">
      <div role="dialog" aria-modal="true" aria-label="Scan complete"
        className="sv-card relative flex w-[540px] max-w-[92vw] flex-col gap-4 px-7 py-6 animate-rise-in max-h-[88vh]">
        <button onClick={onClose} aria-label="Back to setup"
          className="absolute left-3 top-3 flex h-7 w-7 items-center justify-center rounded-md text-ink-300 transition hover:bg-white/[0.05] hover:text-ink-50">
          <X className="h-4 w-4" />
        </button>
        <div className="flex flex-col items-center gap-3">
          <ProcessingPulse totalProjects={report.projects_scanned} projectsParsed={report.projects_scanned} dedupRatio={1} copyRatio={1} done />
          <div className="flex items-center gap-2 text-[13.5px] font-semibold text-ink-50">
            <CheckCircle2 className="h-4 w-4 text-accent-300" />
            Scan complete
          </div>
          <div className="text-[12px] text-ink-300">
            {totalSaved.toLocaleString()} samples saved across {report.categories.length} {report.categories.length === 1 ? "category" : "categories"} ·{" "}
            {report.projects_scanned.toLocaleString()} projects scanned
          </div>
        </div>
        <button onClick={openOutput} className="sv-button-primary h-10 w-full">
          <FolderOpen className="h-4 w-4" />
          Open consolidated folder
        </button>
        <div className="flex flex-col gap-1.5 overflow-y-auto pr-1" style={{ maxHeight: "44vh" }}>
          {report.categories.length === 0 && (
            <div className="rounded-md border border-white/[0.05] bg-ink-900/40 p-4 text-center text-[12.5px] text-ink-300">
              No samples matched the selected categories.
            </div>
          )}
          {report.categories.map((cat) => {
            const open = expanded.has(cat.path);
            return (
              <div key={cat.path} className="rounded-md border border-white/[0.05] bg-ink-900/40 [.light_&]:bg-white">
                <button onClick={() => toggle(cat.path)} className="flex w-full items-center justify-between gap-2 px-3 py-2.5 text-left">
                  <div className="flex min-w-0 items-center gap-2">
                    {open ? <ChevronDown className="h-3.5 w-3.5 text-ink-400" /> : <ChevronRight className="h-3.5 w-3.5 text-ink-400" />}
                    <div className="flex min-w-0 flex-col">
                      <div className="truncate text-[12.5px] font-medium text-ink-100">{cat.display_name}</div>
                      <div className="truncate text-[10.5px] text-ink-400">{cat.path}</div>
                    </div>
                  </div>
                  <div className="shrink-0 text-[11.5px] font-mono text-ink-300">
                    {cat.samples.length} saved · {cat.total_occurrences.toLocaleString()} occurrences · {cat.project_count} {cat.project_count === 1 ? "project" : "projects"}
                  </div>
                </button>
                {open && (
                  <div className="border-t border-white/[0.04] [.light_&]:border-ink-900/5">
                    {cat.samples.map((s, idx) => <SampleRow key={`${cat.path}-${idx}`} idx={idx} sample={s} />)}
                  </div>
                )}
              </div>
            );
          })}
        </div>
        {report.parse_errors.length > 0 && (
          <div className="rounded-md border border-amber-400/20 bg-amber-500/[0.06] p-3">
            <button onClick={() => setErrorsOpen((v) => !v)} className="flex w-full items-center justify-between text-left">
              <div className="flex items-center gap-2 text-[12px] text-amber-200">
                <AlertTriangle className="h-3.5 w-3.5" />
                {report.parse_errors.length} project{report.parse_errors.length === 1 ? "" : "s"} had errors — review
              </div>
              {errorsOpen ? <ChevronDown className="h-3.5 w-3.5 text-amber-200" /> : <ChevronRight className="h-3.5 w-3.5 text-amber-200" />}
            </button>
            {errorsOpen && (
              <ul className="mt-2 flex flex-col gap-1 text-[11px] text-amber-100/90">
                {report.parse_errors.map((e, i) => (
                  <li key={i}>
                    <span className="font-mono">{truncateMiddle(e.project_path, 48)}</span>: {e.message}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function SampleRow({ idx, sample }: { idx: number; sample: UniqueSample }) {
  const isWinner = idx === 0;
  return (
    <div className={cn("grid grid-cols-[1fr_auto] items-center gap-3 px-3 py-1.5 text-[11.5px]",
        idx > 0 && "border-t border-white/[0.03] [.light_&]:border-ink-900/5")}>
      <div className="flex min-w-0 items-center gap-2">
        {isWinner && <Sparkles className="h-3 w-3 text-glow-400" />}
        <div className="flex min-w-0 flex-col">
          <div className={cn("truncate font-mono", isWinner ? "text-glow-400" : "text-ink-100")} title={sample.filename}>
            {sample.filename}
            {sample.factory && (
              <span className="ml-2 rounded-sm bg-white/[0.06] px-1 py-px text-[9px] uppercase tracking-wider text-ink-400">factory</span>
            )}
          </div>
          <div className="truncate text-[10px] text-ink-400" title={sample.original_path ?? sample.canonical_path}>
            {truncateMiddle(sample.original_path ?? sample.canonical_path, 56)}
          </div>
        </div>
      </div>
      <div className="shrink-0 text-right font-mono text-[10.5px] text-ink-300">
        {sample.project_count} {sample.project_count === 1 ? "project" : "projects"} · {sample.clip_count} use{sample.clip_count === 1 ? "" : "s"}
      </div>
    </div>
  );
}
