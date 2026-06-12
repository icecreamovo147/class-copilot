use chrono::Local;
use rust_xlsxwriter::Workbook;
use sqlx::Row;
use std::{fs, process::Command};
use tauri::State;

use crate::AppState;

fn create_pdf_from_plain_text(file_path: &str, content: &str) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let temp_dir = std::env::temp_dir().join(format!(
            "class_copilot_pdf_{}",
            Local::now().timestamp_millis()
        ));
        fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {}", e))?;
        let text_path = temp_dir.join("report.txt");
        fs::write(&text_path, content).map_err(|e| format!("写入临时报表失败: {}", e))?;

        let output = Command::new("cupsfilter")
            .args(["-m", "application/pdf"])
            .arg(&text_path)
            .output()
            .map_err(|e| format!("调用 PDF 渲染器失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(format!("生成 PDF 失败: {}", stderr.trim()));
        }

        fs::write(file_path, output.stdout).map_err(|e| format!("保存 PDF 失败: {}", e))?;
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let _ = file_path;
        let _ = content;
        Err("当前平台尚未启用 PDF 导出".to_string())
    }
}

async fn build_cohort_statistics_summary(
    pool: &sqlx::SqlitePool,
    cohort_id: i64,
) -> Result<serde_json::Value, String> {
    let (total_homework, avg_homework_rate, total_incomplete): (i64, f64, i64) = sqlx::query_as(
        "SELECT COUNT(*) as total,
            COALESCE(AVG(CASE WHEN total_records > 0 THEN CAST(completed AS REAL) / total_records ELSE 0 END), 0) as avg_rate,
            COALESCE(SUM(incomplete), 0) as total_incomplete
         FROM (
             SELECT h.id,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id) as total_records,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id AND status = '已完成') as completed,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id AND status IN ('未登记', '未完成')) as incomplete
             FROM homework h WHERE h.cohort_id = ?1 AND h.deleted_at IS NULL
         )"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let (
        attendance_total,
        attendance_normal,
        attendance_late,
        attendance_early,
        attendance_leave,
        attendance_absent,
    ): (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*),
                SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = '迟到' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = '早退' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = '请假' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = '旷课' THEN 1 ELSE 0 END)
             FROM attendance WHERE cohort_id = ?1",
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let (exams_count, subjects_count, avg_score): (i64, i64, Option<f64>) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM exam WHERE cohort_id = ?1 AND deleted_at IS NULL),
            (SELECT COUNT(DISTINCT sc.subject_id) FROM score sc JOIN exam e ON sc.exam_id = e.id WHERE e.cohort_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL),
            (SELECT AVG(sc.score_value) FROM score sc JOIN exam e ON sc.exam_id = e.id WHERE e.cohort_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL)"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "homework": {
            "total": total_homework,
            "avg_rate": avg_homework_rate,
            "total_incomplete": total_incomplete
        },
        "attendance": {
            "total": attendance_total,
            "normal": attendance_normal,
            "late": attendance_late,
            "early": attendance_early,
            "leave": attendance_leave,
            "absent": attendance_absent,
            "rate": if attendance_total > 0 { attendance_normal as f64 / attendance_total as f64 } else { 0.0 }
        },
        "scores": {
            "exams_count": exams_count,
            "subjects_count": subjects_count,
            "avg_score": avg_score.unwrap_or(0.0)
        }
    }))
}

