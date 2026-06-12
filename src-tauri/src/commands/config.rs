use std::{fs, path::PathBuf};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, State};

use crate::{resolve_app_data_dir, AppState};

const CONFIG_SCHOOL_NAME: &str = "default_school_name";
const CONFIG_HEAD_TEACHER: &str = "default_head_teacher";
const CONFIG_SEMESTER: &str = "default_semester";
const CONFIG_BACKUP_DIR: &str = "default_backup_dir";
const CONFIG_REMINDER_THRESHOLD: &str = "reminder_threshold";
const CONFIG_EXPORT_PREFERENCE: &str = "export_preference";

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsOverview {
    pub school_name: Option<String>,
    pub head_teacher: Option<String>,
    pub default_semester: Option<String>,
    pub default_backup_dir: String,
    pub reminder_threshold: i64,
    pub export_preference: String,
    pub app_version: String,
    pub database_version: i64,
    pub data_dir: String,
    pub database_path: String,
    pub recent_backups: Vec<BackupFileInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupFileInfo {
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

fn fallback_backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = resolve_app_data_dir(app)?.join("backups");
    fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {}", e))?;
    Ok(dir)
}

async fn get_config_value(state: &AppState, key: &str) -> Result<Option<String>, String> {
    let result = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT config_value FROM system_config WHERE config_key = ?1",
    )
    .bind(key)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("获取配置失败: {}", e))?;

    Ok(result.and_then(|r| r.0))
}

async fn upsert_config(state: &AppState, key: &str, value: &str) -> Result<(), String> {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO system_config (config_key, config_value, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(config_key) DO UPDATE SET config_value = excluded.config_value, updated_at = excluded.updated_at"
    )
    .bind(key)
    .bind(value)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| format!("设置配置失败: {}", e))?;
    Ok(())
}

fn list_recent_backups(dir: &PathBuf) -> Result<Vec<BackupFileInfo>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<BackupFileInfo> = fs::read_dir(dir)
        .map_err(|e| format!("读取备份目录失败: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("bak") {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            let datetime = chrono::DateTime::<Local>::from(modified);
            Some(BackupFileInfo {
                file_name: path.file_name()?.to_string_lossy().to_string(),
                file_path: path.to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                modified_at: datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
            })
        })
        .collect();

    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    entries.truncate(10);
    Ok(entries)
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    get_config_value(&state, &key).await
}

#[tauri::command]
pub async fn set_config(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    upsert_config(&state, &key, &value).await
}

