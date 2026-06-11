use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::State;
use sqlx::Row;

use crate::AppState;
use super::cohort::check_cohort_readonly;

// ==================== Notice ====================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notice {
    pub id: i64,
    pub cohort_id: i64,
    pub title: String,
    pub content: Option<String>,
    pub publish_date: String,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[tauri::command]
pub async fn get_notices(
    state: State<'_, AppState>,
    cohort_id: i64,
    search: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let (where_clause, params) = build_affair_where("n", cohort_id, search, None, None, None, 1);

    let count_query = format!("SELECT COUNT(*) FROM notice n WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params { count_stmt = count_stmt.bind(p); }
    let (total,): (i64,) = count_stmt.fetch_one(pool).await.map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT n.* FROM notice n WHERE {} ORDER BY n.publish_date DESC LIMIT ?{} OFFSET ?{}",
        where_clause,
        params.len() as i64 + 1,
        params.len() as i64 + 2
    );
    let mut data_stmt = sqlx::query_as::<_, Notice>(&data_query);
    for p in &params { data_stmt = data_stmt.bind(p); }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let data = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "data": data, "total": total, "page": page, "page_size": page_size }))
}

#[tauri::command]
pub async fn create_notice(
    state: State<'_, AppState>,
    cohort_id: i64,
    title: String,
    content: Option<String>,
    publish_date: Option<String>,
    remark: Option<String>,
) -> Result<Notice, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let publish_date = publish_date.unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());

    sqlx::query_as::<_, Notice>(
        "INSERT INTO notice (cohort_id, title, content, publish_date, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) RETURNING *"
    )
    .bind(cohort_id).bind(&title).bind(&content).bind(&publish_date)
    .bind(&remark).bind(&now)
    .fetch_one(pool).await
    .map_err(|e| format!("创建通知失败: {}", e))
}

#[tauri::command]
pub async fn update_notice(
    state: State<'_, AppState>,
    id: i64,
    title: Option<String>,
    content: Option<String>,
    publish_date: Option<String>,
    remark: Option<String>,
) -> Result<Notice, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let notice = sqlx::query_as::<_, Notice>("SELECT * FROM notice WHERE id = ?1")
        .bind(id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, notice.cohort_id).await?;

    sqlx::query(
        "UPDATE notice SET title = COALESCE(?1, title), content = COALESCE(?2, content),
         publish_date = COALESCE(?3, publish_date), remark = COALESCE(?4, remark), updated_at = ?5 WHERE id = ?6"
    )
    .bind(&title).bind(&content).bind(&publish_date).bind(&remark).bind(&now).bind(id)
    .execute(pool).await.map_err(|e| format!("更新通知失败: {}", e))?;

    sqlx::query_as::<_, Notice>("SELECT * FROM notice WHERE id = ?1")
        .bind(id).fetch_one(pool).await.map_err(|e| format!("获取通知失败: {}", e))
}

#[tauri::command]
pub async fn delete_notice(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let notice = sqlx::query_as::<_, Notice>("SELECT * FROM notice WHERE id = ?1")
        .bind(id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, notice.cohort_id).await?;
    sqlx::query("UPDATE notice SET deleted_at = ?1 WHERE id = ?2")
        .bind(&now).bind(id).execute(pool).await
        .map_err(|e| format!("删除通知失败: {}", e))?;
    Ok(())
}

// ==================== Duty ====================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Duty {
    pub id: i64,
    pub cohort_id: i64,
    pub duty_date: String,
    pub student_id: Option<i64>,
    pub group_name: Option<String>,
    pub duty_content: Option<String>,
    pub status: String,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_duties(
    state: State<'_, AppState>,
    cohort_id: i64,
    search: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let (where_clause, params) = build_affair_where("d", cohort_id, search, None, None, None, 1);

    let count_query = format!("SELECT COUNT(*) FROM duty d WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params { count_stmt = count_stmt.bind(p); }
    let (total,): (i64,) = count_stmt.fetch_one(pool).await.map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT d.id, d.cohort_id, d.duty_date, d.student_id, d.group_name, d.duty_content, d.status, d.remark, d.created_at, d.updated_at, s.name as student_name
         FROM duty d LEFT JOIN student s ON d.student_id = s.id WHERE {} ORDER BY d.duty_date DESC LIMIT ?{} OFFSET ?{}",
        where_clause, params.len() as i64 + 1, params.len() as i64 + 2
    );
    let mut data_stmt = sqlx::query(&data_query);
    for p in &params { data_stmt = data_stmt.bind(p); }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let rows = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let data: Vec<serde_json::Value> = rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<i64, _>("id"),
            "cohort_id": r.get::<i64, _>("cohort_id"),
            "duty_date": r.get::<String, _>("duty_date"),
            "student_id": r.get::<Option<i64>, _>("student_id"),
            "group_name": r.get::<Option<String>, _>("group_name"),
            "duty_content": r.get::<Option<String>, _>("duty_content"),
            "status": r.get::<String, _>("status"),
            "remark": r.get::<Option<String>, _>("remark"),
            "created_at": r.get::<String, _>("created_at"),
            "updated_at": r.get::<String, _>("updated_at"),
            "student_name": r.get::<Option<String>, _>("student_name")
        })
    }).collect();

    Ok(serde_json::json!({ "data": data, "total": total, "page": page, "page_size": page_size }))
}

