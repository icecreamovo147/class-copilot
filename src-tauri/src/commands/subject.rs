use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subject {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub is_active: bool,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_subjects(
    state: State<'_, AppState>,
    active_only: Option<bool>,
) -> Result<Vec<Subject>, String> {
    let mut query = String::from("SELECT * FROM subject");
    if active_only.unwrap_or(false) {
        query.push_str(" WHERE is_active = 1");
    }
    query.push_str(" ORDER BY is_active DESC, sort_order ASC, id ASC");

    sqlx::query_as::<_, Subject>(&query)
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("获取科目列表失败: {}", e))
}

#[tauri::command]
pub async fn create_subject(
    state: State<'_, AppState>,
    name: String,
    sort_order: Option<i64>,
    is_active: Option<bool>,
    remark: Option<String>,
) -> Result<Subject, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let sort_order = sort_order.unwrap_or(0);

    sqlx::query_as::<_, Subject>(
        "INSERT INTO subject (name, sort_order, is_active, remark, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5) RETURNING *"
    )
    .bind(&name)
    .bind(sort_order)
    .bind(is_active.unwrap_or(true))
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
    is_active: Option<bool>,
    remark: Option<String>,
) -> Result<Subject, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "UPDATE subject SET name = COALESCE(?1, name), sort_order = COALESCE(?2, sort_order), is_active = COALESCE(?3, is_active), remark = COALESCE(?4, remark), updated_at = ?5 WHERE id = ?6"
    )
    .bind(&name)
    .bind(sort_order)
    .bind(is_active)
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
    let pool = &state.db;

    let subject = sqlx::query_as::<_, Subject>("SELECT * FROM subject WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取科目失败: {}", e))?;

    let has_homework_refs: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM homework WHERE subject_id = ?1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("检查作业引用失败: {}", e))?;
    let has_score_refs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM score WHERE subject_id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("检查成绩引用失败: {}", e))?;

    if has_homework_refs.0 > 0 || has_score_refs.0 > 0 {
        return Err("科目已被历史作业或成绩引用，请先停用，不能直接物理删除".to_string());
    }
    if subject.is_active {
        return Err("请先停用科目，再执行删除".to_string());
    }

    sqlx::query("DELETE FROM subject WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除科目失败: {}", e))?;
    Ok(())
}
