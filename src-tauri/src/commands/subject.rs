use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subject {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_subjects(state: State<'_, AppState>) -> Result<Vec<Subject>, String> {
    sqlx::query_as::<_, Subject>("SELECT * FROM subject ORDER BY sort_order ASC, id ASC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("获取科目列表失败: {}", e))
}

#[tauri::command]
pub async fn create_subject(
    state: State<'_, AppState>,
    name: String,
    sort_order: Option<i64>,
    remark: Option<String>,
) -> Result<Subject, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let sort_order = sort_order.unwrap_or(0);

    sqlx::query_as::<_, Subject>(
        "INSERT INTO subject (name, sort_order, remark, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4) RETURNING *"
    )
    .bind(&name)
    .bind(sort_order)
    .bind(&remark)
    .bind(&now)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "科目名已存在".to_string()
        } else {
            format!("创建科目失败: {}", e)
        }
    })
}

#[tauri::command]
pub async fn update_subject(
    state: State<'_, AppState>,
    id: i64,
    name: Option<String>,
    sort_order: Option<i64>,
    remark: Option<String>,
) -> Result<Subject, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "UPDATE subject SET name = COALESCE(?1, name), sort_order = COALESCE(?2, sort_order), remark = COALESCE(?3, remark), updated_at = ?4 WHERE id = ?5"
    )
    .bind(&name)
    .bind(sort_order)
    .bind(&remark)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新科目失败: {}", e))?;

    sqlx::query_as::<_, Subject>("SELECT * FROM subject WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取科目失败: {}", e))
}

#[tauri::command]
pub async fn delete_subject(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM subject WHERE id = ?1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("删除科目失败: {}", e))?;
    Ok(())
}
