use calamine::Reader;
use chrono::Local;
use rust_xlsxwriter::{Workbook, Worksheet};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use super::cohort::check_cohort_readonly;
use super::student::ensure_student_belongs_to_cohort;
use crate::AppState;

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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExamSubjectConfig {
    pub id: i64,
    pub exam_id: i64,
    pub subject_id: i64,
    pub full_score: f64,
    pub pass_score: f64,
    pub excellent_score: f64,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExamSubjectConfigInput {
    pub subject_id: i64,
    pub full_score: f64,
    pub pass_score: f64,
    pub excellent_score: f64,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ScoreImportPreviewRow {
    row: usize,
    student_no: String,
    student_name: Option<String>,
    score_value: Option<f64>,
    valid: bool,
    warning: Option<String>,
}

struct ScoreConfigRule {
    full_score: f64,
    pass_score: f64,
    excellent_score: f64,
}

struct ScoreImportValidation {
    valid_rows: Vec<(i64, f64)>,
    preview_rows: Vec<ScoreImportPreviewRow>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn validate_score_rule(
    full_score: f64,
    pass_score: f64,
    excellent_score: f64,
) -> Result<(), String> {
    if full_score <= 0.0 {
        return Err("满分必须大于 0".to_string());
    }
    if pass_score < 0.0 || pass_score > full_score {
        return Err("及格线必须在 0 到满分之间".to_string());
    }
    if excellent_score < pass_score || excellent_score > full_score {
        return Err("优秀线必须大于等于及格线且不超过满分".to_string());
    }
    Ok(())
}

async fn get_exam_subject_rule(
    pool: &sqlx::SqlitePool,
    exam_id: i64,
    subject_id: i64,
) -> Result<ScoreConfigRule, String> {
    let row = sqlx::query_as::<_, (f64, f64, f64)>(
        "SELECT full_score, pass_score, excellent_score
         FROM exam_subject_config
         WHERE exam_id = ?1 AND subject_id = ?2",
    )
    .bind(exam_id)
    .bind(subject_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    row.map(
        |(full_score, pass_score, excellent_score)| ScoreConfigRule {
            full_score,
            pass_score,
            excellent_score,
        },
    )
    .ok_or_else(|| "请先为该考试配置科目、满分和统计规则".to_string())
}

async fn validate_score_import_file(
    pool: &sqlx::SqlitePool,
    exam_id: i64,
    subject_id: i64,
    cohort_id: i64,
    file_path: &str,
) -> Result<ScoreImportValidation, String> {
    let rule = get_exam_subject_rule(pool, exam_id, subject_id).await?;
    let mut workbook: calamine::Xlsx<_> =
        calamine::open_workbook(file_path).map_err(|e| format!("无法打开 Excel 文件: {}", e))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or("Excel 文件没有工作表".to_string())?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    let mut valid_rows = Vec::new();
    let mut preview_rows = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut file_score_keys = std::collections::HashSet::new();

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 {
            continue;
        }
        if row.len() < 3 {
            continue;
        }

        let row_number = row_idx + 1;
        let student_no = row[0].to_string().trim().to_string();
        let student_name = row[1].to_string().trim().to_string();
        let score_str = row[2].to_string().trim().to_string();

        if student_no.is_empty() && score_str.is_empty() {
            continue;
        }

        let mut preview_row = ScoreImportPreviewRow {
            row: row_number,
            student_no: student_no.clone(),
            student_name: if student_name.is_empty() {
                None
            } else {
                Some(student_name.clone())
            },
            score_value: None,
            valid: false,
            warning: None,
        };

        if student_no.is_empty() {
            errors.push(format!("第{}行: 学号为空", row_number));
            preview_rows.push(preview_row);
            continue;
        }

        let student = sqlx::query_as::<_, (i64, String)>(
            "SELECT id, name FROM student WHERE student_no = ?1 AND cohort_id = ?2 AND deleted_at IS NULL",
        )
        .bind(&student_no)
        .bind(cohort_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        let (student_id, actual_name) = match student {
            Some(value) => value,
            None => {
                errors.push(format!(
                    "第{}行: 学号 '{}' 不属于当前届次或不存在",
                    row_number, student_no
                ));
                preview_rows.push(preview_row);
                continue;
            }
        };
        preview_row.student_name = Some(actual_name);

        if score_str.is_empty() {
            errors.push(format!(
                "第{}行: 学号 '{}' 的分数为空",
                row_number, student_no
            ));
            preview_rows.push(preview_row);
            continue;
        }

        let score_value = match score_str.parse::<f64>() {
            Ok(v) if v >= 0.0 && v <= rule.full_score => v,
            Ok(v) => {
                errors.push(format!(
                    "第{}行: 学号 '{}' 的分数 {:.1} 超出有效范围 (0-{:.1})",
                    row_number, student_no, v, rule.full_score
                ));
                preview_rows.push(preview_row);
                continue;
            }
            Err(_) => {
                errors.push(format!(
                    "第{}行: 学号 '{}' 的分数 '{}' 不是有效数字",
                    row_number, student_no, score_str
                ));
                preview_rows.push(preview_row);
                continue;
            }
        };
        preview_row.score_value = Some(score_value);

        let file_key = format!("{}|{}|{}", student_no, exam_id, subject_id);
        if !file_score_keys.insert(file_key) {
            errors.push(format!(
                "第{}行: 学号 '{}' 在文件中重复出现",
                row_number, student_no
            ));
            preview_rows.push(preview_row);
            continue;
        }

        let existing: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM score WHERE exam_id = ?1 AND subject_id = ?2 AND student_id = ?3",
        )
        .bind(exam_id)
        .bind(subject_id)
        .bind(student_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

        if existing.0 > 0 {
            let warning = format!(
                "第{}行: 学号 '{}' 已有成绩，导入后将覆盖",
                row_number, student_no
            );
            preview_row.warning = Some(warning.clone());
            warnings.push(warning);
        }

        preview_row.valid = true;
        valid_rows.push((student_id, score_value));
        preview_rows.push(preview_row);
    }

    Ok(ScoreImportValidation {
        valid_rows,
        preview_rows,
        errors,
        warnings,
    })
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
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) RETURNING *",
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
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
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
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取考试失败: {}", e))
}

#[tauri::command]
pub async fn delete_exam(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, exam.cohort_id).await?;

    sqlx::query("UPDATE exam SET deleted_at = ?1 WHERE id = ?2")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除考试失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn get_exam_subject_configs(
    state: State<'_, AppState>,
    exam_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "SELECT esc.id, esc.exam_id, esc.subject_id, esc.full_score, esc.pass_score, esc.excellent_score,
                esc.sort_order, esc.created_at, esc.updated_at, sub.name as subject_name, sub.is_active
         FROM exam_subject_config esc
         JOIN subject sub ON sub.id = esc.subject_id
         WHERE esc.exam_id = ?1
         ORDER BY esc.sort_order ASC, sub.sort_order ASC, sub.id ASC",
    )
    .bind(exam_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("获取考试科目配置失败: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<i64, _>("id"),
                "exam_id": row.get::<i64, _>("exam_id"),
                "subject_id": row.get::<i64, _>("subject_id"),
                "subject_name": row.get::<String, _>("subject_name"),
                "full_score": row.get::<f64, _>("full_score"),
                "pass_score": row.get::<f64, _>("pass_score"),
                "excellent_score": row.get::<f64, _>("excellent_score"),
                "sort_order": row.get::<i64, _>("sort_order"),
                "is_active": row.get::<i64, _>("is_active") == 1,
                "created_at": row.get::<String, _>("created_at"),
                "updated_at": row.get::<String, _>("updated_at"),
            })
        })
        .collect())
}

