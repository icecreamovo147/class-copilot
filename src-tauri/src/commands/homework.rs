use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::State;
use sqlx::Row;

use crate::AppState;
use super::cohort::check_cohort_readonly;
use super::student::Student;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Homework {
    pub id: i64,
    pub cohort_id: i64,
    pub title: String,
    pub subject_id: Option<i64>,
    pub subject_name: Option<String>,
    pub description: Option<String>,
    pub publish_date: String,
    pub deadline: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct HomeworkRecord {
    pub id: i64,
    pub homework_id: i64,
    pub student_id: i64,
    pub status: String,
    pub submit_time: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_homeworks(
    state: State<'_, AppState>,
    cohort_id: i64,
    search: Option<String>,
    subject_id: Option<i64>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(10);
    let offset = (page - 1) * page_size;

    let mut where_clauses = vec!["h.cohort_id = ?1".to_string(), "h.deleted_at IS NULL".to_string()];
    let mut params: Vec<String> = vec![cohort_id.to_string()];
    let mut param_idx = 2;

    if let Some(ref s) = search {
        where_clauses.push(format!("h.title LIKE ?{}", param_idx));
        params.push(format!("%{}%", s));
        param_idx += 1;
    }
    if let Some(sid) = subject_id {
        where_clauses.push(format!("h.subject_id = ?{}", param_idx));
        params.push(sid.to_string());
        param_idx += 1;
    }

    let where_clause = where_clauses.join(" AND ");

    // 总数
    let count_query = format!("SELECT COUNT(*) FROM homework h WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params { count_stmt = count_stmt.bind(p); }
    let (total,): (i64,) = count_stmt.fetch_one(pool).await.map_err(|e| e.to_string())?;

    // 数据
    let data_query = format!(
        "SELECT h.id, h.cohort_id, h.title, h.subject_id, h.subject_name, h.description,
                h.publish_date, h.deadline, h.remark, h.created_at, h.updated_at, h.deleted_at,
            (SELECT COUNT(*) FROM homework_record hr WHERE hr.homework_id = h.id AND hr.status = '已完成') as completed_count,
            (SELECT COUNT(*) FROM homework_record hr WHERE hr.homework_id = h.id) as total_count,
            CASE WHEN (SELECT COUNT(*) FROM homework_record hr WHERE hr.homework_id = h.id) > 0 
                THEN CAST((SELECT COUNT(*) FROM homework_record hr WHERE hr.homework_id = h.id AND hr.status = '已完成') AS REAL) / 
                     (SELECT COUNT(*) FROM homework_record hr WHERE hr.homework_id = h.id) 
                ELSE 0 END as completion_rate,
            (SELECT COUNT(*) FROM homework_record hr WHERE hr.homework_id = h.id AND hr.status IN ('未登记', '未完成')) as incomplete_count
         FROM homework h WHERE {} ORDER BY h.created_at DESC LIMIT ?{} OFFSET ?{}",
        where_clause, param_idx, param_idx + 1
    );
    let mut data_stmt = sqlx::query(&data_query);
    for p in &params { data_stmt = data_stmt.bind(p); }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let rows = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    let data: Vec<serde_json::Value> = rows.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<i64, _>("id"),
            "cohort_id": row.get::<i64, _>("cohort_id"),
            "title": row.get::<String, _>("title"),
            "subject_id": row.get::<Option<i64>, _>("subject_id"),
            "subject_name": row.get::<Option<String>, _>("subject_name"),
            "description": row.get::<Option<String>, _>("description"),
            "publish_date": row.get::<String, _>("publish_date"),
            "deadline": row.get::<Option<String>, _>("deadline"),
            "remark": row.get::<Option<String>, _>("remark"),
            "created_at": row.get::<String, _>("created_at"),
            "updated_at": row.get::<String, _>("updated_at"),
            "deleted_at": row.get::<Option<String>, _>("deleted_at"),
            "completed_count": row.get::<i64, _>("completed_count"),
            "total_count": row.get::<i64, _>("total_count"),
            "completion_rate": row.get::<f64, _>("completion_rate"),
            "incomplete_count": row.get::<i64, _>("incomplete_count")
        })
    }).collect();

    Ok(serde_json::json!({
        "data": data, "total": total, "page": page, "page_size": page_size
    }))
}

