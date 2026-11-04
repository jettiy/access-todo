// REST client for the local api-server.
// Each post-it window reads its Tauri label to know which agent it represents.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

const BASE = (import.meta.env.VITE_API_BASE as string) || "http://127.0.0.1:7878";
export const API_BASE = BASE;

// ORDER-004: by default all HTTP goes through the Rust `api_call` command
// (reqwest) instead of WebView fetch — the browser socket pool is never
// used, so N windows can't exhaust it. VITE_USE_PROXY=0 빌드로 직접 fetch
// 경로(폴백)를 복원할 수 있다. 브라우저 개발 모드(vite dev)에서는 invoke가
// 없으므로 자동으로 fetch로 동작한다.
export const USE_PROXY = import.meta.env.VITE_USE_PROXY !== "0";

export const IN_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Tauri 창 label이 에이전트 식별자 (hermes/omp/zcode/user).
// dev 환경(브라우저)에서는 URL 쿼리 ?agent= 사용.
export function getWindowAgent(): string {
  try {
    const label = getCurrentWindow().label;
    if (label && label !== "postit" && label !== "main") return label;
  } catch {
    /* not in tauri */
  }
  const params = new URLSearchParams(window.location.search);
  return params.get("agent") || "user";
}

export const AGENT = getWindowAgent();

// 에이전트별 표시 정보
export const AGENT_META: Record<string, { emoji: string; title: string; bg: string; bgSoft: string; border: string; text: string }> = {
  hermes: { emoji: "🤖", title: "Hermes", bg: "#ffd0e8", bgSoft: "#ffe5f2", border: "#c46b9c", text: "#7a2a5a" },
  omp: { emoji: "🛠️", title: "OMP", bg: "#c8e0f5", bgSoft: "#dceaf5", border: "#3c7ab0", text: "#1a3a5a" },
  zcode: { emoji: "⚡", title: "ZCode", bg: "#c8f0e0", bgSoft: "#dcf5ec", border: "#3ca87e", text: "#1a5a44" },
  user: { emoji: "👤", title: "내 할 일", bg: "#fff0a0", bgSoft: "#fff8cc", border: "#b89b00", text: "#5a4a00" },
  codex: { emoji: "🧠", title: "Codex", bg: "#e0d0f5", bgSoft: "#ede2fa", border: "#7a4ab0", text: "#3a1a5a" },
  dsh: { emoji: "🐋", title: "DSH", bg: "#ffd8c8", bgSoft: "#ffe8dd", border: "#b06a3c", text: "#5a2a1a" },
};

export type Priority = "high" | "medium" | "low";

export interface Category {
  id: string;
  agent: string;
  name: string;
  order: number;
}

export interface Todo {
  id: string;
  title: string;
  note?: string | null;
  done: boolean;
  priority: Priority;
  due_date?: string | null;
  tags: string[];
  category_id?: string | null;
  created_at: string;
  created_by: string;
  completed_at?: string | null;
  completed_by?: string | null;
  updated_at?: string | null;
  updated_by?: string | null;
}

export type NoteType = "worklog" | "retrospect" | "issue" | "playtest";
export type GameSystem = "combat" | "inventory" | "quest" | "skill" | "ui" | "audio" | "map" | "ai" | "save" | "network" | "etc";

export interface DevNote {
  id: string;
  note_type: NoteType;
  title: string;
  body: string;
  game_system?: GameSystem | null;
  version?: string | null;
  linked_todos: string[];
  tags: string[];
  created_at: string;
  created_by: string;
  updated_at?: string | null;
  updated_by?: string | null;
}

export interface WorklogResult {
  created: boolean;
  note_id: string;
  todos_counted: number;
  systems: string[];
}

async function req<T>(path: string, init: RequestInit = {}): Promise<T> {
  const method = (init.method || "GET").toUpperCase();
  const body = init.body ? JSON.parse(String(init.body)) : undefined;
  if (USE_PROXY && IN_TAURI) {
    // invoke 실패는 그대로 전파(중복 실행 방지) — 브라우저 dev에선 fetch 경로
    return invoke<T>("api_call", { agent: AGENT, method, path, body });
  }
  const r = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { "X-Agent": AGENT, "Content-Type": "application/json", ...(init.headers || {}) },
  });
  if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
  return r.json() as Promise<T>;
}

export interface LatestSummary {
  title: string;
  summary: string;
  completed_at?: string | null;
  completed_by?: string | null;
}

export interface ListResult {
  todos: Todo[];
  categories: Category[];
  latest_summary?: LatestSummary | null;
}

