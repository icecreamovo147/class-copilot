import { create } from 'zustand';
import type { Cohort } from '@/types';

export type ThemeMode = 'light' | 'dark' | 'auto';

const STORAGE_KEY = 'theme-mode';

function loadThemeMode(): ThemeMode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'light' || stored === 'dark' || stored === 'auto') {
      return stored;
    }
  } catch {
    // localStorage 不可用时回退默认值
  }
  return 'auto';
}

function saveThemeMode(mode: ThemeMode) {
  try {
    localStorage.setItem(STORAGE_KEY, mode);
  } catch {
    // 静默失败
  }
}

interface AppState {
  // 当前届次
  currentCohort: Cohort | null;
  setCurrentCohort: (cohort: Cohort | null) => void;

  // 所有届次列表缓存
  cohorts: Cohort[];
  setCohorts: (cohorts: Cohort[]) => void;

  // 侧边栏折叠
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;

  // 全局只读状态
  isReadonly: boolean;
  setIsReadonly: (readonly: boolean) => void;

  // 版本信息
  appVersion: string;
  setAppVersion: (version: string) => void;

  // 主题模式
  themeMode: ThemeMode;
  setThemeMode: (mode: ThemeMode) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentCohort: null,
  setCurrentCohort: (cohort) =>
    set({
      currentCohort: cohort,
      isReadonly: cohort?.status === '已归档',
    }),

  cohorts: [],
  setCohorts: (cohorts) => set({ cohorts }),

  sidebarCollapsed: false,
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

  isReadonly: false,
  setIsReadonly: (readonly) => set({ isReadonly: readonly }),

  appVersion: '1.0.0',
  setAppVersion: (version) => set({ appVersion: version }),

  themeMode: loadThemeMode(),
  setThemeMode: (mode) => {
    saveThemeMode(mode);
    set({ themeMode: mode });
  },
}));
