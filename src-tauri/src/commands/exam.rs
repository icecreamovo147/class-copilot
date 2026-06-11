use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::State;
use sqlx::Row;
use calamine::Reader;

use crate::AppState;
use super::cohort::check_cohort_readonly;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Exam {
    pub id: i64,
    pub cohort_id: i64,
    pub name: String,
    pub exam_type: Option<String>,
    pub exam_date: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Score {
    pub id: i64,
    pub exam_id: i64,
    pub subject_id: i64,
    pub student_id: i64,
    pub score_value: Option<f64>,
    pub rank_no: Option<i64>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_exams(state: State<'_, AppState>, cohort_id: i64) -> Result<Vec<Exam>, String> {
    sqlx::query_as::<_, Exam>(
        "SELECT * FROM exam WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY exam_date DESC, id DESC"
    )
    .bind(cohort_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("获取考试列表失败: {}", e))
}

#[tauri::command]
pub async fn create_exam(
    state: State<'_, AppState>,
    cohort_id: i64,
    name: String,
    exam_type: Option<String>,
    exam_date: Option<String>,
    remark: Option<String>,
) -> Result<Exam, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query_as::<_, Exam>(
        "INSERT INTO exam (cohort_id, name, exam_type, exam_date, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) RETURNING *"
    )
    .bind(cohort_id)
    .bind(&name)
    .bind(&exam_type)
    .bind(&exam_date)
    .bind(&remark)
    .bind(&now)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("创建考试失败: {}", e))
}

#[tauri::command]
pub async fn update_exam(
    state: State<'_, AppState>,
    id: i64,
    name: Option<String>,
    exam_type: Option<String>,
    exam_date: Option<String>,
    remark: Option<String>,
) -> Result<Exam, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, exam.cohort_id).await?;

    sqlx::query(
        "UPDATE exam SET name = COALESCE(?1, name), exam_type = COALESCE(?2, exam_type),
         exam_date = COALESCE(?3, exam_date), remark = COALESCE(?4, remark), updated_at = ?5 WHERE id = ?6"
    )
    .bind(&name)
    .bind(&exam_type)
    .bind(&exam_date)
    .bind(&remark)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("更新考试失败: {}", e))?;

    sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(id).fetch_one(pool).await
        .map_err(|e| format!("获取考试失败: {}", e))
}

#[tauri::command]
pub async fn delete_exam(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, exam.cohort_id).await?;

    sqlx::query("UPDATE exam SET deleted_at = ?1 WHERE id = ?2")
        .bind(&now).bind(id).execute(pool).await
        .map_err(|e| format!("删除考试失败: {}", e))?;
    Ok(())
}

// 成绩相关
#[tauri::command]
pub async fn get_scores_by_exam(
    state: State<'_, AppState>,
    exam_id: i64,
    subject_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let pool = &state.db;
    let rows = sqlx::query(
        "SELECT sc.id, sc.exam_id, sc.subject_id, sc.student_id, sc.score_value, sc.rank_no, sc.remark, sc.created_at, sc.updated_at,
                s.name as student_name, s.student_no, sub.name as subject_name
         FROM score sc
         JOIN student s ON sc.student_id = s.id
         JOIN subject sub ON sc.subject_id = sub.id
         WHERE sc.exam_id = ?1 AND sc.subject_id = ?2 AND s.deleted_at IS NULL
         ORDER BY s.student_no ASC"
    )
    .bind(exam_id)
    .bind(subject_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<i64, _>("id"),
            "exam_id": r.get::<i64, _>("exam_id"),
            "subject_id": r.get::<i64, _>("subject_id"),
            "student_id": r.get::<i64, _>("student_id"),
            "score_value": r.get::<Option<f64>, _>("score_value"),
            "rank_no": r.get::<Option<i64>, _>("rank_no"),
            "remark": r.get::<Option<String>, _>("remark"),
            "created_at": r.get::<String, _>("created_at"),
            "updated_at": r.get::<String, _>("updated_at"),
            "student_name": r.get::<String, _>("student_name"),
            "student_no": r.get::<String, _>("student_no"),
            "subject_name": r.get::<String, _>("subject_name")
        })
    }).collect())
}