#[tauri::command]
pub async fn get_homework(state: State<'_, AppState>, id: i64) -> Result<Homework, String> {
    sqlx::query_as::<_, Homework>("SELECT * FROM homework WHERE id = ?1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| format!("获取作业失败: {}", e))
}

#[tauri::command]
pub async fn create_homework(
    state: State<'_, AppState>,
    cohort_id: i64,
    title: String,
    subject_id: Option<i64>,
    subject_name: Option<String>,
    description: Option<String>,
    publish_date: Option<String>,
    deadline: Option<String>,
    remark: Option<String>,
) -> Result<Homework, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let publish_date = publish_date.unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());

    // 事务操作：作业创建和记录创建原子化
    let mut tx = pool.begin().await.map_err(|e| format!("开始事务失败: {}", e))?;

    let homework = sqlx::query_as::<_, Homework>(
        "INSERT INTO homework (cohort_id, title, subject_id, subject_name, description, publish_date, deadline, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         RETURNING *"
    )
    .bind(cohort_id)
    .bind(&title)
    .bind(subject_id)
    .bind(&subject_name)
    .bind(&description)
    .bind(&publish_date)
    .bind(&deadline)
    .bind(&remark)
    .bind(&now)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("创建作业失败: {}", e))?;

    // 为当前届次所有有效学生创建作业记录
    let students = sqlx::query_as::<_, Student>(
        "SELECT * FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL AND status = '正常'"
    )
    .bind(cohort_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("查询学生失败: {}", e))?;

    for student in &students {
        sqlx::query(
            "INSERT OR IGNORE INTO homework_record (homework_id, student_id, status, created_at, updated_at)
             VALUES (?1, ?2, '未登记', ?3, ?3)"
        )
        .bind(homework.id)
        .bind(student.id)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("为学生 {} 创建作业记录失败: {}", student.name, e))?;
    }

    tx.commit().await.map_err(|e| format!("提交作业创建失败: {}", e))?;

    Ok(homework)
}

#[tauri::command]
pub async fn update_homework(
    state: State<'_, AppState>,
    id: i64,
    title: Option<String>,
    subject_id: Option<i64>,
    subject_name: Option<String>,
    description: Option<String>,
    publish_date: Option<String>,
    deadline: Option<String>,
    remark: Option<String>,
) -> Result<Homework, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let hw = get_homework_internal(pool, id).await?;
    check_cohort_readonly(pool, hw.cohort_id).await?;

    sqlx::query(
        "UPDATE homework SET title = COALESCE(?1, title), subject_id = COALESCE(?2, subject_id),
         subject_name = COALESCE(?3, subject_name), description = COALESCE(?4, description),
         publish_date = COALESCE(?5, publish_date), deadline = COALESCE(?6, deadline),
         remark = COALESCE(?7, remark), updated_at = ?8 WHERE id = ?9"
    )
    .bind(&title)
    .bind(subject_id)
    .bind(&subject_name)
    .bind(&description)
    .bind(&publish_date)
    .bind(&deadline)
    .bind(&remark)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新作业失败: {}", e))?;

    get_homework_internal(pool, id).await
}

#[tauri::command]
pub async fn delete_homework(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let hw = get_homework_internal(pool, id).await?;
    check_cohort_readonly(pool, hw.cohort_id).await?;

    sqlx::query("UPDATE homework SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除作业失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_homework_records(
    state: State<'_, AppState>,
    homework_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let pool = &state.db;
    let rows = sqlx::query(
        "SELECT hr.id, hr.homework_id, hr.student_id, hr.status, hr.submit_time, hr.remark, hr.created_at, hr.updated_at,
                s.name as student_name, s.student_no, s.group_name
         FROM homework_record hr
         JOIN student s ON hr.student_id = s.id
         WHERE hr.homework_id = ?1 AND s.deleted_at IS NULL
         ORDER BY s.student_no ASC"
    )
    .bind(homework_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<i64, _>("id"),
            "homework_id": r.get::<i64, _>("homework_id"),
            "student_id": r.get::<i64, _>("student_id"),
            "status": r.get::<String, _>("status"),
            "submit_time": r.get::<Option<String>, _>("submit_time"),
            "remark": r.get::<Option<String>, _>("remark"),
            "created_at": r.get::<String, _>("created_at"),
            "updated_at": r.get::<String, _>("updated_at"),
            "student_name": r.get::<String, _>("student_name"),
            "student_no": r.get::<String, _>("student_no"),
            "group_name": r.get::<Option<String>, _>("group_name")
        })
    }).collect())
}

