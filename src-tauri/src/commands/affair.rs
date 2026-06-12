use chrono::Local;
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use super::cohort::check_cohort_readonly;
use super::student::ensure_student_belongs_to_cohort;
use crate::AppState;

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

    let (base_where_clause, params) =
        build_affair_where("n", cohort_id, search, None, None, None, 1);
    let where_clause = format!("{base_where_clause} AND n.deleted_at IS NULL");

    let count_query = format!("SELECT COUNT(*) FROM notice n WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params {
        count_stmt = count_stmt.bind(p);
    }
    let (total,): (i64,) = count_stmt
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT n.* FROM notice n WHERE {} ORDER BY n.publish_date DESC LIMIT ?{} OFFSET ?{}",
        where_clause,
        params.len() as i64 + 1,
        params.len() as i64 + 2
    );
    let mut data_stmt = sqlx::query_as::<_, Notice>(&data_query);
    for p in &params {
        data_stmt = data_stmt.bind(p);
    }
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
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, notice.cohort_id).await?;

    sqlx::query(
        "UPDATE notice SET title = COALESCE(?1, title), content = COALESCE(?2, content),
         publish_date = COALESCE(?3, publish_date), remark = COALESCE(?4, remark), updated_at = ?5 WHERE id = ?6"
    )
    .bind(&title).bind(&content).bind(&publish_date).bind(&remark).bind(&now).bind(id)
    .execute(pool).await.map_err(|e| format!("更新通知失败: {}", e))?;

    sqlx::query_as::<_, Notice>("SELECT * FROM notice WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取通知失败: {}", e))
}

