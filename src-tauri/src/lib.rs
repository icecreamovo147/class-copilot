pub mod commands;
pub mod db;
mod logger;

use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};
use tokio::sync::Mutex;

pub struct AppState {
    pub db: SqlitePool,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

const LEGACY_APP_IDENTIFIER: &str = "com.class-copilot.app";

fn copy_dir_if_missing(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(target).map_err(|e| format!("创建迁移目录失败: {}", e))?;
    for entry in std::fs::read_dir(source).map_err(|e| format!("读取旧数据目录失败: {}", e))?
    {
        let entry = entry.map_err(|e| format!("读取旧数据目录项失败: {}", e))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = entry
            .file_type()
            .map_err(|e| format!("读取旧数据目录项类型失败: {}", e))?;

        if metadata.is_dir() {
            copy_dir_if_missing(&source_path, &target_path)?;
            continue;
        }

        if !target_path.exists() {
            std::fs::copy(&source_path, &target_path)
                .map_err(|e| format!("复制旧数据文件失败: {}", e))?;
        }
    }
    Ok(())
}

fn ensure_legacy_app_data_migrated(current_dir: &Path) -> Result<(), String> {
    let Some(parent_dir) = current_dir.parent() else {
        return Ok(());
    };
    let legacy_dir = parent_dir.join(LEGACY_APP_IDENTIFIER);
    if legacy_dir == current_dir || !legacy_dir.exists() {
        return Ok(());
    }

    let needs_migration = !current_dir.exists()
        || std::fs::read_dir(current_dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true);
    if !needs_migration {
        return Ok(());
    }

    tracing::info!(
        legacy_dir = %legacy_dir.display(),
        current_dir = %current_dir.display(),
        "检测到旧版应用数据目录，开始迁移"
    );
    copy_dir_if_missing(&legacy_dir, current_dir)?;
    tracing::info!("旧版应用数据目录迁移完成");
    Ok(())
}

pub(crate) fn resolve_app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let current_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    ensure_legacy_app_data_migrated(&current_dir)?;
    std::fs::create_dir_all(&current_dir).map_err(|e| format!("创建应用数据目录失败: {}", e))?;
    Ok(current_dir)
}

/// 前端调用：将前端日志写入后端日志系统
#[tauri::command]
fn log_frontend(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(target: "frontend", "{message}"),
        "warn" => tracing::warn!(target: "frontend", "{message}"),
        "info" => tracing::info!(target: "frontend", "{message}"),
        "debug" => tracing::debug!(target: "frontend", "{message}"),
        _ => tracing::info!(target: "frontend", "{message}"),
    }
}

/// 前端调用：获取日志目录路径
#[tauri::command]
fn get_log_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = resolve_app_data_dir(&app)?.join("logs");
    Ok(dir.to_string_lossy().to_string())
}