#[tauri::command]
pub async fn save_exam_subject_configs(
    state: State<'_, AppState>,
    exam_id: i64,
    configs: Vec<ExamSubjectConfigInput>,
) -> Result<(), String> {
    let pool = &state.db;
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(exam_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, exam.cohort_id).await?;

    for config in &configs {
        validate_score_rule(config.full_score, config.pass_score, config.excellent_score)?;
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM exam_subject_config WHERE exam_id = ?1")
        .bind(exam_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("清理旧配置失败: {}", e))?;

    for config in &configs {
        sqlx::query(
            "INSERT INTO exam_subject_config (exam_id, subject_id, full_score, pass_score, excellent_score, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        )
        .bind(exam_id)
        .bind(config.subject_id)
        .bind(config.full_score)
        .bind(config.pass_score)
        .bind(config.excellent_score)
        .bind(config.sort_order.unwrap_or(0))
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("保存考试科目配置失败: {}", e))?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
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

    Ok(rows
        .iter()
        .map(|r| {
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
        })
        .collect())
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
    let rule = get_exam_subject_rule(pool, exam_id, subject_id).await?;

    // 检查考试所属届次是否只读
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(exam_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let cohort_id = exam.cohort_id;
    check_cohort_readonly(pool, cohort_id).await?;

    // 验证所有学生属于考试所属届次
    for score_val in &scores {
        let student_id = score_val["student_id"].as_i64().ok_or("缺少 student_id")?;
        ensure_student_belongs_to_cohort(pool, student_id, cohort_id)
            .await
            .map_err(|_| format!("学生 ID {} 不属于考试所属届次", student_id))?;
        if let Some(score_value) = score_val["score_value"].as_f64() {
            if score_value < 0.0 || score_value > rule.full_score {
                return Err(format!(
                    "学生 ID {} 的成绩超出有效范围 (0-{:.1})",
                    student_id, rule.full_score
                ));
            }
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
async fn recompute_rankings(
    pool: &sqlx::SqlitePool,
    exam_id: i64,
    subject_id: i64,
) -> Result<(), String> {
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
         WHERE exam_id = ?1 AND subject_id = ?2",
    )
    .bind(exam_id)
    .bind(subject_id)
    .execute(pool)
    .await
    .map_err(|e| format!("计算排名失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn preview_scores_excel(
    state: State<'_, AppState>,
    exam_id: i64,
    subject_id: i64,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let exam = sqlx::query_as::<_, Exam>("SELECT * FROM exam WHERE id = ?1")
        .bind(exam_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let validation =
        validate_score_import_file(pool, exam_id, subject_id, exam.cohort_id, &file_path).await?;

    Ok(serde_json::json!({
        "total_rows": validation.preview_rows.len(),
        "valid_rows": validation.valid_rows.len(),
        "error_rows": validation.errors.len(),
        "rows": validation.preview_rows,
        "errors": validation.errors,
        "warnings": validation.warnings,
    }))
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
        .bind(exam_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    check_cohort_readonly(pool, exam.cohort_id).await?;
    let validation =
        validate_score_import_file(pool, exam_id, subject_id, exam.cohort_id, &file_path).await?;

    if !validation.errors.is_empty() {
        return Ok(serde_json::json!({
            "success": 0i64,
            "errors": validation.errors,
            "warnings": validation.warnings
        }));
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 阶段3: 事务批量写入
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for (student_id, score_value) in &validation.valid_rows {
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

    let success_count = validation.valid_rows.len() as i64;
    let empty_errors: Vec<String> = Vec::new();
    Ok(serde_json::json!({
        "success": success_count,
        "errors": empty_errors,
        "warnings": validation.warnings
    }))
}

#[tauri::command]
pub async fn score_statistics(
    state: State<'_, AppState>,
    exam_id: i64,
    subject_id: i64,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    let rule = get_exam_subject_rule(pool, exam_id, subject_id).await?;
    let result = sqlx::query_as::<_, (Option<f64>, Option<f64>, Option<f64>, f64, f64)>(
        "SELECT AVG(score_value), MAX(score_value), MIN(score_value),
            CAST(SUM(CASE WHEN score_value >= ?3 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as pass_rate,
            CAST(SUM(CASE WHEN score_value >= ?4 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as excellent_rate
         FROM score WHERE exam_id = ?1 AND subject_id = ?2 AND score_value IS NOT NULL"
    )
    .bind(exam_id)
    .bind(subject_id)
    .bind(rule.pass_score)
    .bind(rule.excellent_score)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "avg_score": result.0.unwrap_or(0.0),
        "max_score": result.1.unwrap_or(0.0),
        "min_score": result.2.unwrap_or(0.0),
        "pass_rate": result.3,
        "excellent_rate": result.4,
        "full_score": rule.full_score,
        "pass_score": rule.pass_score,
        "excellent_score": rule.excellent_score
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
         ORDER BY total_score DESC",
    )
    .bind(exam_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(rows.len());
    let mut last_score: Option<f64> = None;
    let mut current_rank = 0i64;

    for (idx, (id, name, no, score)) in rows.into_iter().enumerate() {
        if last_score != Some(score) {
            current_rank = (idx + 1) as i64;
            last_score = Some(score);
        }
        result.push(serde_json::json!({
            "student_id": id,
            "student_name": name,
            "student_no": no,
            "total_score": score,
            "rank_no": current_rank
        }));
    }

    Ok(result)
}

#[tauri::command]
pub async fn export_scores_excel(
    state: State<'_, AppState>,
    exam_id: i64,
    subject_id: i64,
    file_path: String,
) -> Result<(), String> {
    let pool = &state.db;
    let rows = sqlx::query(
        "SELECT s.name as student_name, s.student_no, sc.score_value, sc.rank_no,
                e.name as exam_name, sub.name as subject_name
         FROM score sc
         JOIN student s ON sc.student_id = s.id
         JOIN exam e ON sc.exam_id = e.id
         JOIN subject sub ON sc.subject_id = sub.id
         WHERE sc.exam_id = ?1 AND sc.subject_id = ?2 AND s.deleted_at IS NULL
         ORDER BY s.student_no ASC",
    )
    .bind(exam_id)
    .bind(subject_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询成绩失败: {}", e))?;

    let mut workbook = Workbook::new();
    let mut sheet = Worksheet::new();
    let headers = ["考试", "科目", "学号", "姓名", "成绩", "排名"];
    for (idx, header) in headers.iter().enumerate() {
        sheet
            .write_string(0, idx as u16, *header)
            .map_err(|e| e.to_string())?;
    }

    for (row_idx, row) in rows.iter().enumerate() {
        let line = (row_idx + 1) as u32;
        sheet
            .write_string(line, 0, &row.get::<String, _>("exam_name"))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(line, 1, &row.get::<String, _>("subject_name"))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(line, 2, &row.get::<String, _>("student_no"))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(line, 3, &row.get::<String, _>("student_name"))
            .map_err(|e| e.to_string())?;
        if let Some(score) = row.get::<Option<f64>, _>("score_value") {
            sheet
                .write_number(line, 4, score)
                .map_err(|e| e.to_string())?;
        }
        if let Some(rank_no) = row.get::<Option<i64>, _>("rank_no") {
            sheet
                .write_number(line, 5, rank_no as f64)
                .map_err(|e| e.to_string())?;
        }
    }

    workbook.push_worksheet(sheet);
    workbook
        .save(&file_path)
        .map_err(|e| format!("导出成绩失败: {}", e))?;
    Ok(())
}