#[tauri::command]
pub async fn create_duty(state: State<'_, AppState>, cohort_id: i64, duty_date: String, group_name: Option<String>, student_id: Option<i64>, duty_content: Option<String>, status: Option<String>, remark: Option<String>) -> Result<Duty, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;

    // 如果指定了学生，校验学生属于目标届次
    if let Some(sid) = student_id {
        let belongs: (i64,) = sqlx::query_as(
            "SELECT cohort_id FROM student WHERE id = ?1 AND deleted_at IS NULL"
        )
        .bind(sid)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("学生 ID {} 不存在或已删除", sid))?;

        if belongs.0 != cohort_id {
            return Err(format!(
                "学生 ID {} 不属于届次 {} (学生届次: {})",
                sid, cohort_id, belongs.0
            ));
        }
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query_as::<_, Duty>(
        "INSERT INTO duty (cohort_id, duty_date, group_name, student_id, duty_content, status, remark, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8) RETURNING *"
    )
    .bind(cohort_id).bind(&duty_date).bind(&group_name).bind(student_id)
    .bind(&duty_content).bind(&status.unwrap_or_else(|| "未完成".to_string()))
    .bind(&remark).bind(&now)
    .fetch_one(pool).await.map_err(|e| format!("创建值日记录失败: {}", e))
}

#[tauri::command]
pub async fn update_duty(state: State<'_, AppState>, id: i64, duty_date: Option<String>, group_name: Option<String>, duty_content: Option<String>, status: Option<String>, remark: Option<String>) -> Result<Duty, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let existing = sqlx::query_as::<_, Duty>("SELECT * FROM duty WHERE id = ?1").bind(id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, existing.cohort_id).await?;
    sqlx::query(
        "UPDATE duty SET duty_date = COALESCE(?1, duty_date), group_name = COALESCE(?2, group_name), duty_content = COALESCE(?3, duty_content), status = COALESCE(?4, status), remark = COALESCE(?5, remark), updated_at = ?6 WHERE id = ?7"
    )
    .bind(&duty_date).bind(&group_name).bind(&duty_content).bind(&status).bind(&remark).bind(&now).bind(id)
    .execute(pool).await.map_err(|e| format!("更新值日记录失败: {}", e))?;
    sqlx::query_as::<_, Duty>("SELECT * FROM duty WHERE id = ?1").bind(id).fetch_one(pool).await.map_err(|e| format!("获取值日记录失败: {}", e))
}

#[tauri::command]
pub async fn delete_duty(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let existing = sqlx::query_as::<_, Duty>("SELECT * FROM duty WHERE id = ?1").bind(id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, existing.cohort_id).await?;
    sqlx::query("DELETE FROM duty WHERE id = ?1").bind(id).execute(pool).await.map_err(|e| format!("删除值日记录失败: {}", e))?;
    Ok(())
}

