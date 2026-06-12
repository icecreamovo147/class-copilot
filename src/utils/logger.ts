import { invoke } from '@tauri-apps/api/core';

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

/**
 * 将前端日志发送到 Rust 后端日志系统
 */
async function sendToBackend(level: LogLevel, message: string): Promise<void> {
  try {
    await invoke('log_frontend', { level, message });
  } catch {
    // 后端不可用时静默忽略（如开发阶段后端未启动）
  }
}

/**
 * 前端日志工具
 *
 * 同时输出到浏览器控制台和 Rust 后端日志文件，确保前后端日志统一管理。
 *
 * @example
 * import { logger } from '@/utils/logger';
 * logger.info('Dashboard 页面加载完成');
 * logger.error('数据加载失败', { cohortId: 1 });
 */
export const logger = {
  debug: (message: string, context?: Record<string, unknown>) => {
    const full = context ? `${message} ${JSON.stringify(context)}` : message;
    console.debug(`[DEBUG] ${full}`);
    sendToBackend('debug', full);
  },

  info: (message: string, context?: Record<string, unknown>) => {
    const full = context ? `${message} ${JSON.stringify(context)}` : message;
    console.info(`[INFO] ${full}`);
    sendToBackend('info', full);
  },

  warn: (message: string, context?: Record<string, unknown>) => {
    const full = context ? `${message} ${JSON.stringify(context)}` : message;
    console.warn(`[WARN] ${full}`);
    sendToBackend('warn', full);
  },

  error: (message: string, error?: unknown) => {
    const detail = error instanceof Error ? error.stack || error.message : String(error ?? '');
    const full = detail ? `${message}\n${detail}` : message;
    console.error(`[ERROR] ${full}`);
    sendToBackend('error', full);
  },
};