#[tauri::command]
pub async fn save_scores(
    state: State<'_, AppState>,
    exam_id: i64,
    subject_id: i64,
    scores: Vec<serde_json::Value>,
) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 检查考试所属届次是否只读
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(exam_id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    let cohort_id = exam.cohort_id;
    check_cohort_readonly(pool, cohort_id).await?;

    // 验证所有学生属于考试所属届次
    for score_val in &scores {
        let student_id = score_val["student_id"].as_i64().ok_or("缺少 student_id")?;
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
                "学生 ID {} 不属于考试所属届次 (学生届次: {}, 考试届次: {})",
                student_id, belongs.0, cohort_id
            ));
        }
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for score_val in &scores {
        let student_id = score_val["student_id"].as_i64().ok_or("缺少 student_id")?;
        let score_value = score_val["score_value"].as_f64();

        sqlx::query(
            "INSERT INTO score (exam_id, subject_id, student_id, score_value, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(exam_id, subject_id, student_id) DO UPDATE SET
             score_value = excluded.score_value, updated_at = excluded.updated_at"
        )
        .bind(exam_id)
        .bind(subject_id)
        .bind(student_id)
        .bind(score_value)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    // 保存完成后重新计算该考试+科目的排名并写入 rank_no
    recompute_rankings(pool, exam_id, subject_id).await?;

    Ok(())
}

/// 根据 score_value 降序计算指定考试+科目的排名并写入 rank_no
async fn recompute_rankings(pool: &sqlx::SqlitePool, exam_id: i64, subject_id: i64) -> Result<(), String> {
    sqlx::query(
        "UPDATE score SET rank_no = CASE
            WHEN score_value IS NULL THEN NULL
            ELSE (
                SELECT COUNT(*) + 1 FROM score s2
                WHERE s2.exam_id = ?1 AND s2.subject_id = ?2
                  AND s2.score_value IS NOT NULL
                  AND s2.score_value > score.score_value
            )
         END
         WHERE exam_id = ?1 AND subject_id = ?2"
    )
    .bind(exam_id)
    .bind(subject_id)
    .execute(pool)
    .await
    .map_err(|e| format!("计算排名失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn import_scores_excel(
    state: State<'_, AppState>,
    exam_id: i64,
    subject_id: i64,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(exam_id).fetch_one(pool).await.map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, exam.cohort_id).await?;

    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(&file_path)
        .map_err(|e| format!("无法打开 Excel 文件: {}", e))?;
    let sheet_name = workbook.sheet_names().first().cloned()
        .ok_or("Excel 文件没有工作表".to_string())?;
    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 阶段1: 预校验所有行
    let mut parsed_rows: Vec<(i64, f64)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    // 检测文件内同一学生+同一科目重复
    let mut file_score_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; }
        if row.len() < 3 { continue; }

        let student_no = row[0].to_string().trim().to_string();
        let score_str = row[2].to_string().trim().to_string();

        if student_no.is_empty() { continue; }

        // 查找学生（含归属校验）
        let student = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM student WHERE student_no = ?1 AND cohort_id = ?2 AND deleted_at IS NULL"
        )
        .bind(&student_no)
        .bind(exam.cohort_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        let student_id = match student {
            Some((id,)) => id,
            None => {
                errors.push(format!("第{}行: 学号 '{}' 不属于当前届次或不存在", row_idx + 1, student_no));
                continue;
            }
        };

        // 分数格式校验
        if score_str.is_empty() {
            errors.push(format!("第{}行: 学号 '{}' 的分数为空", row_idx + 1, student_no));
            continue;
        }

        let score_value: f64 = match score_str.parse::<f64>() {
            Ok(v) if v >= 0.0 && v <= 750.0 => v,
            Ok(v) => {
                errors.push(format!(
                    "第{}行: 学号 '{}' 的分数 {:.1} 超出有效范围 (0-750)",
                    row_idx + 1, student_no, v
                ));
                continue;
            }
            Err(_) => {
                errors.push(format!(
                    "第{}行: 学号 '{}' 的分数 '{}' 不是有效数字",
                    row_idx + 1, student_no, score_str
                ));
                continue;
            }
        };

        // 检测文件内同一学生的重复成绩（会被最后一行静默覆盖）
        let file_key = format!("{}|{}|{}", student_no, exam_id, subject_id);
        if !file_score_keys.insert(file_key) {
            errors.push(format!(
                "第{}行: 学号 '{}' 在文件中重复出现（同考试同科目），会覆盖前面的成绩",
                row_idx + 1, student_no
            ));
            continue;
        }

        // 检查是否有已有成绩（将被覆盖）
        let existing: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM score WHERE exam_id = ?1 AND subject_id = ?2 AND student_id = ?3"
        )
        .bind(exam_id).bind(subject_id).bind(student_id)
        .fetch_one(pool).await.map_err(|e| e.to_string())?;
        if existing.0 > 0 {
            warnings.push(format!("第{}行: 学号 '{}' 已有成绩，将被覆盖", row_idx + 1, student_no));
        }

        parsed_rows.push((student_id, score_value));
    }

    // 阶段2: 有校验错误则不写入
    if !errors.is_empty() {
        let empty_warnings: Vec<String> = Vec::new();
        return Ok(serde_json::json!({
            "success": 0i64,
            "errors": errors,
            "warnings": empty_warnings
        }));
    }

    // 阶段3: 事务批量写入
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for (student_id, score_value) in &parsed_rows {
        sqlx::query(
            "INSERT INTO score (exam_id, subject_id, student_id, score_value, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(exam_id, subject_id, student_id) DO UPDATE SET
             score_value = excluded.score_value, updated_at = excluded.updated_at"
        )
        .bind(exam_id)
        .bind(subject_id)
        .bind(student_id)
        .bind(score_value)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    // 导入完成后重新计算排名
    recompute_rankings(pool, exam_id, subject_id).await?;

    let success_count = parsed_rows.len() as i64;
    let empty_errors: Vec<String> = Vec::new();
    Ok(serde_json::json!({
        "success": success_count,
        "errors": empty_errors,
        "warnings": warnings
    }))
}

