use chrono::Local;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_config(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    let result = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT config_value FROM system_config WHERE config_key = ?1"
    )
    .bind(&key)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("获取配置失败: {}", e))?;

    Ok(result.map(|r| r.0).flatten())
}

#[tauri::command]
pub async fn set_config(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO system_config (config_key, config_value, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(config_key) DO UPDATE SET config_value = excluded.config_value, updated_at = excluded.updated_at"
    )
    .bind(&key)
    .bind(&value)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("设置配置失败: {}", e))?;

    Ok(())
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
            let student_headers = ["学号", "姓名", "性别", "联系电话", "家长姓名", "家长电话", "小组"];
            for (ci, h) in student_headers.iter().enumerate() {
                sheet.write_string(0, ci as u16, *h).map_err(|e| e.to_string())?;
            }
            sheet.write_string(1, 0, "2025001").map_err(|e| e.to_string())?;
            sheet.write_string(1, 1, "张三").map_err(|e| e.to_string())?;
            sheet.write_string(1, 2, "男").map_err(|e| e.to_string())?;
            workbook.push_worksheet(sheet);
            workbook.save(&file_path).map_err(|e| format!("生成模板失败: {}", e))?;
        }
        "score" => {
            use rust_xlsxwriter::*;
            let mut workbook = Workbook::new();
            let mut sheet = Worksheet::new();
            let score_headers = ["学号", "姓名", "成绩"];
            for (ci, h) in score_headers.iter().enumerate() {
                sheet.write_string(0, ci as u16, *h).map_err(|e| e.to_string())?;
            }
            sheet.write_string(1, 0, "2025001").map_err(|e| e.to_string())?;
            sheet.write_string(1, 1, "张三").map_err(|e| e.to_string())?;
            sheet.write_number(1, 2, 95.0).map_err(|e| e.to_string())?;
            workbook.push_worksheet(sheet);
            workbook.save(&file_path).map_err(|e| format!("生成模板失败: {}", e))?;
        }
        _ => return Err("不支持的模板类型".to_string()),
    }
    Ok(())
}
