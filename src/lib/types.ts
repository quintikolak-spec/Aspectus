export type WidgetKind =
  | "clock"
  | "weather"
  | "news"
  | "system"
  | "calendar"
  | "media"
  | "source";

export interface WidgetLayout {
  id: string;
  kind: WidgetKind;
  x: number;
  y: number;
  w: number;
  h: number;
  sourceId?: string; // set when kind === "source"
  hiddenProcesses?: string[]; // used by the system widget's process list
}

export interface Interest {
  id: string;
  label: string;
  enabled: boolean;
}

export type InfoFilterLevel = "all" | "relevant" | "important" | "critical";

export type OperatingMode = "normal" | "sparsam" | "eco";

export interface Source {
  id: string;
  url: string;
  title: string;
  category: string;
  kind: "rss" | "html";
  lastCheckedAt: string | null;
  lastUpdatedAt: string | null;
  healthy: boolean;
  lastError?: string;
}

export interface NewsCard {
  id: string;
  sourceId: string;
  sourceTitle: string;
  originalUrl: string;
  headline: string;
  summary: string;
  category: string;
  importance: number; // 0..1, produced by the AI relevance step
  publishedAt: string | null;
  fetchedAt: string;
}

export interface ProcessInfo {
  name: string;
  cpuPercent: number;
  memoryMb: number;
}

export interface SystemStats {
  cpuPercent: number;
  ramPercent: number;
  gpuPercent: number | null;
  systemLoadPercent: number;
  diskFreeGb: number;
  batteryPercent: number | null;
  tempsC: Record<string, number>;
  topProcesses: ProcessInfo[];
}

export interface WeatherSnapshot {
  locationName: string;
  tempC: number;
  condition: string;
  forecast: { day: string; high: number; low: number; condition: string }[];
}

export interface AppSettings {
  mode: OperatingMode;
  theme: "dark" | "light" | "system";
  cornerStyle: "sharp" | "soft" | "round";
  accentColor: string;
  interests: Interest[];
  infoFilter: InfoFilterLevel;
  layout: WidgetLayout[];
  launchOnStartup: boolean;
}
