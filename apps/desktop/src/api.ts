// REST client for the local api-server.
// Each post-it window reads its Tauri label to know which agent it represents.

import { getCurrentWindow } from "@tauri-apps/api/window";

const BASE = (import.meta.env.VITE_API_BASE as string) || "http://127.0.0.1:7878";

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
};

export type Priority = "high" | "medium" | "low";

export interface Todo {
  id: string;
  title: string;
  note?: string | null;
  done: boolean;
  priority: Priority;
  due_date?: string | null;
  tags: string[];
  created_at: string;
  created_by: string;
  completed_at?: string | null;
  completed_by?: string | null;
  updated_at?: string | null;
  updated_by?: string | null;
}

async function req<T>(path: string, init: RequestInit = {}): Promise<T> {
  const r = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { "X-Agent": AGENT, "Content-Type": "application/json", ...(init.headers || {}) },
  });
  if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
  return r.json() as Promise<T>;
}

export const api = {
  // 자기 에이전트의 할 일만 조회
  list: () => req<{ todos: Todo[] }>(`/todos?agent=${AGENT}`),
  today: () => req<{ todos: Todo[] }>(`/todos/today`),
  add: (title: string, note?: string, priority: Priority = "medium", due_date?: string) =>
    req<Todo>("/todos", { method: "POST", body: JSON.stringify({ title, note, priority, due_date }) }),
  addRaw: (title: string, note?: string, priority: Priority = "medium", due_date?: string, tags?: string[]) =>
    req<Todo>("/todos", { method: "POST", body: JSON.stringify({ title, note, priority, due_date, tags }) }),
  toggle: (id: string) => req<Todo>(`/todos/${id}/toggle`, { method: "POST" }),
  del: (id: string) => req<{ deleted: string }>(`/todos/${id}`, { method: "DELETE" }),
};
