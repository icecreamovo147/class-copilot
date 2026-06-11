import { create } from 'zustand';
import type { Cohort } from '@/types';

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
}));