/// 前端调用：在文件管理器中打开日志目录
#[tauri::command]
fn open_log_dir(app: tauri::AppHandle) -> Result<(), String> {
    let dir = resolve_app_data_dir(&app)?.join("logs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // ── 日志系统初始化（最早执行） ──
            let app_data_dir =
                resolve_app_data_dir(&app.handle()).expect("Failed to get app data dir");
            let log_dir = app_data_dir.join("logs");
            std::fs::create_dir_all(&log_dir).ok();
            let log_guard = logger::init_logger(&log_dir);

            std::fs::create_dir_all(&app_data_dir).ok();

            // ── 启动信息 ──
            tracing::info!("══════════════════════════════════════════");
            tracing::info!("  数字化班级事务管理系统 v{}", env!("CARGO_PKG_VERSION"));
            tracing::info!("══════════════════════════════════════════");
            tracing::info!(platform = %std::env::consts::OS, "平台信息");
            tracing::info!(data_dir = %app_data_dir.display(), "数据目录");
            tracing::info!(log_dir = %log_dir.display(), "日志目录");
            #[cfg(debug_assertions)]
            tracing::info!("开发服务器: http://localhost:1420");
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            // ── 数据库初始化 ──
            let db_path = app_data_dir.join("class_management.db");
            tracing::info!(path = %db_path.display(), "正在初始化数据库...");

            let pool = tauri::async_runtime::block_on(async {
                db::init_db(&db_path)
                    .await
                    .expect("Failed to initialize database")
            });

            tracing::info!("✓ 数据库初始化完成 (WAL 模式, 最大 5 连接)");

            let state = AppState {
                db: pool,
                app_handle: Arc::new(Mutex::new(Some(app.handle().clone()))),
            };

            app.manage(state);
            app.manage(log_guard);

            // ── 系统托盘 ──
            let show_item = MenuItemBuilder::with_id("show", "显示主界面").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出程序").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(
                    Image::from_bytes(include_bytes!("../icons/32x32.png"))
                        .expect("Failed to load tray icon"),
                )
                .menu(&tray_menu)
                .tooltip("数字化班级事务管理系统")
                .on_menu_event(|app_handle, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        tracing::info!("用户通过托盘菜单退出程序");
                        app_handle.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            tracing::info!("✓ 插件已加载 (dialog, fs, shell)");
            tracing::info!("✓ 系统托盘已创建");
            tracing::info!("══════════════════════════════════════════");
            tracing::info!("  应用启动完成，等待用户操作...");
            tracing::info!("══════════════════════════════════════════");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();

                #[cfg(target_os = "macos")]
                {
                    tracing::info!("窗口关闭 → 隐藏到 Dock / 菜单栏托盘");
                    let _ = window.hide();
                }

                #[cfg(not(target_os = "macos"))]
                {
                    let handle = window.app_handle().clone();
                    let label = window.label().to_string();

                    tauri::async_runtime::spawn(async move {
                        use tauri_plugin_dialog::DialogExt;

                        let hide_to_tray = handle
                            .dialog()
                            .ask(
                                "点击「确定」最小化到系统托盘，点击「取消」退出程序",
                                "关闭窗口",
                            )
                            .await;

                        if hide_to_tray {
                            tracing::info!("窗口关闭 → 最小化到系统托盘");
                            if let Some(win) = handle.get_webview_window(&label) {
                                let _ = win.hide();
                            }
                        } else {
                            tracing::info!("用户选择退出程序");
                            handle.exit(0);
                        }
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            log_frontend,
            get_log_dir,
            open_log_dir,
            // Cohort
            commands::cohort::get_cohorts,
            commands::cohort::get_cohort,
            commands::cohort::get_current_cohort,
            commands::cohort::create_cohort,
            commands::cohort::update_cohort,
            commands::cohort::archive_cohort,
            commands::cohort::unarchive_cohort,
            commands::cohort::set_current_cohort,
            // Student
            commands::student::get_students,
            commands::student::get_all_students,
            commands::student::get_student,
            commands::student::create_student,
            commands::student::update_student,
            commands::student::delete_student,
            commands::student::preview_students_excel,
            commands::student::import_students_excel,
            commands::student::export_students_excel,
            // Subject
            commands::subject::get_subjects,
            commands::subject::create_subject,
            commands::subject::update_subject,
            commands::subject::delete_subject,
            // Homework
            commands::homework::get_homeworks,
            commands::homework::get_homework,
            commands::homework::create_homework,
            commands::homework::update_homework,
            commands::homework::delete_homework,
            commands::homework::open_homework_attachment,
            commands::homework::get_homework_records,
            commands::homework::get_student_homework_records,
            commands::homework::update_homework_record,
            commands::homework::batch_update_homework_records,
            commands::homework::export_incomplete_homework,
            // Attendance
            commands::attendance::get_attendance_by_date,
            commands::attendance::save_attendance,
            commands::attendance::set_all_attendance_normal,
            commands::attendance::query_attendance,
            commands::attendance::attendance_statistics,
            commands::attendance::attendance_statistics_cohort,
            commands::attendance::export_attendance_excel,
            // Exam & Score
            commands::exam::get_exams,
            commands::exam::create_exam,
            commands::exam::update_exam,
            commands::exam::delete_exam,
            commands::exam::get_exam_subject_configs,
            commands::exam::save_exam_subject_configs,
            commands::exam::get_scores_by_exam,
            commands::exam::save_scores,
            commands::exam::preview_scores_excel,
            commands::exam::import_scores_excel,
            commands::exam::export_scores_excel,
            commands::exam::score_statistics,
            commands::exam::score_rankings,
            // Affairs
            commands::affair::get_notices,
            commands::affair::create_notice,
            commands::affair::update_notice,
            commands::affair::delete_notice,
            commands::affair::export_notices_excel,
            commands::affair::get_duties,
            commands::affair::create_duty,
            commands::affair::update_duty,
            commands::affair::delete_duty,
            commands::affair::export_duties_excel,
            commands::affair::get_behavior_records,
            commands::affair::create_behavior_record,
            commands::affair::delete_behavior_record,
            commands::affair::get_class_fee_records,
            commands::affair::create_class_fee_record,
            commands::affair::update_class_fee_record,
            commands::affair::delete_class_fee_record,
            commands::affair::export_class_fee_excel,
            // Statistics & Dashboard
            commands::stats::get_dashboard_stats,
            commands::stats::homework_statistics,
            commands::stats::homework_trend_statistics,
            commands::stats::score_statistics_cohort,
            commands::stats::attendance_trend_statistics,
            commands::stats::score_trend_statistics,
            commands::stats::cross_cohort_comparison,
            commands::stats::export_cross_cohort_comparison,
            commands::stats::export_cross_cohort_comparison_pdf,
            commands::stats::export_cohort_statistics_excel,
            commands::stats::export_cohort_statistics_pdf,
            commands::stats::get_student_profile,
            commands::stats::export_student_growth_archive,
            commands::stats::export_student_growth_archive_pdf,
            // Backup & Config
            commands::backup::create_backup,
            commands::backup::restore_backup,
            commands::backup::export_cohort,
            commands::config::get_config,
            commands::config::set_config,
            commands::config::get_settings_overview,
            commands::config::save_settings_preferences,
            commands::config::get_recent_backups,
            commands::config::download_template,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // ── 运行事件循环 ──
    app.run(|app_handle, event| {
        if let tauri::RunEvent::Reopen { .. } = event {
            tracing::info!("Dock 图标点击 → 恢复主窗口");
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::ensure_legacy_app_data_migrated;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("class_copilot_{name}_{suffix}"))
    }

    #[test]
    fn migrates_legacy_data_when_current_dir_is_missing() {
        let root = temp_test_dir("legacy_migration_missing");
        let legacy_dir = root.join("com.class-copilot.app");
        let current_dir = root.join("com.class-copilot");
        let nested_legacy_dir = legacy_dir.join("attachments").join("homework");
        fs::create_dir_all(&nested_legacy_dir).unwrap();
        fs::write(legacy_dir.join("class_management.db"), b"db").unwrap();
        fs::write(nested_legacy_dir.join("sample.txt"), b"attachment").unwrap();

        ensure_legacy_app_data_migrated(&current_dir).unwrap();

        assert_eq!(
            fs::read(current_dir.join("class_management.db")).unwrap(),
            b"db"
        );
        assert_eq!(
            fs::read(current_dir.join("attachments/homework/sample.txt")).unwrap(),
            b"attachment"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_legacy_copy_when_current_dir_already_has_data() {
        let root = temp_test_dir("legacy_migration_skip");
        let legacy_dir = root.join("com.class-copilot.app");
        let current_dir = root.join("com.class-copilot");
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::create_dir_all(&current_dir).unwrap();
        fs::write(legacy_dir.join("class_management.db"), b"legacy").unwrap();
        fs::write(current_dir.join("class_management.db"), b"current").unwrap();

        ensure_legacy_app_data_migrated(&current_dir).unwrap();

        assert_eq!(
            fs::read(current_dir.join("class_management.db")).unwrap(),
            b"current"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