#[tauri::command]
pub async fn get_settings_overview(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<SettingsOverview, String> {
    let school_name = get_config_value(&state, CONFIG_SCHOOL_NAME).await?;
    let head_teacher = get_config_value(&state, CONFIG_HEAD_TEACHER).await?;
    let default_semester = get_config_value(&state, CONFIG_SEMESTER).await?;
    let backup_dir_value = get_config_value(&state, CONFIG_BACKUP_DIR).await?;
    let reminder_threshold = get_config_value(&state, CONFIG_REMINDER_THRESHOLD)
        .await?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(3);
    let export_preference = get_config_value(&state, CONFIG_EXPORT_PREFERENCE)
        .await?
        .unwrap_or_else(|| "xlsx".to_string());

    let default_backup_dir = backup_dir_value.unwrap_or_else(|| {
        fallback_backup_dir(&app)
            .map(|dir| dir.to_string_lossy().to_string())
            .unwrap_or_else(|_| "不可用".to_string())
    });
    let backup_dir = PathBuf::from(&default_backup_dir);
    fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {}", e))?;

    let data_dir = resolve_app_data_dir(&app)?;
    let database_path = data_dir.join("class_management.db");
    let database_version: i64 = sqlx::query("PRAGMA user_version")
        .fetch_one(&state.db)
        .await
        .map_err(|e| format!("获取数据库版本失败: {}", e))?
        .get(0);

    Ok(SettingsOverview {
        school_name,
        head_teacher,
        default_semester,
        default_backup_dir: default_backup_dir.clone(),
        reminder_threshold,
        export_preference,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        database_version,
        data_dir: data_dir.to_string_lossy().to_string(),
        database_path: database_path.to_string_lossy().to_string(),
        recent_backups: list_recent_backups(&backup_dir)?,
    })
}

#[tauri::command]
pub async fn save_settings_preferences(
    state: State<'_, AppState>,
    app: AppHandle,
    school_name: Option<String>,
    head_teacher: Option<String>,
    default_semester: Option<String>,
    default_backup_dir: Option<String>,
    reminder_threshold: Option<i64>,
    export_preference: Option<String>,
) -> Result<(), String> {
    if let Some(value) = school_name {
        upsert_config(&state, CONFIG_SCHOOL_NAME, value.trim()).await?;
    }
    if let Some(value) = head_teacher {
        upsert_config(&state, CONFIG_HEAD_TEACHER, value.trim()).await?;
    }
    if let Some(value) = default_semester {
        upsert_config(&state, CONFIG_SEMESTER, value.trim()).await?;
    }
    if let Some(value) = default_backup_dir {
        let dir = if value.trim().is_empty() {
            fallback_backup_dir(&app)?
        } else {
            PathBuf::from(value.trim())
        };
        fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {}", e))?;
        upsert_config(&state, CONFIG_BACKUP_DIR, &dir.to_string_lossy()).await?;
    }
    if let Some(value) = reminder_threshold {
        let sanitized = value.clamp(1, 30).to_string();
        upsert_config(&state, CONFIG_REMINDER_THRESHOLD, &sanitized).await?;
    }
    if let Some(value) = export_preference {
        let normalized = match value.as_str() {
            "xlsx" | "pdf" | "both" => value,
            _ => return Err("导出偏好仅支持 xlsx、pdf 或 both".to_string()),
        };
        upsert_config(&state, CONFIG_EXPORT_PREFERENCE, &normalized).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_recent_backups(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<BackupFileInfo>, String> {
    let backup_dir_value = get_config_value(&state, CONFIG_BACKUP_DIR).await?;
    let dir = match backup_dir_value {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => fallback_backup_dir(&app)?,
    };
    fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {}", e))?;
    list_recent_backups(&dir)
}

#[tauri::command]
pub async fn download_template(
    _state: State<'_, AppState>,
    template_type: String,
    file_path: String,
) -> Result<(), String> {
    match template_type.as_str() {
        "student" => {
            use rust_xlsxwriter::*;
            let mut workbook = Workbook::new();
            let mut sheet = Worksheet::new();
            let student_headers = [
                "学号",
                "姓名",
                "性别",
                "联系电话",
                "家长姓名",
                "家长电话",
                "小组",
            ];
            for (ci, h) in student_headers.iter().enumerate() {
                sheet
                    .write_string(0, ci as u16, *h)
                    .map_err(|e| e.to_string())?;
            }
            sheet
                .write_string(1, 0, "2025001")
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(1, 1, "张三")
                .map_err(|e| e.to_string())?;
            sheet.write_string(1, 2, "男").map_err(|e| e.to_string())?;
            workbook.push_worksheet(sheet);
            workbook
                .save(&file_path)
                .map_err(|e| format!("生成模板失败: {}", e))?;
        }
        "score" => {
            use rust_xlsxwriter::*;
            let mut workbook = Workbook::new();
            let mut sheet = Worksheet::new();
            let score_headers = ["学号", "姓名", "成绩"];
            for (ci, h) in score_headers.iter().enumerate() {
                sheet
                    .write_string(0, ci as u16, *h)
                    .map_err(|e| e.to_string())?;
            }
            sheet
                .write_string(1, 0, "2025001")
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(1, 1, "张三")
                .map_err(|e| e.to_string())?;
            sheet.write_number(1, 2, 95.0).map_err(|e| e.to_string())?;
            workbook.push_worksheet(sheet);
            workbook
                .save(&file_path)
                .map_err(|e| format!("生成模板失败: {}", e))?;
        }
        _ => return Err("不支持的模板类型".to_string()),
    }
    Ok(())
}
