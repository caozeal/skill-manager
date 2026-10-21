import { create } from "zustand";
import type { Skill } from "@/types";

interface DrawerState {
  skill: Skill | null;
  open: (skill: Skill) => void;
  close: () => void;
}

interface SearchState {
  query: string;
  setQuery: (q: string) => void;
}

export const useDrawerStore = create<DrawerState>((set) => ({
  skill: null,
  open: (skill) => set({ skill }),
  close: () => set({ skill: null }),
}));

export const useSearchStore = create<SearchState>((set) => ({
  query: "",
  setQuery: (query) => set({ query }),
}));

/**
 * 应用内确认弹窗（替代 window.confirm）。
 * Tauri 的 WKWebView 未实现 WKUIDelegate 的 confirm 面板，
 * window.confirm 会不弹窗直接返回 false，导致所有带确认的操作「点了没反应」。
 */
interface ConfirmOptions {
  title: string;
  message: string;
  confirmText?: string;
  danger?: boolean;
}

interface ConfirmState {
  current: (ConfirmOptions & { resolve: (ok: boolean) => void }) | null;
  confirm: (opts: ConfirmOptions) => Promise<boolean>;
  answer: (ok: boolean) => void;
}

export const useConfirmStore = create<ConfirmState>((set, get) => ({
  current: null,
  confirm: (opts) =>
    new Promise<boolean>((resolve) => {
      // 已有待确认弹窗时，前一个按取消处理
      get().current?.resolve(false);
      set({ current: { ...opts, resolve } });
    }),
  answer: (ok) => {
    const cur = get().current;
    if (cur) {
      cur.resolve(ok);
      set({ current: null });
    }
  },
}));

/** 轻量 toast：mutation 失败等需要可见反馈的场景 */
export interface ToastItem {
  id: number;
  text: string;
  kind: "error" | "info";
}

interface ToastState {
  toasts: ToastItem[];
  push: (text: string, kind?: ToastItem["kind"]) => void;
  dismiss: (id: number) => void;
}

let toastSeq = 1;

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],
  push: (text, kind = "error") => {
    const id = toastSeq++;
    set({ toasts: [...get().toasts, { id, text, kind }] });
    window.setTimeout(() => get().dismiss(id), 4000);
  },
  dismiss: (id) =>
    set({ toasts: get().toasts.filter((t) => t.id !== id) }),
}));
