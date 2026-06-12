use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use super::cohort::check_cohort_readonly;
use super::student::ensure_student_belongs_to_cohort;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Attendance {
    pub id: i64,
    pub cohort_id: i64,
    pub student_id: i64,
    pub attendance_date: String,
    pub status: String,
    pub leave_type: Option<String>,
    pub leave_start_date: Option<String>,
    pub leave_end_date: Option<String>,
    pub reason: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AttendanceRecord {
    pub student_id: i64,
    pub status: String,
    pub leave_type: Option<String>,
    pub leave_start_date: Option<String>,
    pub leave_end_date: Option<String>,
    pub reason: Option<String>,
    pub remark: Option<String>,
}

fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| format!("无效日期格式: {}", value))
}

#[tauri::command]
pub async fn get_attendance_by_date(
    state: State<'_, AppState>,
    cohort_id: i64,
    date: String,
) -> Result<Vec<serde_json::Value>, String> {
    let pool = &state.db;
    let rows = sqlx::query(
        "SELECT a.id, a.cohort_id, a.student_id, a.attendance_date, a.status, a.leave_type, a.leave_start_date, a.leave_end_date, a.reason, a.remark, a.created_at, a.updated_at,
                s.name as student_name, s.student_no, s.group_name
         FROM attendance a
         JOIN student s ON a.student_id = s.id
         WHERE a.cohort_id = ?1 AND a.attendance_date = ?2 AND s.deleted_at IS NULL
         ORDER BY s.student_no ASC"
    )
    .bind(cohort_id)
    .bind(&date)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "cohort_id": r.get::<i64, _>("cohort_id"),
                "student_id": r.get::<i64, _>("student_id"),
                "attendance_date": r.get::<String, _>("attendance_date"),
                "status": r.get::<String, _>("status"),
                "leave_type": r.get::<Option<String>, _>("leave_type"),
                "leave_start_date": r.get::<Option<String>, _>("leave_start_date"),
                "leave_end_date": r.get::<Option<String>, _>("leave_end_date"),
                "reason": r.get::<Option<String>, _>("reason"),
                "remark": r.get::<Option<String>, _>("remark"),
                "created_at": r.get::<String, _>("created_at"),
                "updated_at": r.get::<String, _>("updated_at"),
                "student_name": r.get::<String, _>("student_name"),
                "student_no": r.get::<String, _>("student_no"),
                "group_name": r.get::<Option<String>, _>("group_name")
            })
        })
        .collect())
}

