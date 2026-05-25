import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, Square, CheckSquare } from "lucide-react";
import { cn } from "../lib/utils";
import type { FlatCategory } from "../lib/types";

interface Props {
  categories: FlatCategory[];
  selected: Set<string>;
  onChange: (s: Set<string>) => void;
}

interface TreeNode {
  category: FlatCategory;
  children: TreeNode[];
}

function buildTree(cats: FlatCategory[]): TreeNode[] {
  // Sort by depth ascending, then by path so parents come before children.
  const sorted = [...cats].sort((a, b) =>
    a.depth - b.depth || a.path.localeCompare(b.path),
  );
  const byPath: Record<string, TreeNode> = {};
  const roots: TreeNode[] = [];
  for (const c of sorted) {
    const node: TreeNode = { category: c, children: [] };
    byPath[c.path] = node;
    if (c.depth === 1) {
      roots.push(node);
    } else {
      const parentPath = c.components.slice(0, -1).join(" / ");
      const parent = byPath[parentPath];
      if (parent) parent.children.push(node);
      else roots.push(node);
    }
  }
  return roots;
}

function collectDescendants(node: TreeNode, out: string[]) {
  out.push(node.category.path);
  for (const child of node.children) collectDescendants(child, out);
}

export default function TypeSelector({ categories, selected, onChange }: Props) {
  const tree = useMemo(() => buildTree(categories), [categories]);
  const [expanded, setExpanded] = useState<Set<string>>(
    () => new Set(categories.filter((c) => c.depth <= 2).map((c) => c.path)),
  );

  function toggleExpand(path: string) {
    const next = new Set(expanded);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setExpanded(next);
  }

  function toggleSelect(node: TreeNode) {
    const next = new Set(selected);
    const paths: string[] = [];
    collectDescendants(node, paths);
    const allOn = paths.every((p) => next.has(p));
    if (allOn) {
      for (const p of paths) next.delete(p);
    } else {
      for (const p of paths) next.add(p);
    }
    onChange(next);
  }

  function indeterminateOrChecked(node: TreeNode): "off" | "on" | "partial" {
    const paths: string[] = [];
    collectDescendants(node, paths);
    const onCount = paths.filter((p) => selected.has(p)).length;
    if (onCount === 0) return "off";
    if (onCount === paths.length) return "on";
    return "partial";
  }

  return (
    <div className="max-h-[220px] overflow-y-auto rounded-md border border-white/[0.05] bg-ink-900/40 [.light_&]:bg-white">
      <div className="py-1">
        {tree.map((node) => (
          <TreeRow
            key={node.category.path}
            node={node}
            level={0}
            expanded={expanded}
            onToggleExpand={toggleExpand}
            onToggleSelect={toggleSelect}
            state={indeterminateOrChecked}
          />
        ))}
      </div>
    </div>
  );
}

function TreeRow({
  node,
  level,
  expanded,
  onToggleExpand,
  onToggleSelect,
  state,
}: {
  node: TreeNode;
  level: number;
  expanded: Set<string>;
  onToggleExpand: (path: string) => void;
  onToggleSelect: (node: TreeNode) => void;
  state: (node: TreeNode) => "off" | "on" | "partial";
}) {
  const hasChildren = node.children.length > 0;
  const isOpen = expanded.has(node.category.path);
  const s = state(node);

  return (
    <>
      <div
        className="group flex items-center gap-1.5 px-2 py-1.5 text-[12.5px] hover:bg-white/[0.03] [.light_&]:hover:bg-ink-900/[0.04]"
        style={{ paddingLeft: 8 + level * 14 }}
      >
        {hasChildren ? (
          <button
            onClick={() => onToggleExpand(node.category.path)}
            className="flex h-4 w-4 items-center justify-center text-ink-400 hover:text-ink-200"
            aria-label={isOpen ? "Collapse" : "Expand"}
          >
            {isOpen ? (
              <ChevronDown className="h-3.5 w-3.5" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5" />
            )}
          </button>
        ) : (
          <span className="inline-block h-4 w-4" />
        )}
        <button
          onClick={() => onToggleSelect(node)}
          className="flex flex-1 items-center gap-2 text-left"
        >
          <span
            className={cn(
              "flex h-4 w-4 items-center justify-center rounded-[4px] border transition",
              s === "on"
                ? "border-accent-400/80 bg-accent-500/80"
                : s === "partial"
                ? "border-accent-400/60 bg-accent-500/30"
                : "border-white/15 bg-ink-800",
            )}
            aria-checked={s === "on"}
            role="checkbox"
          >
            {s === "on" ? (
              <CheckSquare className="h-3 w-3 text-white" />
            ) : s === "partial" ? (
              <span className="h-0.5 w-2 rounded bg-accent-200" />
            ) : (
              <Square className="h-3 w-3 text-transparent" />
            )}
          </span>
          <span
            className={cn(
              "truncate",
              node.category.depth === 1 ? "font-medium text-ink-100" : "text-ink-200",
            )}
          >
            {node.category.name}
          </span>
        </button>
      </div>
      {isOpen &&
        node.children.map((child) => (
          <TreeRow
            key={child.category.path}
            node={child}
            level={level + 1}
            expanded={expanded}
            onToggleExpand={onToggleExpand}
            onToggleSelect={onToggleSelect}
            state={state}
          />
        ))}
    </>
  );
}
