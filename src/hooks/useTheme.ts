import { useState, useEffect } from 'react';
import { useAppStore } from '@/app/store';
import { getCurrentWindow } from '@tauri-apps/api/window';

function readSystemDark(): boolean {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

/**
 * 监听系统偏好并计算最终是否为暗色模式。
 *
 * 核心问题：Tauri 的 window.setTheme('dark') 会锁定窗口主题，导致
 * webview 内 matchMedia('prefers-color-scheme') 永久返回锁定值，
 * 即使之后切回 "跟随系统" 模式，matchMedia 仍然被污染。
 *
 * 修复方案：
 * 1. 使用 Tauri onThemeChanged 追踪系统级主题（不受 setTheme 影响）
 * 2. 组件挂载时记录 initialSystemDark（此时尚未调用 setTheme）
 * 3. 切回 auto 时立即回退到 initialSystemDark，再用 rAF 延迟重读
 */
export function useIsDark(): boolean {
  const themeMode = useAppStore((s) => s.themeMode);

  // 挂载时捕获的系统偏好——此时尚未调用任何 setTheme，保证是真实的
  const [initialSystemDark] = useState(readSystemDark);
  const [systemDark, setSystemDark] = useState(initialSystemDark);

  // 方案 A：Tauri onThemeChanged — 仅响应系统级主题变更，不受 setTheme 污染
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onThemeChanged(({ payload: theme }) => {
        setSystemDark(theme === 'dark');
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // 非 Tauri 环境，降级到 matchMedia 监听
      });
    return () => {
      unlisten?.();
    };
  }, []);

  // 方案 B：matchMedia 监听 — 作为非 Tauri 环境（浏览器开发）的兜底
  useEffect(() => {
    if (!window.matchMedia) return;
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  // 切回 auto 时：① 立刻回退到挂载时的真实系统值（避免一帧闪烁）
  //              ② 延迟一帧重读（等 setTheme(null) 解锁窗口后生效）
  useEffect(() => {
    if (themeMode === 'auto') {
      setSystemDark(initialSystemDark);
      const raf = requestAnimationFrame(() => {
        setSystemDark(readSystemDark());
      });
      return () => cancelAnimationFrame(raf);
    }
  }, [themeMode, initialSystemDark]);

  if (themeMode === 'dark') return true;
  if (themeMode === 'light') return false;
  return systemDark;
}