#[tauri::command]
pub async fn save_attendance(
    state: State<'_, AppState>,
    cohort_id: i64,
    date: String,
    records: Vec<AttendanceRecord>,
) -> Result<(), String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let valid_statuses = ["正常", "迟到", "早退", "请假", "旷课"];

    for record in &records {
        ensure_student_belongs_to_cohort(pool, record.student_id, cohort_id).await?;
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for record in &records {
        if !valid_statuses.contains(&record.status.as_str()) {
            return Err(format!("无效的考勤状态: {}", record.status));
        }
        if record.status == "请假" {
            let start = record
                .leave_start_date
                .clone()
                .unwrap_or_else(|| date.clone());
            let end = record
                .leave_end_date
                .clone()
                .unwrap_or_else(|| start.clone());
            let start_date = parse_date(&start)?;
            let end_date = parse_date(&end)?;
            if end_date < start_date {
                return Err("请假结束日期不能早于开始日期".to_string());
            }
            let mut current = start_date;
            while current <= end_date {
                let current_str = current.format("%Y-%m-%d").to_string();
                sqlx::query(
                    "INSERT INTO attendance (cohort_id, student_id, attendance_date, status, leave_type, leave_start_date, leave_end_date, reason, remark, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                     ON CONFLICT(student_id, attendance_date) DO UPDATE SET
                     status = excluded.status, leave_type = excluded.leave_type, leave_start_date = excluded.leave_start_date,
                     leave_end_date = excluded.leave_end_date, reason = excluded.reason, remark = excluded.remark, updated_at = excluded.updated_at"
                )
                .bind(cohort_id)
                .bind(record.student_id)
                .bind(&current_str)
                .bind(&record.status)
                .bind(&record.leave_type)
                .bind(&record.leave_start_date)
                .bind(&record.leave_end_date)
                .bind(&record.reason)
                .bind(&record.remark)
                .bind(&now)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                current += Duration::days(1);
            }
        } else {
            sqlx::query(
                "INSERT INTO attendance (cohort_id, student_id, attendance_date, status, leave_type, leave_start_date, leave_end_date, reason, remark, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, ?5, ?6, ?7, ?7)
                 ON CONFLICT(student_id, attendance_date) DO UPDATE SET
                 status = excluded.status, leave_type = NULL, leave_start_date = NULL, leave_end_date = NULL,
                 reason = excluded.reason, remark = excluded.remark, updated_at = excluded.updated_at"
            )
            .bind(cohort_id)
            .bind(record.student_id)
            .bind(&date)
            .bind(&record.status)
            .bind(&record.reason)
            .bind(&record.remark)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_all_attendance_normal(
    state: State<'_, AppState>,
    cohort_id: i64,
    date: String,
) -> Result<(), String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "INSERT OR REPLACE INTO attendance (cohort_id, student_id, attendance_date, status, leave_type, leave_start_date, leave_end_date, created_at, updated_at)
         SELECT ?1, id, ?2, '正常', NULL, NULL, NULL, ?3, ?3 FROM student
         WHERE cohort_id = ?1 AND deleted_at IS NULL AND status = '正常'"
    )
    .bind(cohort_id)
    .bind(&date)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("设置全部正常失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn query_attendance(
    state: State<'_, AppState>,
    cohort_id: i64,
    start_date: Option<String>,
    end_date: Option<String>,
    student_id: Option<i64>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let mut where_clauses = vec!["a.cohort_id = ?1".to_string()];
    let mut params: Vec<String> = vec![cohort_id.to_string()];
    let mut param_idx = 2;

    if let Some(ref sd) = start_date {
        where_clauses.push(format!("a.attendance_date >= ?{}", param_idx));
        params.push(sd.clone());
        param_idx += 1;
    }
    if let Some(ref ed) = end_date {
        where_clauses.push(format!("a.attendance_date <= ?{}", param_idx));
        params.push(ed.clone());
        param_idx += 1;
    }
    if let Some(sid) = student_id {
        where_clauses.push(format!("a.student_id = ?{}", param_idx));
        params.push(sid.to_string());
        param_idx += 1;
    }
    if let Some(ref st) = status {
        where_clauses.push(format!("a.status = ?{}", param_idx));
        params.push(st.clone());
        param_idx += 1;
    }

    let where_clause = where_clauses.join(" AND ");
    let count_query = format!("SELECT COUNT(*) FROM attendance a WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params {
        count_stmt = count_stmt.bind(p);
    }
    let (total,): (i64,) = count_stmt
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let data_query = format!(
        "SELECT a.id, a.cohort_id, a.student_id, a.attendance_date, a.status, a.leave_type, a.leave_start_date, a.leave_end_date, a.reason, a.remark, a.created_at, a.updated_at,
                s.name as student_name, s.student_no
         FROM attendance a JOIN student s ON a.student_id = s.id
         WHERE {} ORDER BY a.attendance_date DESC, s.student_no ASC LIMIT ?{} OFFSET ?{}",
        where_clause, param_idx, param_idx + 1
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
                "attendance_date": r.get::<String, _>("attendance_date"),
                "status": r.get::<String, _>("status"),
                "leave_type": r.get::<Option<String>, _>("leave_type"),
                "leave_start_date": r.get::<Option<String>, _>("leave_start_date"),
                "leave_end_date": r.get::<Option<String>, _>("leave_end_date"),
                "reason": r.get::<Option<String>, _>("reason"),
                "remark": r.get::<Option<String>, _>("remark"),
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
pub async fn attendance_statistics(
    state: State<'_, AppState>,
    cohort_id: i64,
    start_date: String,
    end_date: String,
) -> Result<Vec<serde_json::Value>, String> {
    let pool = &state.db;
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.student_no,
            COUNT(a.id) as total,
            COALESCE(SUM(CASE WHEN a.status = '正常' THEN 1 ELSE 0 END), 0) as normal,
            COALESCE(SUM(CASE WHEN a.status = '迟到' THEN 1 ELSE 0 END), 0) as late,
            COALESCE(SUM(CASE WHEN a.status = '早退' THEN 1 ELSE 0 END), 0) as early,
            COALESCE(SUM(CASE WHEN a.status = '请假' THEN 1 ELSE 0 END), 0) as leaves,
            COALESCE(SUM(CASE WHEN a.status = '旷课' THEN 1 ELSE 0 END), 0) as absent,
            CASE WHEN COUNT(a.id) > 0 THEN CAST(SUM(CASE WHEN a.status = '正常' THEN 1 ELSE 0 END) AS REAL) / COUNT(a.id) ELSE 0 END as rate
         FROM student s
         LEFT JOIN attendance a ON a.student_id = s.id AND a.attendance_date >= ?2 AND a.attendance_date <= ?3
         WHERE s.cohort_id = ?1 AND s.deleted_at IS NULL AND s.status = '正常'
         GROUP BY s.id
         ORDER BY s.student_no ASC"
    )
    .bind(cohort_id)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "student_id": r.get::<i64, _>("id"),
                "student_name": r.get::<String, _>("name"),
                "student_no": r.get::<String, _>("student_no"),
                "total": r.get::<i64, _>("total"),
                "normal": r.get::<i64, _>("normal"),
                "late": r.get::<i64, _>("late"),
                "early": r.get::<i64, _>("early"),
                "leave": r.get::<i64, _>("leaves"),
                "absent": r.get::<i64, _>("absent"),
                "attendance_rate": r.get::<f64, _>("rate")
            })
        })
        .collect())
}