#[tauri::command]
pub async fn get_dashboard_stats(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    let cohort = sqlx::query_as::<_, (String, String, String)>(
        "SELECT cohort_name, class_name, status FROM cohort WHERE id = ?1",
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let (total_students, male_count, female_count): (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), SUM(CASE WHEN gender = '男' THEN 1 ELSE 0 END), SUM(CASE WHEN gender = '女' THEN 1 ELSE 0 END)
         FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL AND status = '正常'"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let (hw_count, hw_total_records, hw_completed_records, hw_rate): (i64, i64, i64, f64) =
        sqlx::query_as(
            "SELECT COUNT(DISTINCT h.id) as hw_count,
            COUNT(hr.id) as total_records,
            COUNT(CASE WHEN hr.status = '已完成' THEN 1 END) as completed_records,
            CASE WHEN COUNT(hr.id) > 0
                THEN CAST(COUNT(CASE WHEN hr.status = '已完成' THEN 1 END) AS REAL) /
                     CAST(COUNT(hr.id) AS REAL)
                ELSE 0.0 END as rate
         FROM homework h
         LEFT JOIN homework_record hr ON hr.homework_id = h.id
         WHERE h.cohort_id = ?1 AND h.deleted_at IS NULL AND h.publish_date = ?2",
        )
        .bind(cohort_id)
        .bind(&today)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let (normal, late, early, leave_, absent): (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
            SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '迟到' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '早退' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '请假' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = '旷课' THEN 1 ELSE 0 END)
         FROM attendance WHERE cohort_id = ?1 AND attendance_date = ?2",
    )
    .bind(cohort_id)
    .bind(&today)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let (pending_hw,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM homework WHERE cohort_id = ?1 AND deleted_at IS NULL AND publish_date <= ?2
         AND (SELECT COUNT(*) FROM homework_record WHERE homework_id = homework.id AND status != '未登记') <
             (SELECT COUNT(*) FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL AND status = '正常')"
    )
    .bind(cohort_id)
    .bind(&today)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let attendance_today = normal + late + early + leave_ + absent;

    let focus = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, student_no FROM student
         WHERE cohort_id = ?1 AND deleted_at IS NULL AND is_focus = 1
         ORDER BY student_no ASC LIMIT 10",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let focus_students: Vec<serde_json::Value> = focus
        .into_iter()
        .map(|(id, name, no)| {
            serde_json::json!({
                "id": id, "name": name, "student_no": no, "reason": "重点关注"
            })
        })
        .collect();

    Ok(serde_json::json!({
        "cohort_name": cohort.0,
        "class_name": cohort.1,
        "status": cohort.2,
        "total_students": total_students,
        "male_count": male_count,
        "female_count": female_count,
        "today_homework_count": hw_count,
        "today_homework_total_records": hw_total_records,
        "today_homework_completed": hw_completed_records,
        "today_homework_rate": hw_rate,
        "today_attendance_normal": normal,
        "today_attendance_late": late,
        "today_attendance_early": early,
        "today_attendance_leave": leave_,
        "today_attendance_absent": absent,
        "pending_homework": pending_hw,
        "pending_attendance": attendance_today == 0,
        "focus_students": focus_students
    }))
}

#[tauri::command]
pub async fn homework_statistics(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    let (total, avg_rate, total_incomplete): (i64, f64, i64) = sqlx::query_as(
        "SELECT COUNT(*) as total,
            COALESCE(AVG(CASE WHEN total_records > 0 THEN CAST(completed AS REAL) / total_records ELSE 0 END), 0) as avg_rate,
            COALESCE(SUM(incomplete), 0) as total_incomplete
         FROM (
             SELECT h.id,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id) as total_records,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id AND status = '已完成') as completed,
                 (SELECT COUNT(*) FROM homework_record WHERE homework_id = h.id AND status IN ('未登记', '未完成')) as incomplete
             FROM homework h WHERE h.cohort_id = ?1 AND h.deleted_at IS NULL
         )"
    )
    .bind(cohort_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let consecutive = sqlx::query_as::<_, (i64, String, String, i64)>(
        "SELECT s.id, s.name, s.student_no, COUNT(*) as c
         FROM student s
         JOIN homework_record hr ON hr.student_id = s.id
         JOIN homework h ON h.id = hr.homework_id AND h.cohort_id = ?1
         WHERE hr.status IN ('未登记', '未完成') AND s.deleted_at IS NULL
         GROUP BY s.id HAVING c >= 3
         ORDER BY c DESC LIMIT 20",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let consecutive_incomplete: Vec<serde_json::Value> = consecutive
        .into_iter()
        .map(|(id, name, no, c)| {
            serde_json::json!({ "student_id": id, "student_name": name, "student_no": no, "count": c })
        })
        .collect();

    Ok(serde_json::json!({
        "total": total,
        "avg_rate": avg_rate,
        "total_incomplete": total_incomplete,
        "consecutive_incomplete": consecutive_incomplete
    }))
}

#[tauri::command]
pub async fn homework_trend_statistics(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "SELECT h.id, h.title, h.publish_date,
            COUNT(hr.id) as total_count,
            SUM(CASE WHEN hr.status = '已完成' THEN 1 ELSE 0 END) as completed_count,
            SUM(CASE WHEN hr.status IN ('未登记', '未完成') THEN 1 ELSE 0 END) as incomplete_count
         FROM homework h
         LEFT JOIN homework_record hr ON hr.homework_id = h.id
         WHERE h.cohort_id = ?1 AND h.deleted_at IS NULL
         GROUP BY h.id, h.title, h.publish_date
         ORDER BY h.publish_date ASC, h.id ASC",
    )
    .bind(cohort_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let total_count = row.get::<i64, _>("total_count");
            let completed_count = row.get::<i64, _>("completed_count");
            serde_json::json!({
                "homework_id": row.get::<i64, _>("id"),
                "title": row.get::<String, _>("title"),
                "publish_date": row.get::<String, _>("publish_date"),
                "total_count": total_count,
                "completed_count": completed_count,
                "incomplete_count": row.get::<i64, _>("incomplete_count"),
                "completion_rate": if total_count > 0 { completed_count as f64 / total_count as f64 } else { 0.0 }
            })
        })
        .collect())
}