#[tauri::command]
pub async fn score_statistics(
    state: State<'_, AppState>,
    exam_id: i64,
    subject_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let result = sqlx::query_as::<_, (Option<f64>, Option<f64>, Option<f64>, f64, f64)>(
        "SELECT AVG(score_value), MAX(score_value), MIN(score_value),
            CAST(SUM(CASE WHEN score_value >= 60 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as pass_rate,
            CAST(SUM(CASE WHEN score_value >= 90 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as excellent_rate
         FROM score WHERE exam_id = ?1 AND subject_id = ?2 AND score_value IS NOT NULL"
    )
    .bind(exam_id)
    .bind(subject_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "avg_score": result.0.unwrap_or(0.0),
        "max_score": result.1.unwrap_or(0.0),
        "min_score": result.2.unwrap_or(0.0),
        "pass_rate": result.3,
        "excellent_rate": result.4
    }))
}

#[tauri::command]
pub async fn score_rankings(
    state: State<'_, AppState>,
    exam_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let pool = &state.db;
    // 计算总分排名
    let rows = sqlx::query_as::<_, (i64, String, String, f64)>(
        "SELECT s.id, s.name, s.student_no,
            COALESCE(SUM(sc.score_value), 0) as total_score
         FROM student s
         JOIN score sc ON sc.student_id = s.id
         WHERE sc.exam_id = ?1 AND s.deleted_at IS NULL
         GROUP BY s.id
         ORDER BY total_score DESC"
    )
    .bind(exam_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let result: Vec<serde_json::Value> = rows.into_iter().enumerate().map(|(idx, (id, name, no, score))| {
        serde_json::json!({
            "student_id": id,
            "student_name": name,
            "student_no": no,
            "total_score": score,
            "rank_no": (idx + 1) as i64
        })
    }).collect();

    Ok(result)
}
