// TypeScript mirrors of the Rust serde structs in src-tauri/src/.

export type MatchMode = "use_groups" | "auto_detect" | "manual";
export type Tiebreaker = "project_then_clip" | "clip_then_project";

export type ScanSource =
  | { kind: "root"; root: string }
  | { kind: "projects"; projects: string[] };

export interface ManualKeywords {
  per_category: Record<string, string[]>;
}

export type ScanSourceWire =
  | { root: string }
  | { projects: string[] };

export interface ScanConfig {
  source: ScanSourceWire;
  output_folder: string;
  selected_categories: string[];
  top_n: number;
  match_mode: MatchMode;
  manual_keywords: ManualKeywords;
  include_freeze: boolean;
  include_processed: boolean;
  include_recorded: boolean;
  include_missing: boolean;
  tiebreaker: Tiebreaker;
}

export interface FlatCategory {
  path: string;
  components: string[];
  name: string;
  keywords: string[];
  depth: number;
}

export interface UniqueSample {
  canonical_path: string;
  filename: string;
  file_size: number | null;
  content_hash: string | null;
  original_path: string | null;
  track_name: string | null;
  category: string | null;
  project_count: number;
  clip_count: number;
  projects: string[];
  missing: boolean;
  factory: boolean;
}

export interface CategoryReport {
  path: string;
  display_name: string;
  components: string[];
  samples: UniqueSample[];
  total_occurrences: number;
  project_count: number;
}

export interface ParseErrorEntry {
  project_path: string;
  message: string;
}

export interface AnalysisReport {
  categories: CategoryReport[];
  parse_errors: ParseErrorEntry[];
  output_root: string;
  projects_scanned: number;
  unique_samples: number;
  total_occurrences: number;
  app_version: string;
  run_timestamp: string;
}

export type ScanEvent =
  | { kind: "discovery_started"; root: string }
  | { kind: "project_found"; path: string; total: number }
  | {
      kind: "project_parsed";
      path: string;
      index: number;
      total: number;
      samples_found: number;
    }
  | { kind: "parse_error"; path: string; error: string }
  | { kind: "dedup_started"; total_samples: number }
  | { kind: "dedup_progress"; processed: number; total: number }
  | { kind: "copy_started"; total: number }
  | {
      kind: "copy_progress";
      copied: number;
      total: number;
      current_filename: string;
    }
  | { kind: "complete"; report: AnalysisReport }
  | { kind: "cancelled" };

export type Theme = "dark" | "light";