#[tauri::command]
pub async fn attendance_trend_statistics(
    state: State<'_, AppState>,
    cohort_id: i64,
    start_date: String,
    end_date: String,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "SELECT attendance_date,
            COUNT(*) as total_count,
            SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END) as normal_count,
            SUM(CASE WHEN status = '迟到' THEN 1 ELSE 0 END) as late_count,
            SUM(CASE WHEN status = '早退' THEN 1 ELSE 0 END) as early_count,
            SUM(CASE WHEN status = '请假' THEN 1 ELSE 0 END) as leave_count,
            SUM(CASE WHEN status = '旷课' THEN 1 ELSE 0 END) as absent_count
         FROM attendance
         WHERE cohort_id = ?1 AND attendance_date BETWEEN ?2 AND ?3
         GROUP BY attendance_date
         ORDER BY attendance_date ASC",
    )
    .bind(cohort_id)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let total = row.get::<i64, _>("total_count");
            let normal = row.get::<i64, _>("normal_count");
            serde_json::json!({
                "attendance_date": row.get::<String, _>("attendance_date"),
                "total_count": total,
                "normal_count": normal,
                "late_count": row.get::<i64, _>("late_count"),
                "early_count": row.get::<i64, _>("early_count"),
                "leave_count": row.get::<i64, _>("leave_count"),
                "absent_count": row.get::<i64, _>("absent_count"),
                "normal_rate": if total > 0 { normal as f64 / total as f64 } else { 0.0 }
            })
        })
        .collect())
}

#[tauri::command]
pub async fn score_statistics_cohort(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    let (exams_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM exam WHERE cohort_id = ?1 AND deleted_at IS NULL")
            .bind(cohort_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;

    let (subjects_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM subject")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let records = sqlx::query_as::<_, (String, String, f64, f64, f64)>(
        "SELECT e.name, sub.name,
            AVG(sc.score_value), MAX(sc.score_value), MIN(sc.score_value)
         FROM score sc
         JOIN exam e ON sc.exam_id = e.id
         JOIN subject sub ON sc.subject_id = sub.id
         WHERE e.cohort_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL
         GROUP BY e.id, sub.id
         ORDER BY e.exam_date DESC",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|(exam, sub, avg, max, min)| {
            serde_json::json!({
                "exam_name": exam, "subject_name": sub, "avg_score": avg, "max_score": max, "min_score": min
            })
        })
        .collect();

    Ok(serde_json::json!({
        "exams_count": exams_count,
        "subjects_count": subjects_count,
        "records": records_json
    }))
}

#[tauri::command]
pub async fn score_trend_statistics(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "SELECT e.id, e.name, COALESCE(e.exam_date, e.created_at) as exam_point, sub.name as subject_name,
            AVG(sc.score_value) as avg_score
         FROM score sc
         JOIN exam e ON sc.exam_id = e.id
         JOIN subject sub ON sc.subject_id = sub.id
         WHERE e.cohort_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL
         GROUP BY e.id, sub.id
         ORDER BY exam_point ASC, e.id ASC, sub.sort_order ASC, sub.id ASC"
    )
    .bind(cohort_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "exam_id": row.get::<i64, _>("id"),
                "exam_name": row.get::<String, _>("name"),
                "exam_point": row.get::<String, _>("exam_point"),
                "subject_name": row.get::<String, _>("subject_name"),
                "avg_score": row.get::<f64, _>("avg_score")
            })
        })
        .collect())
}

