// REST client for the local api-server.

const BASE = (import.meta.env.VITE_API_BASE as string) || "http://127.0.0.1:7878";
const AGENT = "user";

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
  list: () => req<{ todos: Todo[] }>("/todos"),
  today: () => req<{ todos: Todo[] }>("/todos/today"),
  add: (title: string, note?: string, priority: Priority = "medium", due_date?: string) =>
    req<Todo>("/todos", { method: "POST", body: JSON.stringify({ title, note, priority, due_date }) }),
  addRaw: (title: string, note?: string, priority: Priority = "medium", due_date?: string, tags?: string[]) =>
    req<Todo>("/todos", { method: "POST", body: JSON.stringify({ title, note, priority, due_date, tags }) }),
  toggle: (id: string) => req<Todo>(`/todos/${id}/toggle`, { method: "POST" }),
  del: (id: string) => req<{ deleted: string }>(`/todos/${id}`, { method: "DELETE" }),
};