#[tauri::command]
pub async fn update_homework_record(
    state: State<'_, AppState>,
    id: i64,
    status: String,
    remark: Option<String>,
) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 验证状态值
    let valid_statuses = ["未登记", "已完成", "未完成", "迟交", "补交", "质量较差"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err("无效的作业状态".to_string());
    }

    // 通过 homework_record -> homework -> cohort 查询所属届次，校验归档只读
    let cohort_id: (i64,) = sqlx::query_as(
        "SELECT h.cohort_id FROM homework_record hr
         JOIN homework h ON hr.homework_id = h.id
         WHERE hr.id = ?1"
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|_| "作业记录不存在".to_string())?;

    check_cohort_readonly(pool, cohort_id.0).await?;

    sqlx::query(
        "UPDATE homework_record SET status = ?1, remark = COALESCE(?2, remark), submit_time = CASE WHEN ?1 = '已完成' THEN ?3 ELSE submit_time END, updated_at = ?3 WHERE id = ?4"
    )
    .bind(&status)
    .bind(&remark)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新作业记录失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn batch_update_homework_records(
    state: State<'_, AppState>,
    homework_id: i64,
    student_ids: Vec<i64>,
    status: String,
) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let valid_statuses = ["未登记", "已完成", "未完成", "迟交", "补交", "质量较差"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err("无效的作业状态".to_string());
    }

    // 通过 homework_id 查询所属届次，校验归档只读
    let cohort_id: (i64,) = sqlx::query_as(
        "SELECT cohort_id FROM homework WHERE id = ?1"
    )
    .bind(homework_id)
    .fetch_one(pool)
    .await
    .map_err(|_| "作业不存在".to_string())?;

    check_cohort_readonly(pool, cohort_id.0).await?;

    // 同时校验所有学生属于同一届次
    for student_id in &student_ids {
        let belongs: (i64,) = sqlx::query_as(
            "SELECT cohort_id FROM student WHERE id = ?1 AND deleted_at IS NULL"
        )
        .bind(student_id)
        .fetch_one(pool)
        .await
        .map_err(|_| format!("学生 ID {} 不存在或已删除", student_id))?;
        if belongs.0 != cohort_id.0 {
            return Err(format!(
                "学生 ID {} 不属于作业所属届次 (学生届次: {}, 作业届次: {})",
                student_id, belongs.0, cohort_id.0
            ));
        }
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for student_id in &student_ids {
        sqlx::query(
            "UPDATE homework_record SET status = ?1, submit_time = CASE WHEN ?1 = '已完成' THEN ?2 ELSE submit_time END, updated_at = ?2 WHERE homework_id = ?3 AND student_id = ?4"
        )
        .bind(&status)
        .bind(&now)
        .bind(homework_id)
        .bind(student_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn export_incomplete_homework(
    state: State<'_, AppState>,
    homework_id: i64,
    file_path: String,
) -> Result<(), String> {
    let pool = &state.db;
    let records = sqlx::query_as::<_, (String, String, String)>(
        "SELECT s.name, s.student_no, COALESCE(s.group_name, '')
         FROM homework_record hr
         JOIN student s ON hr.student_id = s.id
         WHERE hr.homework_id = ?1 AND hr.status IN ('未登记', '未完成') AND s.deleted_at IS NULL
         ORDER BY s.student_no ASC"
    )
    .bind(homework_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    use rust_xlsxwriter::*;
    let mut workbook = Workbook::new();
    let mut sheet = Worksheet::new();
    sheet.write_string(0, 0, "姓名").map_err(|e| e.to_string())?;
    sheet.write_string(0, 1, "学号").map_err(|e| e.to_string())?;
    sheet.write_string(0, 2, "小组").map_err(|e| e.to_string())?;
    for (i, (name, no, group)) in records.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet.write_string(row, 0, name).map_err(|e| e.to_string())?;
        sheet.write_string(row, 1, no).map_err(|e| e.to_string())?;
        sheet.write_string(row, 2, group).map_err(|e| e.to_string())?;
    }
    workbook.push_worksheet(sheet);
    workbook.save(&file_path).map_err(|e| format!("导出失败: {}", e))?;

    Ok(())
}

async fn get_homework_internal(pool: &sqlx::SqlitePool, id: i64) -> Result<Homework, String> {
    sqlx::query_as::<_, Homework>("SELECT * FROM homework WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取作业失败: {}", e))
}