#[tauri::command]
pub async fn cross_cohort_comparison(
    state: State<'_, AppState>,
    cohort_ids: Vec<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let pool = &state.db;
    let mut results = Vec::new();

    for cohort_id in cohort_ids {
        let row = sqlx::query("SELECT cohort_name, class_name, status FROM cohort WHERE id = ?1")
            .bind(cohort_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;

        let (student_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL AND status = '正常'"
        )
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

        let homework_stats = sqlx::query(
            "SELECT COUNT(hr.id) as total_records,
                SUM(CASE WHEN hr.status = '已完成' THEN 1 ELSE 0 END) as completed_records
             FROM homework h
             LEFT JOIN homework_record hr ON hr.homework_id = h.id
             WHERE h.cohort_id = ?1 AND h.deleted_at IS NULL",
        )
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
        let homework_total = homework_stats.get::<i64, _>("total_records");
        let homework_completed = homework_stats.get::<i64, _>("completed_records");

        let attendance_stats = sqlx::query(
            "SELECT COUNT(*) as total_records,
                SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END) as normal_records
             FROM attendance
             WHERE cohort_id = ?1",
        )
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
        let attendance_total = attendance_stats.get::<i64, _>("total_records");
        let attendance_normal = attendance_stats.get::<i64, _>("normal_records");

        let score_stats = sqlx::query(
            "SELECT COUNT(score_value) as scored_count, AVG(score_value) as avg_score
             FROM score sc
             JOIN exam e ON sc.exam_id = e.id
             WHERE e.cohort_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL",
        )
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
        let score_count = score_stats.get::<i64, _>("scored_count");
        let avg_score = score_stats
            .get::<Option<f64>, _>("avg_score")
            .unwrap_or(0.0);

        let behavior_stats = sqlx::query(
            "SELECT COUNT(*) as total_count, COALESCE(SUM(score), 0) as total_score
             FROM behavior_record
             WHERE cohort_id = ?1",
        )
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

        results.push(serde_json::json!({
            "cohort_id": cohort_id,
            "cohort_name": row.get::<String, _>("cohort_name"),
            "class_name": row.get::<String, _>("class_name"),
            "status": row.get::<String, _>("status"),
            "student_count": student_count,
            "homework_completion_rate": if homework_total > 0 { homework_completed as f64 / homework_total as f64 } else { 0.0 },
            "attendance_rate": if attendance_total > 0 { attendance_normal as f64 / attendance_total as f64 } else { 0.0 },
            "avg_score": if score_count > 0 { avg_score } else { 0.0 },
            "missing_score_data": score_count == 0,
            "behavior_count": behavior_stats.get::<i64, _>("total_count"),
            "behavior_score_total": behavior_stats.get::<i64, _>("total_score")
        }));
    }

    Ok(results)
}

