import { useEffect, useMemo, useState } from "react";
import {
  FolderOpen,
  FolderInput,
  Play,
  Moon,
  SunMedium,
  Vault,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { cn, truncateMiddle } from "../lib/utils";
import { pickFolder, validateConfig } from "../lib/tauri";
import type {
  FlatCategory,
  MatchMode,
  ScanConfig,
  ScanSourceWire,
  Theme,
  Tiebreaker,
  ManualKeywords,
} from "../lib/types";
import TypeSelector from "./TypeSelector";
import MatchingAlgorithmPicker from "./MatchingAlgorithmPicker";
import AdvancedSettings from "./AdvancedSettings";

type ScanMode = "folder" | "projects";

const TOP_N_OPTIONS = [5, 10, 15, 20, 25, 30, 35, 40, 45, 50];

interface Props {
  taxonomy: FlatCategory[];
  version: string;
  themeBtn: { theme: Theme; onToggle: () => void };
  onStart: (config: ScanConfig) => void;
  lastError: string | null;
  previousConfig: ScanConfig | null;
}

export default function HomePage(props: Props) {
  const { taxonomy, version, themeBtn, onStart, lastError, previousConfig } = props;

  const [scanMode, setScanMode] = useState<ScanMode>("folder");
  const [folderRoot, setFolderRoot] = useState<string | null>(null);
  const [projectPaths, setProjectPaths] = useState<string[]>([]);
  const [outputFolder, setOutputFolder] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [topN, setTopN] = useState<number>(25);
  const [matchMode, setMatchMode] = useState<MatchMode>("use_groups");
  const [manualKeywords, setManualKeywords] = useState<ManualKeywords>({
    per_category: {},
  });
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [includeFreeze, setIncludeFreeze] = useState(false);
  const [includeProcessed, setIncludeProcessed] = useState(false);
  const [includeRecorded, setIncludeRecorded] = useState(false);
  const [includeMissing, setIncludeMissing] = useState(false);
  const [tiebreaker, setTiebreaker] = useState<Tiebreaker>("project_then_clip");
  const [validateError, setValidateError] = useState<string | null>(null);

  // Restore previous config when user X's back from complete state.
  useEffect(() => {
    if (!previousConfig) return;
    if ("root" in previousConfig.source) {
      setScanMode("folder");
      setFolderRoot(previousConfig.source.root);
      setProjectPaths([]);
    } else {
      setScanMode("projects");
      setProjectPaths(previousConfig.source.projects);
      setFolderRoot(null);
    }
    setOutputFolder(previousConfig.output_folder);
    setSelected(new Set(previousConfig.selected_categories));
    setTopN(previousConfig.top_n);
    setMatchMode(previousConfig.match_mode);
    setManualKeywords(previousConfig.manual_keywords);
    setIncludeFreeze(previousConfig.include_freeze);
    setIncludeProcessed(previousConfig.include_processed);
    setIncludeRecorded(previousConfig.include_recorded);
    setIncludeMissing(previousConfig.include_missing);
    setTiebreaker(previousConfig.tiebreaker);
  }, [previousConfig]);

  const source: ScanSourceWire | null = useMemo(() => {
    if (scanMode === "folder" && folderRoot) return { root: folderRoot };
    if (scanMode === "projects" && projectPaths.length > 0)
      return { projects: projectPaths };
    return null;
  }, [scanMode, folderRoot, projectPaths]);

  const config: ScanConfig | null = useMemo(() => {
    if (!source || !outputFolder || selected.size === 0) return null;
    return {
      source,
      output_folder: outputFolder,
      selected_categories: Array.from(selected),
      top_n: topN,
      match_mode: matchMode,
      manual_keywords: manualKeywords,
      include_freeze: includeFreeze,
      include_processed: includeProcessed,
      include_recorded: includeRecorded,
      include_missing: includeMissing,
      tiebreaker,
    };
  }, [
    source,
    outputFolder,
    selected,
    topN,
    matchMode,
    manualKeywords,
    includeFreeze,
    includeProcessed,
    includeRecorded,
    includeMissing,
    tiebreaker,
  ]);

  const canStart = !!config && !validateError;

  const sourceDisplay = useMemo(() => {
    if (scanMode === "folder") return folderRoot ?? "No folder selected";
    if (projectPaths.length === 0) return "No projects selected";
    if (projectPaths.length === 1) return projectPaths[0];
    return `${projectPaths.length} projects selected`;
  }, [scanMode, folderRoot, projectPaths]);

  async function chooseProjectSource() {
    setValidateError(null);
    if (scanMode === "folder") {
      const r = await pickFolder({ title: "Select projects root folder" });
      if (typeof r === "string") {
        setFolderRoot(r);
        setProjectPaths([]);
      }
    } else {
      const r = await pickFolder({
        title: "Select one or more Ableton project folders",
        multiple: true,
      });
      if (Array.isArray(r)) {
        setProjectPaths(r);
        setFolderRoot(null);
      } else if (typeof r === "string") {
        setProjectPaths([r]);
      }
    }
  }

  async function chooseOutput() {
    setValidateError(null);
    const r = await pickFolder({ title: "Select output folder" });
    if (typeof r === "string") setOutputFolder(r);
  }

  async function handleStart() {
    if (!config) return;
    try {
      await validateConfig(config);
      setValidateError(null);
      onStart(config);
    } catch (err) {
      const msg = typeof err === "string" ? err : (err as Error)?.message ?? String(err);
      setValidateError(msg);
    }
  }

  return (
    <div className="relative flex h-full w-full flex-col bg-gradient-to-b from-ink-950 to-ink-900 dark:from-ink-950 dark:to-ink-900 [.light_&]:from-white [.light_&]:to-ink-100">
      <div className="sv-titlebar-spacer" />

      {/* Header */}
      <header className="relative z-10 flex items-center justify-between gap-3 px-6 pb-3 pt-1 sv-no-drag">
        <div className="flex items-center gap-2.5">
          <div className="relative flex h-9 w-9 items-center justify-center rounded-lg border border-white/[0.07] bg-gradient-to-br from-accent-400/30 via-accent-500/20 to-ink-900/40 shadow-glow">
            <Vault className="h-4 w-4 text-accent-200" />
          </div>
          <div className="leading-tight">
            <div className="text-[15px] font-semibold tracking-tight text-ink-50">
              Soundvault
            </div>
            <div className="text-[10.5px] uppercase tracking-[0.12em] text-ink-400">
              {version ? `v${version}` : "v1.0.0"} · read-only on your projects
            </div>
          </div>
        </div>
        <button
          aria-label="Toggle theme"
          onClick={themeBtn.onToggle}
          className="sv-button-ghost h-8 w-8 p-0"
        >
          {themeBtn.theme === "dark" ? (
            <SunMedium className="h-4 w-4" />
          ) : (
            <Moon className="h-4 w-4" />
          )}
        </button>
      </header>

      {/* Content */}
      <main className="relative z-0 flex-1 overflow-y-auto px-6 pb-6 pt-2 sv-no-drag">
        <div className="mx-auto flex max-w-[640px] flex-col gap-4">
          {/* Project source row */}
          <section className="sv-card flex flex-col gap-3 animate-rise-in">
            <div className="flex items-center justify-between gap-3">
              <div className="sv-label">Select project(s)</div>
              <ScanModeToggle mode={scanMode} onChange={(m) => {
                setScanMode(m);
                setFolderRoot(null);
                setProjectPaths([]);
              }} />
            </div>
            <div className="sv-row">
              <button
                onClick={chooseProjectSource}
                className="sv-button min-w-[140px]"
              >
                <FolderOpen className="h-4 w-4 text-accent-300" />
                {scanMode === "folder" ? "Choose folder" : "Choose projects"}
              </button>
              <PathDisplay value={sourceDisplay} empty={!folderRoot && projectPaths.length === 0} />
            </div>
          </section>

          {/* Output folder row */}
          <section className="sv-card flex flex-col gap-3 animate-rise-in" style={{ animationDelay: "30ms" }}>
            <div className="sv-label">Select output folder</div>
            <div className="sv-row">
              <button onClick={chooseOutput} className="sv-button min-w-[140px]">
                <FolderInput className="h-4 w-4 text-accent-300" />
                Choose folder
              </button>
              <PathDisplay value={outputFolder ?? "No folder selected"} empty={!outputFolder} />
            </div>
          </section>

          {/* Type selector */}
          <section className="sv-card flex flex-col gap-3 animate-rise-in" style={{ animationDelay: "60ms" }}>
            <div className="flex items-center justify-between">
              <div className="sv-label">Select type(s)</div>
              <div className="text-[11px] text-ink-400">
                {selected.size === 0 ? "None selected" : `${selected.size} selected`}
              </div>
            </div>
            <TypeSelector
              categories={taxonomy}
              selected={selected}
              onChange={setSelected}
            />
          </section>

          {/* Top N */}
          <section className="sv-card flex flex-col gap-3 animate-rise-in" style={{ animationDelay: "90ms" }}>
            <div className="flex items-center justify-between gap-3">
              <div className="sv-label">Top samples per category</div>
              <div className="text-[12px] font-mono text-ink-200">{topN}</div>
            </div>
            <TopNStepper value={topN} onChange={setTopN} options={TOP_N_OPTIONS} />
          </section>

          {/* Matching algorithm */}
          <section className="sv-card flex flex-col gap-3 animate-rise-in" style={{ animationDelay: "120ms" }}>
            <div className="sv-label">Matching algorithm</div>
            <MatchingAlgorithmPicker
              mode={matchMode}
              onChange={setMatchMode}
              selectedCategories={Array.from(selected)}
              categories={taxonomy}
              manualKeywords={manualKeywords}
              onManualChange={setManualKeywords}
            />
          </section>

          {/* Advanced */}
          <section className="sv-card flex flex-col gap-3 animate-rise-in" style={{ animationDelay: "150ms" }}>
            <button
              onClick={() => setAdvancedOpen((v) => !v)}
              className="flex w-full items-center justify-between gap-2 text-left"
            >
              <div className="sv-label">Advanced settings</div>
              {advancedOpen ? (
                <ChevronDown className="h-4 w-4 text-ink-400" />
              ) : (
                <ChevronRight className="h-4 w-4 text-ink-400" />
              )}
            </button>
            {advancedOpen && (
              <AdvancedSettings
                includeFreeze={includeFreeze}
                setIncludeFreeze={setIncludeFreeze}
                includeProcessed={includeProcessed}
                setIncludeProcessed={setIncludeProcessed}
                includeRecorded={includeRecorded}
                setIncludeRecorded={setIncludeRecorded}
                includeMissing={includeMissing}
                setIncludeMissing={setIncludeMissing}
                tiebreaker={tiebreaker}
                setTiebreaker={setTiebreaker}
              />
            )}
          </section>

          {(validateError || lastError) && (
            <div
              role="alert"
              className="sv-card border-red-500/20 bg-red-950/30 text-[12.5px] text-red-200"
            >
              {validateError ?? lastError}
            </div>
          )}

          {/* Start */}
          <button
            onClick={handleStart}
            disabled={!canStart}
            className="sv-button-primary mt-1 h-11 w-full text-[14px] animate-rise-in"
            style={{ animationDelay: "180ms" }}
          >
            <Play className="h-4 w-4" />
            Start
          </button>
        </div>
      </main>
    </div>
  );
}

function ScanModeToggle({
  mode,
  onChange,
}: {
  mode: ScanMode;
  onChange: (m: ScanMode) => void;
}) {
  return (
    <div className="inline-flex rounded-md border border-white/[0.06] bg-ink-800/60 p-0.5 text-[11px]">
      {(["folder", "projects"] as ScanMode[]).map((m) => (
        <button
          key={m}
          onClick={() => onChange(m)}
          aria-pressed={mode === m}
          className={cn(
            "rounded px-2.5 py-1 transition",
            mode === m
              ? "bg-accent-500/80 text-white shadow-sm"
              : "text-ink-300 hover:text-ink-100",
          )}
        >
          {m === "folder" ? "Folder" : "Pick projects"}
        </button>
      ))}
    </div>
  );
}

function PathDisplay({ value, empty }: { value: string; empty: boolean }) {
  const display = empty ? value : truncateMiddle(value, 64);
  return (
    <div
      title={empty ? undefined : value}
      className={cn(
        "h-9 truncate rounded-lg border border-white/[0.04] bg-ink-800/50 px-3",
        "flex items-center font-mono text-[12px]",
        empty ? "text-ink-500 italic" : "text-ink-200",
      )}
    >
      {display}
    </div>
  );
}

function TopNStepper({
  value,
  onChange,
  options,
}: {
  value: number;
  onChange: (v: number) => void;
  options: number[];
}) {
  return (
    <div className="grid grid-cols-10 gap-1.5">
      {options.map((n) => (
        <button
          key={n}
          onClick={() => onChange(n)}
          aria-pressed={value === n}
          className={cn(
            "h-8 rounded-md text-[11.5px] font-medium font-mono transition",
            value === n
              ? "bg-accent-500/80 text-white shadow-sm"
              : "bg-ink-800/60 text-ink-300 hover:bg-ink-700/70 hover:text-ink-100",
          )}
        >
          {n}
        </button>
      ))}
    </div>
  );
}
