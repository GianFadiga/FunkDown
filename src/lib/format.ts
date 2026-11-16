export function formatSpeed(bytesPerSec: number | null): string {
  if (!bytesPerSec) return "";
  const mb = bytesPerSec / 1_048_576;
  return mb >= 1 ? `${mb.toFixed(1)} MB/s` : `${Math.max(1, Math.round(bytesPerSec / 1024))} kB/s`;
}

export function formatBytes(bytes: number): string {
  const mb = bytes / 1_048_576;
  return mb >= 100 ? `${Math.round(mb)} MB` : `${mb.toFixed(1)} MB`;
}

/** 75 -> "1:15", 3725 -> "1:02:05" */
export function formatClock(totalSeconds: number | null): string {
  if (totalSeconds == null) return "";
  const s = Math.max(0, Math.round(totalSeconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`;
}

export function formatDate(epochSeconds: number, locale: string): string {
  return new Date(epochSeconds * 1000).toLocaleString(locale, {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export const PLATFORM_LABEL = {
  youtube: "YouTube",
  instagram: "Instagram",
  tiktok: "TikTok",
  facebook: "Facebook",
  other: "Web",
} as const;
