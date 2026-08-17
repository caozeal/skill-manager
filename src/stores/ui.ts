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
