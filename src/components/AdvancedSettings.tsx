import { Info } from "lucide-react";
import { cn } from "../lib/utils";
import type { Tiebreaker } from "../lib/types";

interface Props {
  includeFreeze: boolean;
  setIncludeFreeze: (v: boolean) => void;
  includeProcessed: boolean;
  setIncludeProcessed: (v: boolean) => void;
  includeRecorded: boolean;
  setIncludeRecorded: (v: boolean) => void;
  includeMissing: boolean;
  setIncludeMissing: (v: boolean) => void;
  tiebreaker: Tiebreaker;
  setTiebreaker: (t: Tiebreaker) => void;
}

const FREEZE_TOOLTIP =
  "Includes the .wav files Ableton generates when you freeze a track. Turn on if you frequently 'Bounce to New Track' and want those audio bounces counted as samples.";
const PROCESSED_TOOLTIP =
  "Includes warp/stretch renders, consolidation outputs, and other audio produced by Ableton's processing — but not freezes.";
const RECORDED_TOOLTIP =
  "Includes audio recorded into the project during that session (Samples/Recorded/).";

export default function AdvancedSettings(props: Props) {
  return (
    <div className="flex flex-col gap-4">
      <div>
        <div className="mb-2 text-[10.5px] uppercase tracking-[0.1em] text-ink-400">
          Include samples from
        </div>
        <div className="flex flex-col gap-1.5">
          <Checkbox
            label="Frozen tracks (Samples/Processed/Freeze/)"
            checked={props.includeFreeze}
            onChange={props.setIncludeFreeze}
            tooltip={FREEZE_TOOLTIP}
          />
          <Checkbox
            label="Other processed audio (Samples/Processed/)"
            checked={props.includeProcessed}
            onChange={props.setIncludeProcessed}
            tooltip={PROCESSED_TOOLTIP}
          />
          <Checkbox
            label="Session recordings (Samples/Recorded/)"
            checked={props.includeRecorded}
            onChange={props.setIncludeRecorded}
            tooltip={RECORDED_TOOLTIP}
          />
        </div>
      </div>

      <div className="sv-divider" />

      <div>
        <div className="mb-2 text-[10.5px] uppercase tracking-[0.1em] text-ink-400">
          Other settings
        </div>
        <Checkbox
          label="Include samples whose source no longer exists on disk"
          checked={props.includeMissing}
          onChange={props.setIncludeMissing}
          tooltip="Off by default — Soundvault can't copy a sample whose file has been moved or deleted."
        />

        <div className="mt-3 grid grid-cols-[170px_1fr] items-center gap-2">
          <div className="text-[11.5px] text-ink-200">Tiebreaker</div>
          <div className="inline-flex rounded-md border border-white/[0.06] bg-ink-800/60 p-0.5 text-[11px]">
            {(
              [
                ["project_then_clip", "Projects → clips"],
                ["clip_then_project", "Clips → projects"],
              ] as [Tiebreaker, string][]
            ).map(([v, label]) => (
              <button
                key={v}
                onClick={() => props.setTiebreaker(v)}
                aria-pressed={props.tiebreaker === v}
                className={cn(
                  "rounded px-2 py-1 transition",
                  props.tiebreaker === v
                    ? "bg-accent-500/80 text-white"
                    : "text-ink-300 hover:text-ink-100",
                )}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function Checkbox({
  label,
  checked,
  onChange,
  tooltip,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  tooltip?: string;
}) {
  return (
    <label className="group flex items-center gap-2 py-1 text-[12px] text-ink-200">
      <span
        role="checkbox"
        aria-checked={checked}
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === " " || e.key === "Enter") {
            e.preventDefault();
            onChange(!checked);
          }
        }}
        onClick={() => onChange(!checked)}
        className={cn(
          "flex h-4 w-4 cursor-pointer items-center justify-center rounded-[4px] border transition",
          checked
            ? "border-accent-400/80 bg-accent-500/80"
            : "border-white/15 bg-ink-800 group-hover:border-white/30",
        )}
      >
        {checked && (
          <svg viewBox="0 0 12 12" className="h-3 w-3 text-white">
            <path
              d="M2.5 6.5l2.5 2.5 4.5-5"
              stroke="currentColor"
              strokeWidth="1.6"
              strokeLinecap="round"
              strokeLinejoin="round"
              fill="none"
            />
          </svg>
        )}
      </span>
      <span className="flex-1">{label}</span>
      {tooltip && (
        <span
          title={tooltip}
          className="cursor-help text-ink-500 transition hover:text-ink-300"
        >
          <Info className="h-3.5 w-3.5" />
        </span>
      )}
    </label>
  );
}
