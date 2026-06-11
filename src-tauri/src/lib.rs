mod commands;
mod db;

use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: SqlitePool,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 获取应用数据目录
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).ok();

            let db_path = app_data_dir.join("class_management.db");
            log::info!("Database path: {:?}", db_path);

            // 初始化数据库（同步方式）
            let pool = tauri::async_runtime::block_on(async {
                db::init_db(&db_path).await.expect("Failed to initialize database")
            });

            let state = AppState {
                db: pool,
                app_handle: Arc::new(Mutex::new(Some(app.handle().clone()))),
            };

            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            commands::homework::get_homework_records,
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
            commands::exam::get_scores_by_exam,
            commands::exam::save_scores,
            commands::exam::import_scores_excel,
            commands::exam::score_statistics,
            commands::exam::score_rankings,
            // Affairs
            commands::affair::get_notices,
            commands::affair::create_notice,
            commands::affair::update_notice,
            commands::affair::delete_notice,
            commands::affair::get_duties,
            commands::affair::create_duty,
            commands::affair::update_duty,
            commands::affair::delete_duty,
            commands::affair::get_behavior_records,
            commands::affair::create_behavior_record,
            commands::affair::delete_behavior_record,
            // Statistics & Dashboard
            commands::stats::get_dashboard_stats,
            commands::stats::homework_statistics,
            commands::stats::score_statistics_cohort,
            commands::stats::get_student_profile,
            // Backup & Config
            commands::backup::create_backup,
            commands::backup::restore_backup,
            commands::backup::export_cohort,
            commands::config::get_config,
            commands::config::set_config,
            commands::config::download_template,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
