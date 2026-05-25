import { useMemo } from "react";
import { cn } from "../lib/utils";
import type {
  FlatCategory,
  ManualKeywords,
  MatchMode,
} from "../lib/types";

interface Props {
  mode: MatchMode;
  onChange: (m: MatchMode) => void;
  selectedCategories: string[];
  categories: FlatCategory[];
  manualKeywords: ManualKeywords;
  onManualChange: (k: ManualKeywords) => void;
}

const MODES: { value: MatchMode; label: string; hint: string }[] = [
  {
    value: "use_groups",
    label: "Use Groups",
    hint: "Classify by Ableton group track path (most accurate when you organize by group)",
  },
  {
    value: "auto_detect",
    label: "Auto-detect",
    hint: "Classify by filename keywords with track-name fallback",
  },
  {
    value: "manual",
    label: "Manual",
    hint: "Provide your own keywords per category",
  },
];

export default function MatchingAlgorithmPicker(props: Props) {
  const { mode, onChange, selectedCategories, categories, manualKeywords, onManualChange } = props;

  const selectedCats = useMemo(
    () => categories.filter((c) => selectedCategories.includes(c.path)),
    [categories, selectedCategories],
  );

  function updateManual(catPath: string, text: string) {
    const keywords = text
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    onManualChange({
      ...manualKeywords,
      per_category: {
        ...manualKeywords.per_category,
        [catPath]: keywords,
      },
    });
  }

  function manualValue(cat: FlatCategory): string {
    const stored = manualKeywords.per_category[cat.path];
    if (stored && stored.length > 0) return stored.join(", ");
    return cat.keywords.join(", ");
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-3 gap-1.5">
        {MODES.map((m) => (
          <button
            key={m.value}
            onClick={() => onChange(m.value)}
            aria-pressed={mode === m.value}
            title={m.hint}
            className={cn(
              "rounded-md border px-2.5 py-2 text-left transition",
              mode === m.value
                ? "border-accent-400/40 bg-accent-500/20 text-ink-50 shadow-sm"
                : "border-white/[0.06] bg-ink-800/60 text-ink-300 hover:text-ink-100",
            )}
          >
            <div className="text-[12.5px] font-medium">{m.label}</div>
            <div className="mt-0.5 line-clamp-2 text-[10.5px] text-ink-400">
              {m.hint}
            </div>
          </button>
        ))}
      </div>

      {mode === "manual" && selectedCats.length > 0 && (
        <div className="flex flex-col gap-1.5 rounded-md border border-white/[0.05] bg-ink-900/30 [.light_&]:bg-ink-50 p-2">
          <div className="px-1 text-[10.5px] uppercase tracking-[0.1em] text-ink-400">
            Keywords per category (comma-separated)
          </div>
          <div className="flex max-h-[140px] flex-col gap-1.5 overflow-y-auto">
            {selectedCats.map((cat) => (
              <div key={cat.path} className="grid grid-cols-[140px_1fr] items-center gap-2">
                <div
                  title={cat.path}
                  className="truncate text-[11.5px] text-ink-200"
                >
                  {cat.name}
                </div>
                <input
                  value={manualValue(cat)}
                  onChange={(e) => updateManual(cat.path, e.target.value)}
                  spellCheck={false}
                  className="sv-input h-7 text-[11.5px]"
                  placeholder="e.g. kick, bd, bassdrum"
                />
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
