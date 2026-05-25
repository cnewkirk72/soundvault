import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

/**
 * Middle-ellipsis truncation for paths.
 * "/Users/christian/Music/Ableton/Projects/My Track/My Track.als"
 * -> "/Users/christian/.../My Track/My Track.als"
 */
export function truncateMiddle(path: string, max = 60): string {
  if (path.length <= max) return path;
  const sep = path.includes("/") ? "/" : "\\";
  const parts = path.split(sep);
  if (parts.length <= 3) {
    const head = path.slice(0, Math.floor(max / 2) - 2);
    const tail = path.slice(path.length - Math.floor(max / 2) + 1);
    return `${head}…${tail}`;
  }
  const head = parts.slice(0, 2).join(sep);
  const tail = parts.slice(-2).join(sep);
  const combined = `${head}${sep}…${sep}${tail}`;
  if (combined.length <= max) return combined;
  const overflow = combined.length - max;
  const trimmedHead = head.slice(0, Math.max(2, head.length - overflow));
  return `${trimmedHead}${sep}…${sep}${tail}`;
}

export function formatCount(n: number): string {
  return n.toLocaleString();
}

export function basename(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(i + 1) : path;
}