#[tauri::command]
pub async fn delete_notice(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let notice = sqlx::query_as::<_, Notice>("SELECT * FROM notice WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, notice.cohort_id).await?;
    sqlx::query("UPDATE notice SET deleted_at = ?1 WHERE id = ?2")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除通知失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn export_notices_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
    search: Option<String>,
) -> Result<(), String> {
    let result = get_notices(state, cohort_id, search, Some(1), Some(10_000)).await?;
    let rows = result["data"].as_array().cloned().unwrap_or_default();

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = ["发布日期", "标题", "正文", "备注"];
    for (idx, header) in headers.iter().enumerate() {
        worksheet
            .write_string(0, idx as u16, *header)
            .map_err(|e| e.to_string())?;
    }
    for (row_idx, row) in rows.iter().enumerate() {
        let line = (row_idx + 1) as u32;
        worksheet
            .write_string(line, 0, row["publish_date"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 1, row["title"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 2, row["content"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 3, row["remark"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
    }
    workbook
        .save(&file_path)
        .map_err(|e| format!("导出通知失败: {}", e))?;
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
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let (where_clause, params) = build_affair_where("d", cohort_id, search, None, status, None, 1);

    let count_query = format!("SELECT COUNT(*) FROM duty d WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params {
        count_stmt = count_stmt.bind(p);
    }
    let (total,): (i64,) = count_stmt
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT d.id, d.cohort_id, d.duty_date, d.student_id, d.group_name, d.duty_content, d.status, d.remark, d.created_at, d.updated_at, s.name as student_name
         FROM duty d LEFT JOIN student s ON d.student_id = s.id WHERE {} ORDER BY d.duty_date DESC LIMIT ?{} OFFSET ?{}",
        where_clause, params.len() as i64 + 1, params.len() as i64 + 2
    );
    let mut data_stmt = sqlx::query(&data_query);
    for p in &params {
        data_stmt = data_stmt.bind(p);
    }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let rows = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let data: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
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
        })
        .collect();

    Ok(serde_json::json!({ "data": data, "total": total, "page": page, "page_size": page_size }))
}

#[tauri::command]
pub async fn create_duty(
    state: State<'_, AppState>,
    cohort_id: i64,
    duty_date: String,
    group_name: Option<String>,
    student_id: Option<i64>,
    duty_content: Option<String>,
    status: Option<String>,
    remark: Option<String>,
) -> Result<Duty, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;

    // 如果指定了学生，校验学生属于目标届次
    if let Some(sid) = student_id {
        ensure_student_belongs_to_cohort(pool, sid, cohort_id).await?;
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
pub async fn update_duty(
    state: State<'_, AppState>,
    id: i64,
    duty_date: Option<String>,
    group_name: Option<String>,
    duty_content: Option<String>,
    status: Option<String>,
    remark: Option<String>,
) -> Result<Duty, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let existing = sqlx::query_as::<_, Duty>("SELECT * FROM duty WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, existing.cohort_id).await?;
    sqlx::query(
        "UPDATE duty SET duty_date = COALESCE(?1, duty_date), group_name = COALESCE(?2, group_name), duty_content = COALESCE(?3, duty_content), status = COALESCE(?4, status), remark = COALESCE(?5, remark), updated_at = ?6 WHERE id = ?7"
    )
    .bind(&duty_date).bind(&group_name).bind(&duty_content).bind(&status).bind(&remark).bind(&now).bind(id)
    .execute(pool).await.map_err(|e| format!("更新值日记录失败: {}", e))?;
    sqlx::query_as::<_, Duty>("SELECT * FROM duty WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取值日记录失败: {}", e))
}

#[tauri::command]
pub async fn delete_duty(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let existing = sqlx::query_as::<_, Duty>("SELECT * FROM duty WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, existing.cohort_id).await?;
    sqlx::query("DELETE FROM duty WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除值日记录失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn export_duties_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
    status: Option<String>,
    search: Option<String>,
) -> Result<(), String> {
    let result = get_duties(state, cohort_id, search, status, Some(1), Some(10_000)).await?;
    let rows = result["data"].as_array().cloned().unwrap_or_default();

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = ["日期", "学生", "小组", "内容", "状态", "备注"];
    for (idx, header) in headers.iter().enumerate() {
        worksheet
            .write_string(0, idx as u16, *header)
            .map_err(|e| e.to_string())?;
    }
    for (row_idx, row) in rows.iter().enumerate() {
        let line = (row_idx + 1) as u32;
        worksheet
            .write_string(line, 0, row["duty_date"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 1, row["student_name"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 2, row["group_name"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 3, row["duty_content"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 4, row["status"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 5, row["remark"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
    }
    workbook
        .save(&file_path)
        .map_err(|e| format!("导出值日安排失败: {}", e))?;
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

    let (where_clause, params) =
        build_affair_where("b", cohort_id, None, student_id, record_type, None, 1);

    let count_query = format!(
        "SELECT COUNT(*) FROM behavior_record b WHERE {}",
        where_clause
    );
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params {
        count_stmt = count_stmt.bind(p);
    }
    let (total,): (i64,) = count_stmt
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT b.id, b.cohort_id, b.student_id, b.type, b.title, b.score, b.description, b.record_date, b.created_at, b.updated_at,
                s.name as student_name, s.student_no
         FROM behavior_record b JOIN student s ON b.student_id = s.id WHERE {} ORDER BY b.record_date DESC LIMIT ?{} OFFSET ?{}",
        where_clause, params.len() as i64 + 1, params.len() as i64 + 2
    );
    let mut data_stmt = sqlx::query(&data_query);
    for p in &params {
        data_stmt = data_stmt.bind(p);
    }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let rows = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let data: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
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
        })
        .collect();

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
    let belongs: (i64,) =
        sqlx::query_as("SELECT cohort_id FROM student WHERE id = ?1 AND deleted_at IS NULL")
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
    let existing =
        sqlx::query_as::<_, BehaviorRecord>("SELECT * FROM behavior_record WHERE id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, existing.cohort_id).await?;
    sqlx::query("DELETE FROM behavior_record WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除奖惩记录失败: {}", e))?;
    Ok(())
}

// ==================== Class Fee ====================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClassFeeRecord {
    pub id: i64,
    pub cohort_id: i64,
    pub fee_date: String,
    pub fee_type: String,
    pub category: Option<String>,
    pub title: String,
    pub amount: f64,
    pub student_id: Option<i64>,
    pub payment_status: Option<String>,
    pub voucher_path: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[tauri::command]
pub async fn get_class_fee_records(
    state: State<'_, AppState>,
    cohort_id: i64,
    fee_type: Option<String>,
    student_id: Option<i64>,
    payment_status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let mut clauses = vec![
        "f.cohort_id = ?1".to_string(),
        "f.deleted_at IS NULL".to_string(),
    ];
    let mut params: Vec<String> = vec![cohort_id.to_string()];
    let mut idx = 2i64;

    if let Some(ref fee_type) = fee_type {
        clauses.push(format!("f.fee_type = ?{}", idx));
        params.push(fee_type.clone());
        idx += 1;
    }
    if let Some(student_id) = student_id {
        clauses.push(format!("f.student_id = ?{}", idx));
        params.push(student_id.to_string());
        idx += 1;
    }
    if let Some(ref payment_status) = payment_status {
        clauses.push(format!("f.payment_status = ?{}", idx));
        params.push(payment_status.clone());
        idx += 1;
    }
    let where_clause = clauses.join(" AND ");

    let count_query = format!("SELECT COUNT(*) FROM class_fee f WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params {
        count_stmt = count_stmt.bind(p);
    }
    let (total,): (i64,) = count_stmt
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let summary_query = format!(
        "SELECT
            CAST(COALESCE(SUM(CASE WHEN f.fee_type = '收入' THEN f.amount ELSE 0 END), 0) AS REAL),
            CAST(COALESCE(SUM(CASE WHEN f.fee_type = '支出' THEN f.amount ELSE 0 END), 0) AS REAL),
            CAST(COALESCE(SUM(CASE WHEN f.payment_status = '欠费' THEN f.amount ELSE 0 END), 0) AS REAL)
         FROM class_fee f WHERE {}",
        where_clause
    );
    let mut summary_stmt = sqlx::query_as::<_, (f64, f64, f64)>(&summary_query);
    for p in &params {
        summary_stmt = summary_stmt.bind(p);
    }
    let (income_total, expense_total, outstanding_total) = summary_stmt
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT f.*, s.name as student_name, s.student_no
         FROM class_fee f
         LEFT JOIN student s ON f.student_id = s.id
         WHERE {}
         ORDER BY f.fee_date DESC, f.id DESC
         LIMIT ?{} OFFSET ?{}",
        where_clause,
        idx,
        idx + 1
    );
    let mut data_stmt = sqlx::query(&data_query);
    for p in &params {
        data_stmt = data_stmt.bind(p);
    }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let rows = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let data: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "cohort_id": r.get::<i64, _>("cohort_id"),
                "fee_date": r.get::<String, _>("fee_date"),
                "fee_type": r.get::<String, _>("fee_type"),
                "category": r.get::<Option<String>, _>("category"),
                "title": r.get::<String, _>("title"),
                "amount": r.get::<f64, _>("amount"),
                "student_id": r.get::<Option<i64>, _>("student_id"),
                "payment_status": r.get::<Option<String>, _>("payment_status"),
                "voucher_path": r.get::<Option<String>, _>("voucher_path"),
                "remark": r.get::<Option<String>, _>("remark"),
                "created_at": r.get::<String, _>("created_at"),
                "updated_at": r.get::<String, _>("updated_at"),
                "deleted_at": r.get::<Option<String>, _>("deleted_at"),
                "student_name": r.get::<Option<String>, _>("student_name"),
                "student_no": r.get::<Option<String>, _>("student_no"),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "data": data,
        "total": total,
        "page": page,
        "page_size": page_size,
        "summary": {
            "income_total": income_total,
            "expense_total": expense_total,
            "balance": income_total - expense_total,
            "outstanding_total": outstanding_total
        }
    }))
}

#[tauri::command]
pub async fn create_class_fee_record(
    state: State<'_, AppState>,
    cohort_id: i64,
    fee_date: Option<String>,
    fee_type: String,
    category: Option<String>,
    title: String,
    amount: f64,
    student_id: Option<i64>,
    payment_status: Option<String>,
    voucher_path: Option<String>,
    remark: Option<String>,
) -> Result<ClassFeeRecord, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;
    validate_class_fee_payload(
        pool,
        cohort_id,
        &fee_type,
        amount,
        student_id,
        payment_status.as_deref(),
    )
    .await?;

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let fee_date = fee_date.unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());

    sqlx::query_as::<_, ClassFeeRecord>(
        "INSERT INTO class_fee (cohort_id, fee_date, fee_type, category, title, amount, student_id, payment_status, voucher_path, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11) RETURNING *"
    )
    .bind(cohort_id)
    .bind(&fee_date)
    .bind(&fee_type)
    .bind(&category)
    .bind(&title)
    .bind(amount)
    .bind(student_id)
    .bind(&payment_status)
    .bind(&voucher_path)
    .bind(&remark)
    .bind(&now)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("创建班费记录失败: {}", e))
}

#[tauri::command]
pub async fn update_class_fee_record(
    state: State<'_, AppState>,
    id: i64,
    fee_date: Option<String>,
    fee_type: Option<String>,
    category: Option<String>,
    title: Option<String>,
    amount: Option<f64>,
    student_id: Option<i64>,
    payment_status: Option<String>,
    voucher_path: Option<String>,
    remark: Option<String>,
) -> Result<ClassFeeRecord, String> {
    let pool = &state.db;
    let existing = sqlx::query_as::<_, ClassFeeRecord>(
        "SELECT * FROM class_fee WHERE id = ?1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("获取班费记录失败: {}", e))?;
    check_cohort_readonly(pool, existing.cohort_id).await?;

    let next_fee_type = fee_type
        .clone()
        .unwrap_or_else(|| existing.fee_type.clone());
    let next_amount = amount.unwrap_or(existing.amount);
    let next_student_id = student_id.or(existing.student_id);
    let next_payment_status = payment_status.clone().or(existing.payment_status.clone());
    validate_class_fee_payload(
        pool,
        existing.cohort_id,
        &next_fee_type,
        next_amount,
        next_student_id,
        next_payment_status.as_deref(),
    )
    .await?;

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "UPDATE class_fee
         SET fee_date = COALESCE(?1, fee_date), fee_type = COALESCE(?2, fee_type), category = COALESCE(?3, category),
             title = COALESCE(?4, title), amount = COALESCE(?5, amount), student_id = COALESCE(?6, student_id),
             payment_status = COALESCE(?7, payment_status), voucher_path = COALESCE(?8, voucher_path),
             remark = COALESCE(?9, remark), updated_at = ?10
         WHERE id = ?11"
    )
    .bind(&fee_date)
    .bind(&fee_type)
    .bind(&category)
    .bind(&title)
    .bind(amount)
    .bind(next_student_id)
    .bind(&payment_status)
    .bind(&voucher_path)
    .bind(&remark)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新班费记录失败: {}", e))?;

    sqlx::query_as::<_, ClassFeeRecord>("SELECT * FROM class_fee WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取班费记录失败: {}", e))
}

#[tauri::command]
pub async fn delete_class_fee_record(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let existing = sqlx::query_as::<_, ClassFeeRecord>(
        "SELECT * FROM class_fee WHERE id = ?1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("获取班费记录失败: {}", e))?;
    check_cohort_readonly(pool, existing.cohort_id).await?;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query("UPDATE class_fee SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除班费记录失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn export_class_fee_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
    fee_type: Option<String>,
    student_id: Option<i64>,
    payment_status: Option<String>,
) -> Result<(), String> {
    let result = get_class_fee_records(
        state,
        cohort_id,
        fee_type,
        student_id,
        payment_status,
        Some(1),
        Some(10_000),
    )
    .await?;
    let rows = result["data"].as_array().cloned().unwrap_or_default();

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = [
        "日期",
        "类型",
        "分类",
        "标题",
        "金额",
        "学生",
        "学号",
        "缴费状态",
        "凭证",
        "备注",
    ];
    for (idx, header) in headers.iter().enumerate() {
        worksheet
            .write_string(0, idx as u16, *header)
            .map_err(|e| e.to_string())?;
    }
    for (row_idx, row) in rows.iter().enumerate() {
        let line = (row_idx + 1) as u32;
        worksheet
            .write_string(line, 0, row["fee_date"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 1, row["fee_type"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 2, row["category"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 3, row["title"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(line, 4, row["amount"].as_f64().unwrap_or_default())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 5, row["student_name"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 6, row["student_no"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 7, row["payment_status"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 8, row["voucher_path"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 9, row["remark"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
    }
    workbook
        .save(&file_path)
        .map_err(|e| format!("导出班费失败: {}", e))?;
    Ok(())
}

async fn validate_class_fee_payload(
    pool: &sqlx::SqlitePool,
    cohort_id: i64,
    fee_type: &str,
    amount: f64,
    student_id: Option<i64>,
    payment_status: Option<&str>,
) -> Result<(), String> {
    let valid_types = ["收入", "支出"];
    if !valid_types.contains(&fee_type) {
        return Err("班费类型只能是“收入”或“支出”".to_string());
    }
    if amount < 0.0 {
        return Err("金额不能为负数".to_string());
    }
    if let Some(student_id) = student_id {
        ensure_student_belongs_to_cohort(pool, student_id, cohort_id).await?;
    }
    if let Some(payment_status) = payment_status {
        let valid_statuses = ["待缴费", "已缴费", "欠费"];
        if !valid_statuses.contains(&payment_status) {
            return Err("缴费状态无效".to_string());
        }
        if fee_type == "支出" {
            return Err("支出记录不应设置缴费状态".to_string());
        }
    }
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
        clauses.push(format!(
            "({}.title LIKE ?{} OR {}.content LIKE ?{})",
            alias, idx, alias, idx
        ));
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