#[tauri::command]
pub async fn export_cross_cohort_comparison(
    state: State<'_, AppState>,
    cohort_ids: Vec<i64>,
    file_path: String,
) -> Result<(), String> {
    let comparisons = cross_cohort_comparison(state, cohort_ids).await?;
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = [
        "届次名称",
        "班级名称",
        "状态",
        "学生人数",
        "作业完成率",
        "出勤率",
        "平均成绩",
        "成绩缺失",
        "奖惩次数",
        "奖惩分值合计",
    ];
    for (idx, header) in headers.iter().enumerate() {
        worksheet
            .write_string(0, idx as u16, *header)
            .map_err(|e| e.to_string())?;
    }
    for (row_idx, item) in comparisons.iter().enumerate() {
        let line = (row_idx + 1) as u32;
        worksheet
            .write_string(line, 0, item["cohort_name"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 1, item["class_name"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(line, 2, item["status"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(line, 3, item["student_count"].as_i64().unwrap_or(0) as f64)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(
                line,
                4,
                item["homework_completion_rate"].as_f64().unwrap_or(0.0),
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(line, 5, item["attendance_rate"].as_f64().unwrap_or(0.0))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(line, 6, item["avg_score"].as_f64().unwrap_or(0.0))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(
                line,
                7,
                if item["missing_score_data"].as_bool().unwrap_or(false) {
                    "是"
                } else {
                    "否"
                },
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(line, 8, item["behavior_count"].as_i64().unwrap_or(0) as f64)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(
                line,
                9,
                item["behavior_score_total"].as_i64().unwrap_or(0) as f64,
            )
            .map_err(|e| e.to_string())?;
    }
    workbook
        .save(file_path)
        .map_err(|e| format!("导出跨届对比失败: {}", e))
}

#[tauri::command]
pub async fn export_cross_cohort_comparison_pdf(
    state: State<'_, AppState>,
    cohort_ids: Vec<i64>,
    file_path: String,
) -> Result<(), String> {
    let comparisons = cross_cohort_comparison(state, cohort_ids).await?;
    let mut lines = vec![
        "跨届对比统计报表".to_string(),
        format!("导出时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S")),
        "".to_string(),
    ];
    for item in comparisons {
        lines.push(format!(
            "{} {} [{}]",
            item["cohort_name"].as_str().unwrap_or(""),
            item["class_name"].as_str().unwrap_or(""),
            item["status"].as_str().unwrap_or("")
        ));
        lines.push(format!(
            "学生人数: {}",
            item["student_count"].as_i64().unwrap_or(0)
        ));
        lines.push(format!(
            "作业完成率: {:.1}%",
            item["homework_completion_rate"].as_f64().unwrap_or(0.0) * 100.0
        ));
        lines.push(format!(
            "出勤率: {:.1}%",
            item["attendance_rate"].as_f64().unwrap_or(0.0) * 100.0
        ));
        lines.push(if item["missing_score_data"].as_bool().unwrap_or(false) {
            "平均成绩: 缺失".to_string()
        } else {
            format!("平均成绩: {:.1}", item["avg_score"].as_f64().unwrap_or(0.0))
        });
        lines.push(format!(
            "奖惩次数: {}  奖惩分值: {}",
            item["behavior_count"].as_i64().unwrap_or(0),
            item["behavior_score_total"].as_i64().unwrap_or(0)
        ));
        lines.push("".to_string());
    }
    create_pdf_from_plain_text(&file_path, &lines.join("\n"))
}

#[tauri::command]
pub async fn export_cohort_statistics_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
) -> Result<(), String> {
    let summary = build_cohort_statistics_summary(&state.db, cohort_id).await?;
    let cohort = sqlx::query_as::<_, (String, String)>(
        "SELECT cohort_name, class_name FROM cohort WHERE id = ?1",
    )
    .bind(cohort_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet
        .write_string(0, 0, "统计报表")
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(1, 0, "届次")
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(1, 1, &format!("{} {}", cohort.0, cohort.1))
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(2, 0, "导出时间")
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(2, 1, &Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(4, 0, "作业总数")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(
            4,
            1,
            summary["homework"]["total"].as_i64().unwrap_or(0) as f64,
        )
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(5, 0, "作业平均完成率")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(
            5,
            1,
            summary["homework"]["avg_rate"].as_f64().unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(6, 0, "总未交次数")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(
            6,
            1,
            summary["homework"]["total_incomplete"]
                .as_i64()
                .unwrap_or(0) as f64,
        )
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(8, 0, "考勤总记录")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(
            8,
            1,
            summary["attendance"]["total"].as_i64().unwrap_or(0) as f64,
        )
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(9, 0, "出勤率")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(9, 1, summary["attendance"]["rate"].as_f64().unwrap_or(0.0))
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(10, 0, "迟到/早退/请假/旷课")
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(
            10,
            1,
            &format!(
                "{}/{}/{}/{}",
                summary["attendance"]["late"].as_i64().unwrap_or(0),
                summary["attendance"]["early"].as_i64().unwrap_or(0),
                summary["attendance"]["leave"].as_i64().unwrap_or(0),
                summary["attendance"]["absent"].as_i64().unwrap_or(0)
            ),
        )
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(12, 0, "考试数量")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(
            12,
            1,
            summary["scores"]["exams_count"].as_i64().unwrap_or(0) as f64,
        )
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(13, 0, "科目数量")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(
            13,
            1,
            summary["scores"]["subjects_count"].as_i64().unwrap_or(0) as f64,
        )
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(14, 0, "平均成绩")
        .map_err(|e| e.to_string())?;
    sheet
        .write_number(
            14,
            1,
            summary["scores"]["avg_score"].as_f64().unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;
    workbook
        .save(file_path)
        .map_err(|e| format!("导出统计报表失败: {}", e))
}

#[tauri::command]
pub async fn export_cohort_statistics_pdf(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
) -> Result<(), String> {
    let summary = build_cohort_statistics_summary(&state.db, cohort_id).await?;
    let cohort = sqlx::query_as::<_, (String, String)>(
        "SELECT cohort_name, class_name FROM cohort WHERE id = ?1",
    )
    .bind(cohort_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    let lines = vec![
        "班级统计报表".to_string(),
        format!("届次: {} {}", cohort.0, cohort.1),
        format!("导出时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S")),
        "".to_string(),
        "作业统计".to_string(),
        format!(
            "作业总数: {}",
            summary["homework"]["total"].as_i64().unwrap_or(0)
        ),
        format!(
            "平均完成率: {:.1}%",
            summary["homework"]["avg_rate"].as_f64().unwrap_or(0.0) * 100.0
        ),
        format!(
            "总未交次数: {}",
            summary["homework"]["total_incomplete"]
                .as_i64()
                .unwrap_or(0)
        ),
        "".to_string(),
        "考勤统计".to_string(),
        format!(
            "总记录数: {}",
            summary["attendance"]["total"].as_i64().unwrap_or(0)
        ),
        format!(
            "出勤率: {:.1}%",
            summary["attendance"]["rate"].as_f64().unwrap_or(0.0) * 100.0
        ),
        format!(
            "迟到/早退/请假/旷课: {}/{}/{}/{}",
            summary["attendance"]["late"].as_i64().unwrap_or(0),
            summary["attendance"]["early"].as_i64().unwrap_or(0),
            summary["attendance"]["leave"].as_i64().unwrap_or(0),
            summary["attendance"]["absent"].as_i64().unwrap_or(0)
        ),
        "".to_string(),
        "成绩统计".to_string(),
        format!(
            "考试数量: {}",
            summary["scores"]["exams_count"].as_i64().unwrap_or(0)
        ),
        format!(
            "科目数量: {}",
            summary["scores"]["subjects_count"].as_i64().unwrap_or(0)
        ),
        format!(
            "平均成绩: {:.1}",
            summary["scores"]["avg_score"].as_f64().unwrap_or(0.0)
        ),
    ];
    create_pdf_from_plain_text(&file_path, &lines.join("\n"))
}

#[tauri::command]
pub async fn get_student_profile(
    state: State<'_, AppState>,
    student_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;

    let student =
        sqlx::query_as::<_, super::student::Student>("SELECT * FROM student WHERE id = ?1")
            .bind(student_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("获取学生信息失败: {}", e))?;

    let (hw_total, hw_completed, hw_rate, consecutive): (i64, i64, f64, i64) = sqlx::query_as(
        "SELECT COUNT(*),
            SUM(CASE WHEN status = '已完成' THEN 1 ELSE 0 END),
            CASE WHEN COUNT(*) > 0 THEN CAST(SUM(CASE WHEN status = '已完成' THEN 1 ELSE 0 END) AS REAL) / COUNT(*) ELSE 0 END,
            (SELECT COUNT(*) FROM homework_record hr2
             JOIN homework h2 ON hr2.homework_id = h2.id
             WHERE hr2.student_id = ?1 AND hr2.status IN ('未登记', '未完成')
             AND h2.deleted_at IS NULL
             AND hr2.id >= COALESCE(
                 (SELECT MAX(id) FROM homework_record WHERE student_id = ?1 AND status = '已完成' AND homework_id IN (SELECT id FROM homework WHERE deleted_at IS NULL)),
                 0
             ))
         FROM homework_record hr
         JOIN homework h ON hr.homework_id = h.id
         WHERE hr.student_id = ?1 AND h.deleted_at IS NULL"
    )
    .bind(student_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let (att_total, att_normal, att_abnormal, att_rate): (i64, i64, i64, f64) = sqlx::query_as(
        "SELECT COUNT(*),
            SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status != '正常' THEN 1 ELSE 0 END),
            CASE WHEN COUNT(*) > 0 THEN CAST(SUM(CASE WHEN status = '正常' THEN 1 ELSE 0 END) AS REAL) / COUNT(*) ELSE 0 END
         FROM attendance WHERE student_id = ?1"
    )
    .bind(student_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let scores = sqlx::query(
        "SELECT e.name as exam_name, COALESCE(e.exam_date, e.created_at) as exam_point, sub.name as subject_name, sc.score_value
         FROM score sc
         JOIN exam e ON sc.exam_id = e.id
         JOIN subject sub ON sc.subject_id = sub.id
         WHERE sc.student_id = ?1 AND e.deleted_at IS NULL AND sc.score_value IS NOT NULL
         ORDER BY exam_point DESC, e.id DESC"
    )
    .bind(student_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let scores_json: Vec<serde_json::Value> = scores
        .iter()
        .map(|row| {
            serde_json::json!({
                "exam_name": row.get::<String, _>("exam_name"),
                "exam_point": row.get::<String, _>("exam_point"),
                "subject_name": row.get::<String, _>("subject_name"),
                "score_value": row.get::<Option<f64>, _>("score_value")
            })
        })
        .collect();

    let behaviors = sqlx::query_as::<_, super::affair::BehaviorRecord>(
        "SELECT * FROM behavior_record WHERE student_id = ?1 ORDER BY record_date DESC LIMIT 50",
    )
    .bind(student_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let behavior_score_total: i64 = behaviors.iter().map(|item| item.score as i64).sum();
    let focus_reasons = {
        let mut reasons = Vec::new();
        if student.is_focus {
            reasons.push("已被标记为重点关注".to_string());
        }
        if consecutive >= 3 {
            reasons.push(format!("连续未交作业 {} 次", consecutive));
        }
        if att_abnormal >= 3 {
            reasons.push(format!("考勤异常 {} 次", att_abnormal));
        }
        if behavior_score_total < 0 {
            reasons.push(format!("奖惩累计分值为 {}", behavior_score_total));
        }
        reasons
    };
    let overall_evaluation = if focus_reasons.is_empty() {
        "整体表现稳定，可继续保持当前跟进节奏。".to_string()
    } else {
        format!("建议重点跟进：{}。", focus_reasons.join("；"))
    };

    Ok(serde_json::json!({
        "student": student,
        "homework": {
            "total": hw_total,
            "completed": hw_completed,
            "rate": hw_rate,
            "consecutive_incomplete": consecutive
        },
        "attendance": {
            "total": att_total,
            "normal": att_normal,
            "abnormal": att_abnormal,
            "rate": att_rate
        },
        "scores": scores_json,
        "score_trend": scores_json,
        "behaviors": behaviors,
        "focus_reasons": focus_reasons,
        "overall_evaluation": overall_evaluation
    }))
}

#[tauri::command]
pub async fn export_student_growth_archive(
    state: State<'_, AppState>,
    student_id: i64,
    file_path: String,
) -> Result<(), String> {
    let profile = get_student_profile(state, student_id).await?;
    let mut workbook = Workbook::new();

    {
        let base = workbook.add_worksheet();
        base.write_string(0, 0, "学生成长档案")
            .map_err(|e| e.to_string())?;
        base.write_string(1, 0, "姓名").map_err(|e| e.to_string())?;
        base.write_string(1, 1, profile["student"]["name"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        base.write_string(2, 0, "学号").map_err(|e| e.to_string())?;
        base.write_string(
            2,
            1,
            profile["student"]["student_no"].as_str().unwrap_or(""),
        )
        .map_err(|e| e.to_string())?;
        base.write_string(3, 0, "作业完成率")
            .map_err(|e| e.to_string())?;
        base.write_number(3, 1, profile["homework"]["rate"].as_f64().unwrap_or(0.0))
            .map_err(|e| e.to_string())?;
        base.write_string(4, 0, "出勤率")
            .map_err(|e| e.to_string())?;
        base.write_number(4, 1, profile["attendance"]["rate"].as_f64().unwrap_or(0.0))
            .map_err(|e| e.to_string())?;
        base.write_string(5, 0, "关注原因")
            .map_err(|e| e.to_string())?;
        base.write_string(
            5,
            1,
            &profile["focus_reasons"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .collect::<Vec<_>>()
                        .join("；")
                })
                .unwrap_or_default(),
        )
        .map_err(|e| e.to_string())?;
        base.write_string(6, 0, "综合评价")
            .map_err(|e| e.to_string())?;
        base.write_string(6, 1, profile["overall_evaluation"].as_str().unwrap_or(""))
            .map_err(|e| e.to_string())?;
    }

    {
        let scores = workbook.add_worksheet();
        scores.set_name("成绩趋势").map_err(|e| e.to_string())?;
        scores
            .write_string(0, 0, "考试")
            .map_err(|e| e.to_string())?;
        scores
            .write_string(0, 1, "日期")
            .map_err(|e| e.to_string())?;
        scores
            .write_string(0, 2, "科目")
            .map_err(|e| e.to_string())?;
        scores
            .write_string(0, 3, "成绩")
            .map_err(|e| e.to_string())?;
        if let Some(items) = profile["score_trend"].as_array() {
            for (idx, item) in items.iter().enumerate() {
                let line = (idx + 1) as u32;
                scores
                    .write_string(line, 0, item["exam_name"].as_str().unwrap_or(""))
                    .map_err(|e| e.to_string())?;
                scores
                    .write_string(line, 1, item["exam_point"].as_str().unwrap_or(""))
                    .map_err(|e| e.to_string())?;
                scores
                    .write_string(line, 2, item["subject_name"].as_str().unwrap_or(""))
                    .map_err(|e| e.to_string())?;
                if let Some(score) = item["score_value"].as_f64() {
                    scores
                        .write_number(line, 3, score)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    {
        let behaviors = workbook.add_worksheet();
        behaviors.set_name("奖惩记录").map_err(|e| e.to_string())?;
        behaviors
            .write_string(0, 0, "日期")
            .map_err(|e| e.to_string())?;
        behaviors
            .write_string(0, 1, "类型")
            .map_err(|e| e.to_string())?;
        behaviors
            .write_string(0, 2, "标题")
            .map_err(|e| e.to_string())?;
        behaviors
            .write_string(0, 3, "分值")
            .map_err(|e| e.to_string())?;
        if let Some(items) = profile["behaviors"].as_array() {
            for (idx, item) in items.iter().enumerate() {
                let line = (idx + 1) as u32;
                behaviors
                    .write_string(line, 0, item["record_date"].as_str().unwrap_or(""))
                    .map_err(|e| e.to_string())?;
                behaviors
                    .write_string(line, 1, item["type"].as_str().unwrap_or(""))
                    .map_err(|e| e.to_string())?;
                behaviors
                    .write_string(line, 2, item["title"].as_str().unwrap_or(""))
                    .map_err(|e| e.to_string())?;
                behaviors
                    .write_number(line, 3, item["score"].as_i64().unwrap_or(0) as f64)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    workbook
        .save(file_path)
        .map_err(|e| format!("导出成长档案失败: {}", e))
}

#[tauri::command]
pub async fn export_student_growth_archive_pdf(
    state: State<'_, AppState>,
    student_id: i64,
    file_path: String,
) -> Result<(), String> {
    let profile = get_student_profile(state, student_id).await?;
    let mut lines = vec![
        "学生成长档案".to_string(),
        format!(
            "姓名: {}",
            profile["student"]["name"].as_str().unwrap_or("")
        ),
        format!(
            "学号: {}",
            profile["student"]["student_no"].as_str().unwrap_or("")
        ),
        format!("导出时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S")),
        "".to_string(),
        "作业表现".to_string(),
        format!(
            "总作业数: {}",
            profile["homework"]["total"].as_i64().unwrap_or(0)
        ),
        format!(
            "已完成: {}",
            profile["homework"]["completed"].as_i64().unwrap_or(0)
        ),
        format!(
            "完成率: {:.1}%",
            profile["homework"]["rate"].as_f64().unwrap_or(0.0) * 100.0
        ),
        format!(
            "连续未交: {}",
            profile["homework"]["consecutive_incomplete"]
                .as_i64()
                .unwrap_or(0)
        ),
        "".to_string(),
        "考勤表现".to_string(),
        format!(
            "总记录: {}",
            profile["attendance"]["total"].as_i64().unwrap_or(0)
        ),
        format!(
            "出勤率: {:.1}%",
            profile["attendance"]["rate"].as_f64().unwrap_or(0.0) * 100.0
        ),
        format!(
            "异常次数: {}",
            profile["attendance"]["abnormal"].as_i64().unwrap_or(0)
        ),
        "".to_string(),
        "重点关注原因".to_string(),
    ];
    if let Some(items) = profile["focus_reasons"].as_array() {
        if items.is_empty() {
            lines.push("无".to_string());
        } else {
            for item in items {
                lines.push(format!("- {}", item.as_str().unwrap_or("")));
            }
        }
    }
    lines.push("".to_string());
    lines.push("综合评价".to_string());
    lines.push(
        profile["overall_evaluation"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    );
    lines.push("".to_string());
    lines.push("最近成绩".to_string());
    if let Some(items) = profile["score_trend"].as_array() {
        for item in items.iter().take(12) {
            lines.push(format!(
                "{} {} {}",
                item["exam_point"].as_str().unwrap_or(""),
                item["subject_name"].as_str().unwrap_or(""),
                item["score_value"]
                    .as_f64()
                    .map(|v| format!("{:.1}", v))
                    .unwrap_or_else(|| "-".to_string())
            ));
        }
    }
    create_pdf_from_plain_text(&file_path, &lines.join("\n"))
}