#[tauri::command]
pub async fn attendance_statistics_cohort(
    state: State<'_, AppState>,
    cohort_id: i64,
    start_date: String,
    end_date: String,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.student_no,
            COUNT(a.id) as total,
            COALESCE(SUM(CASE WHEN a.status = '正常' THEN 1 ELSE 0 END), 0) as normal,
            COALESCE(SUM(CASE WHEN a.status = '迟到' THEN 1 ELSE 0 END), 0) as late,
            COALESCE(SUM(CASE WHEN a.status = '早退' THEN 1 ELSE 0 END), 0) as early,
            COALESCE(SUM(CASE WHEN a.status = '请假' THEN 1 ELSE 0 END), 0) as leaves,
            COALESCE(SUM(CASE WHEN a.status = '旷课' THEN 1 ELSE 0 END), 0) as absent,
            CASE WHEN COUNT(a.id) > 0 THEN CAST(SUM(CASE WHEN a.status = '正常' THEN 1 ELSE 0 END) AS REAL) / COUNT(a.id) ELSE 0 END as rate
         FROM student s
         LEFT JOIN attendance a ON a.student_id = s.id AND a.attendance_date >= ?2 AND a.attendance_date <= ?3
         WHERE s.cohort_id = ?1 AND s.deleted_at IS NULL AND s.status = '正常'
         GROUP BY s.id
         ORDER BY s.student_no ASC"
    )
    .bind(cohort_id)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let records: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "student_id": r.get::<i64, _>("id"),
                "student_name": r.get::<String, _>("name"),
                "student_no": r.get::<String, _>("student_no"),
                "total": r.get::<i64, _>("total"),
                "normal": r.get::<i64, _>("normal"),
                "late": r.get::<i64, _>("late"),
                "early": r.get::<i64, _>("early"),
                "leave": r.get::<i64, _>("leaves"),
                "absent": r.get::<i64, _>("absent"),
                "rate": r.get::<f64, _>("rate")
            })
        })
        .collect();

    let (total_days,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT attendance_date) FROM attendance WHERE cohort_id = ?1 AND attendance_date >= ?2 AND attendance_date <= ?3"
    )
    .bind(cohort_id)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "total_days": total_days, "records": records }))
}

#[tauri::command]
pub async fn export_attendance_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<(), String> {
    let pool = &state.db;

    let use_range = start_date.is_some() && end_date.is_some();
    let rows = if use_range {
        let start = start_date.unwrap();
        let end = end_date.unwrap();
        sqlx::query(
            "SELECT a.attendance_date, s.name, s.student_no, a.status, a.leave_type, a.leave_start_date, a.leave_end_date, a.reason
             FROM attendance a JOIN student s ON a.student_id = s.id
             WHERE a.cohort_id = ?1 AND s.deleted_at IS NULL
               AND a.attendance_date >= ?2 AND a.attendance_date <= ?3
             ORDER BY a.attendance_date DESC, s.student_no ASC"
        )
        .bind(cohort_id)
        .bind(&start)
        .bind(&end)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query(
            "SELECT a.attendance_date, s.name, s.student_no, a.status, a.leave_type, a.leave_start_date, a.leave_end_date, a.reason
             FROM attendance a JOIN student s ON a.student_id = s.id
             WHERE a.cohort_id = ?1 AND s.deleted_at IS NULL
             ORDER BY a.attendance_date DESC, s.student_no ASC"
        )
        .bind(cohort_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    };

    use rust_xlsxwriter::*;
    let mut workbook = Workbook::new();
    let mut sheet = Worksheet::new();
    let headers = [
        "日期",
        "姓名",
        "学号",
        "状态",
        "请假类型",
        "开始日期",
        "结束日期",
        "原因",
    ];
    for (ci, h) in headers.iter().enumerate() {
        sheet
            .write_string(0, ci as u16, *h)
            .map_err(|e| e.to_string())?;
    }
    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet
            .write_string(row, 0, &r.get::<String, _>(0))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 1, &r.get::<String, _>(1))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 2, &r.get::<String, _>(2))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 3, &r.get::<String, _>(3))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 4, &r.get::<Option<String>, _>(4).unwrap_or_default())
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 5, &r.get::<Option<String>, _>(5).unwrap_or_default())
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 6, &r.get::<Option<String>, _>(6).unwrap_or_default())
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 7, &r.get::<Option<String>, _>(7).unwrap_or_default())
            .map_err(|e| e.to_string())?;
    }
    workbook.push_worksheet(sheet);
    workbook
        .save(&file_path)
        .map_err(|e| format!("导出失败: {}", e))?;
    Ok(())
}