// ==================== Behavior Record ====================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BehaviorRecord {
    pub id: i64,
    pub cohort_id: i64,
    pub student_id: i64,
    #[sqlx(rename = "type")]
    pub record_type: String,
    pub title: String,
    pub score: i64,
    pub description: Option<String>,
    pub record_date: String,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_behavior_records(
    state: State<'_, AppState>,
    cohort_id: i64,
    student_id: Option<i64>,
    record_type: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let (where_clause, params) = build_affair_where("b", cohort_id, None, student_id, record_type, None, 1);

    let count_query = format!("SELECT COUNT(*) FROM behavior_record b WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params { count_stmt = count_stmt.bind(p); }
    let (total,): (i64,) = count_stmt.fetch_one(pool).await.map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT b.id, b.cohort_id, b.student_id, b.type, b.title, b.score, b.description, b.record_date, b.created_at, b.updated_at,
                s.name as student_name, s.student_no
         FROM behavior_record b JOIN student s ON b.student_id = s.id WHERE {} ORDER BY b.record_date DESC LIMIT ?{} OFFSET ?{}",
        where_clause, params.len() as i64 + 1, params.len() as i64 + 2
    );
    let mut data_stmt = sqlx::query(&data_query);
    for p in &params { data_stmt = data_stmt.bind(p); }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let rows = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let data: Vec<serde_json::Value> = rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<i64, _>("id"),
            "cohort_id": r.get::<i64, _>("cohort_id"),
            "student_id": r.get::<i64, _>("student_id"),
            "type": r.get::<String, _>("type"),
            "title": r.get::<String, _>("title"),
            "score": r.get::<i64, _>("score"),
            "description": r.get::<Option<String>, _>("description"),
            "record_date": r.get::<String, _>("record_date"),
            "created_at": r.get::<String, _>("created_at"),
            "updated_at": r.get::<String, _>("updated_at"),
            "student_name": r.get::<String, _>("student_name"),
            "student_no": r.get::<String, _>("student_no")
        })
    }).collect();

    Ok(serde_json::json!({ "data": data, "total": total, "page": page, "page_size": page_size }))
}

#[tauri::command]
pub async fn create_behavior_record(
    state: State<'_, AppState>,
    cohort_id: i64,
    student_id: i64,
    record_type: String,
    title: String,
    score: Option<i64>,
    description: Option<String>,
    record_date: Option<String>,
) -> Result<BehaviorRecord, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;

    // 校验学生属于目标届次
    let belongs: (i64,) = sqlx::query_as(
        "SELECT cohort_id FROM student WHERE id = ?1 AND deleted_at IS NULL"
    )
    .bind(student_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("学生 ID {} 不存在或已删除", student_id))?;

    if belongs.0 != cohort_id {
        return Err(format!(
            "学生 ID {} 不属于届次 {} (学生届次: {})",
            student_id, cohort_id, belongs.0
        ));
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let record_date = record_date.unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());

    sqlx::query_as::<_, BehaviorRecord>(
        "INSERT INTO behavior_record (cohort_id, student_id, type, title, score, description, record_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8) RETURNING *"
    )
    .bind(cohort_id).bind(student_id).bind(&record_type).bind(&title)
    .bind(score.unwrap_or(0)).bind(&description).bind(&record_date).bind(&now)
    .fetch_one(pool).await
    .map_err(|e| format!("创建奖惩记录失败: {}", e))
}

#[tauri::command]
pub async fn delete_behavior_record(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let existing = sqlx::query_as::<_, BehaviorRecord>("SELECT * FROM behavior_record WHERE id = ?1")
        .bind(id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, existing.cohort_id).await?;
    sqlx::query("DELETE FROM behavior_record WHERE id = ?1")
        .bind(id).execute(pool).await
        .map_err(|e| format!("删除奖惩记录失败: {}", e))?;
    Ok(())
}

// 通用查询构建辅助
fn build_affair_where(
    alias: &str,
    cohort_id: i64,
    search: Option<String>,
    student_id: Option<i64>,
    record_type: Option<String>,
    _start_date: Option<String>,
    start_idx: i64,
) -> (String, Vec<String>) {
    let mut clauses = vec![format!("{}.cohort_id = ?{}", alias, start_idx)];
    let mut params = vec![cohort_id.to_string()];
    let mut idx = start_idx + 1;

    if let Some(ref s) = search {
        clauses.push(format!("({}.title LIKE ?{} OR {}.content LIKE ?{})", alias, idx, alias, idx));
        params.push(format!("%{}%", s));
        idx += 1;
    }
    if let Some(sid) = student_id {
        clauses.push(format!("{}.student_id = ?{}", alias, idx));
        params.push(sid.to_string());
        idx += 1;
    }
    if let Some(ref rt) = record_type {
        clauses.push(format!("{}.type = ?{}", alias, idx));
        params.push(rt.clone());
    }

    (clauses.join(" AND "), params)
}
