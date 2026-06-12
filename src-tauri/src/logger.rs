use std::path::Path;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// 日志写入器守卫，需在应用生命周期内保持存活以保证日志完整写入
#[allow(dead_code)]
pub struct LogGuard(WorkerGuard);

/// 初始化日志系统
///
/// - 控制台输出：开发模式 DEBUG 级别，生产模式 WARN 级别（带颜色）
/// - 文件输出：始终 INFO 级别，按天轮转，保留 7 天
/// - bridge `log` crate → `tracing`（兼容 sqlx 等使用 `log` 的依赖）
pub fn init_logger(log_dir: &Path) -> LogGuard {
    // ── 文件输出：按天轮转 ──
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "class-copilot.log");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // ── 控制台层 ──
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_line_number(cfg!(debug_assertions))
        .pretty()
        .with_ansi(true);

    // ── 文件层（无 ANSI 颜色码） ──
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_line_number(true)
        .with_ansi(false)
        .with_writer(non_blocking);

    // ── 日志级别过滤 ──
    // 优先读 RUST_LOG 环境变量，否则根据构建模式选择默认级别
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            // 开发环境：控制台 DEBUG，文件 INFO
            EnvFilter::new("debug")
        } else {
            // 生产环境：INFO
            EnvFilter::new("info")
        }
    });

    // ── 组装 subscriber ──
    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    // ── 桥接 log → tracing（让 sqlx 等 crate 的日志也能被捕获） ──
    tracing_log::LogTracer::init().expect("Failed to init LogTracer");

    LogGuard(guard)
}