export const api = {
  // 자기 에이전트의 할 일 + 카테고리 (+ 최신 작업 요약) 조회
  list: () => req<ListResult>(`/todos?agent=${AGENT}`),
  today: () => req<{ todos: Todo[] }>(`/todos/today`),
  add: (title: string, note?: string, priority: Priority = "medium", due_date?: string) =>
    req<Todo>("/todos", { method: "POST", body: JSON.stringify({ title, note, priority, due_date }) }),
  addRaw: (title: string, note?: string, priority: Priority = "medium", due_date?: string, tags?: string[], category_id?: string) =>
    req<Todo>("/todos", { method: "POST", body: JSON.stringify({ title, note, priority, due_date, tags, category_id }) }),
  toggle: (id: string) => req<Todo>(`/todos/${id}/toggle`, { method: "POST" }),
  del: (id: string) => req<{ deleted: string }>(`/todos/${id}`, { method: "DELETE" }),
  // 카테고리
  addCategory: (name: string) =>
    req<Category>("/categories", { method: "POST", body: JSON.stringify({ agent: AGENT, name }) }),
  renameCategory: (id: string, name: string) =>
    req<Category>(`/categories/${id}`, { method: "PATCH", body: JSON.stringify({ name }) }),
  deleteCategory: (id: string) =>
    req<{ deleted: string }>(`/categories/${id}`, { method: "DELETE" }),
  reorderCategories: (orderedIds: string[]) =>
    req<{ ok: boolean }>("/categories/reorder", { method: "POST", body: JSON.stringify({ agent: AGENT, ordered_ids: orderedIds }) }),
  sync: () => req<{ ok: boolean }>("/sync", { method: "POST" }),
};

export const devnoteApi = {  list: (params?: { note_type?: NoteType; game_system?: GameSystem; version?: string }) => {
    const qs = new URLSearchParams();
    if (params?.note_type) qs.set("note_type", params.note_type);
    if (params?.game_system) qs.set("game_system", params.game_system);
    if (params?.version) qs.set("version", params.version);
    const query = qs.toString();
    return req<{ devnotes: DevNote[] }>(`/devnotes${query ? "?" + query : ""}`);
  },
  timeline: (tag?: string) => {
    const qs = tag ? `?tag=${encodeURIComponent(tag)}` : "";
    return req<{ devnotes: DevNote[] }>(`/devnotes/timeline${qs}`);
  },
  classifyUntagged: () =>
    req<{ classified: number; fallback: number; skipped: number; error?: string }>("/devnotes/classify-untagged", { method: "POST" }),
  byVersion: (ver: string) => req<{ devnotes: DevNote[]; version: string }>(`/devnotes/by-version/${ver}`),
  create: (data: { note_type: NoteType; title: string; body: string; game_system?: GameSystem; version?: string; tags?: string[] }) =>
    req<DevNote>("/devnotes", { method: "POST", body: JSON.stringify(data) }),
  update: (id: string, data: { title?: string; body?: string; game_system?: GameSystem | null; version?: string | null }) =>
    req<DevNote>(`/devnotes/${id}`, { method: "PATCH", body: JSON.stringify(data) }),
  del: (id: string) => req<{ deleted: string }>(`/devnotes/${id}`, { method: "DELETE" }),
  generateWorklog: (date?: string) =>
    req<WorklogResult>("/devnotes/worklog/generate", { method: "POST", body: JSON.stringify({ date }) }),
};

// ── 설정창(P1): settings + agents API ──

export interface AgentRow {
  id: string;
  display: string;
  integration: "mcp" | "rest" | string;
  experimental: boolean;
  installed: boolean;
  config_path: string;
  registered: boolean;
  enabled: boolean;
}

export interface SettingsInfo {
  pilot_level: string; // beginner | intermediate | advanced | auto
  sync_mode: string;   // gist | local
  enabled_agents: string[];
  gist_sync: boolean;
  valid_levels: string[];
  // 작업 공간 (게임/프로젝트 모드)
  workspace_mode: "game" | "project" | string;
  active_game: string | null;
  active_project: string | null;
  games: string[];
  projects: string[];
}

export interface WorkspacePatch {
  workspace_mode?: string;
  active_game?: string | null;
  active_project?: string | null;
  games?: string[];
  projects?: string[];
}

export interface AgentAction {
  id: string;
  outcome: string;
  error?: string | null;
  settings_saved: boolean;
}

export const settingsApi = {
  get: () => req<SettingsInfo>("/settings"),
  put: (patch: { pilot_level?: string; sync_mode?: string; enabled_agents?: string[] } & WorkspacePatch) =>
    req<{ ok: boolean; settings: SettingsInfo }>("/settings", { method: "PUT", body: JSON.stringify(patch) }),
};

export const agentsApi = {
  list: () => req<{ agents: AgentRow[] }>("/agents"),
  register: (id: string) =>
    req<AgentAction>(`/agents/${encodeURIComponent(id)}/register`, { method: "POST" }),
  unregister: (id: string) =>
    req<AgentAction>(`/agents/${encodeURIComponent(id)}/unregister`, { method: "POST" }),
};
