import { useCallback, useEffect, useMemo, useState } from "react";
import HomePage from "./components/HomePage";
import ProcessingOverlay from "./components/ProcessingOverlay";
import CompletionReport from "./components/CompletionReport";
import {
  appVersion,
  cancelScan,
  loadTaxonomy,
  onScanProgress,
  startScan,
} from "./lib/tauri";
import type {
  AnalysisReport,
  FlatCategory,
  ScanConfig,
  ScanEvent,
  Theme,
} from "./lib/types";

type Stage =
  | { kind: "home" }
  | { kind: "processing"; events: ScanEvent[]; current: ScanEvent | null }
  | { kind: "complete"; report: AnalysisReport };

const DEFAULT_THEME: Theme = "dark";

export default function App() {
  const [stage, setStage] = useState<Stage>({ kind: "home" });
  const [taxonomy, setTaxonomy] = useState<FlatCategory[]>([]);
  const [version, setVersion] = useState<string>("");
  const [theme, setTheme] = useState<Theme>(DEFAULT_THEME);
  const [scanError, setScanError] = useState<string | null>(null);
  const [config, setConfig] = useState<ScanConfig | null>(null);

  useEffect(() => {
    const root = document.documentElement;
    if (theme === "dark") root.classList.add("dark");
    else root.classList.remove("dark");
  }, [theme]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [cats, v] = await Promise.all([loadTaxonomy(), appVersion()]);
        if (cancelled) return;
        setTaxonomy(cats);
        setVersion(v);
      } catch (e) {
        console.error("Failed to load taxonomy", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await onScanProgress((event) => {
        setStage((prev) => {
          if (prev.kind !== "processing") return prev;
          if (event.kind === "complete") {
            return { kind: "complete", report: event.report };
          }
          if (event.kind === "cancelled") {
            return { kind: "home" };
          }
          return {
            kind: "processing",
            events: [...prev.events, event],
            current: event,
          };
        });
      });
    })();
    return () => {
      unlisten?.();
    };
  }, []);

  const handleStart = useCallback(async (cfg: ScanConfig) => {
    setScanError(null);
    setConfig(cfg);
    setStage({ kind: "processing", events: [], current: null });
    try {
      await startScan(cfg);
    } catch (err) {
      const msg = typeof err === "string" ? err : (err as Error)?.message ?? String(err);
      setScanError(msg);
      setStage({ kind: "home" });
    }
  }, []);

  const handleCancel = useCallback(async () => {
    try {
      await cancelScan();
    } catch {
      // Already done — ignore.
    }
    setStage({ kind: "home" });
  }, []);

  const handleClose = useCallback(() => {
    setStage({ kind: "home" });
  }, []);

  const themeButton = useMemo(
    () => ({
      theme,
      onToggle: () => setTheme(theme === "dark" ? "light" : "dark"),
    }),
    [theme],
  );

  return (
    <div className="relative h-screen w-screen overflow-hidden">
      <HomePage
        taxonomy={taxonomy}
        version={version}
        themeBtn={themeButton}
        onStart={handleStart}
        lastError={scanError}
        previousConfig={config}
      />
      {stage.kind === "processing" && (
        <ProcessingOverlay
          events={stage.events}
          current={stage.current}
          onCancel={handleCancel}
        />
      )}
      {stage.kind === "complete" && (
        <CompletionReport
          report={stage.report}
          onClose={handleClose}
        />
      )}
    </div>
  );
}
