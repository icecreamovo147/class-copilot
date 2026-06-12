import React, { useEffect } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ConfigProvider, theme } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import dayjs from 'dayjs';
import 'dayjs/locale/zh-cn';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAppStore } from '@/app/store';
import { useIsDark } from '@/hooks/useTheme';

dayjs.locale('zh-cn');

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
      staleTime: 30_000,
    },
  },
});

interface AppProvidersProps {
  children: React.ReactNode;
}

export function AppProviders({ children }: AppProvidersProps) {
  const isDark = useIsDark();
  const themeMode = useAppStore((s) => s.themeMode);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', isDark ? 'dark' : 'light');

    // auto 模式：解锁窗口主题，让 webview 跟随真实系统偏好
    // 手动模式：强制指定窗口主题
    if (themeMode === 'auto') {
      getCurrentWindow().setTheme(null).catch(() => {});
    } else {
      getCurrentWindow().setTheme(themeMode).catch(() => {});
    }
  }, [isDark, themeMode]);

  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: isDark ? theme.darkAlgorithm : undefined,
        token: {
          colorPrimary: '#1677ff',
          borderRadius: 6,
        },
      }}
    >
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </ConfigProvider>
  );
}
