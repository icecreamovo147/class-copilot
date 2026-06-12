use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Cohort {
    pub id: i64,
    pub cohort_name: String,
    pub class_name: String,
    pub grade_name: Option<String>,
    pub school_name: Option<String>,
    pub head_teacher: Option<String>,
    pub admission_year: Option<i64>,
    pub graduation_year: Option<i64>,
    pub semester: Option<String>,
    pub status: String,
    pub is_current: bool,
    pub archive_time: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_cohorts(
    state: State<'_, AppState>,
    search: Option<String>,
    status: Option<String>,
) -> Result<Vec<Cohort>, String> {
    let pool = &state.db;
    let mut query = String::from("SELECT * FROM cohort WHERE 1=1");
    let mut idx = 1;
    if search.is_some() {
        query.push_str(&format!(
            " AND (cohort_name LIKE ?{} OR class_name LIKE ?{})",
            idx, idx
        ));
        idx += 1;
    }
    if status.is_some() {
        query.push_str(&format!(" AND status = ?{}", idx));
    }
    query.push_str(" ORDER BY id DESC");
    let mut q = sqlx::query_as::<_, Cohort>(&query);
    if let Some(s) = search {
        q = q.bind(format!("%{}%", s));
    }
    if let Some(st) = status {
        q = q.bind(st);
    }
    q.fetch_all(pool)
        .await
        .map_err(|e| format!("查询届次失败: {}", e))
}

#[tauri::command]
pub async fn get_cohort(state: State<'_, AppState>, id: i64) -> Result<Cohort, String> {
    sqlx::query_as::<_, Cohort>("SELECT * FROM cohort WHERE id = ?1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| format!("获取届次失败: {}", e))
}

#[tauri::command]
pub async fn get_current_cohort(state: State<'_, AppState>) -> Result<Option<Cohort>, String> {
    sqlx::query_as::<_, Cohort>(
        "SELECT * FROM cohort WHERE is_current = 1 AND status = '使用中' ORDER BY id ASC LIMIT 1",
    )
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("获取当前届次失败: {}", e))
}

#[tauri::command]
pub async fn create_cohort(
    state: State<'_, AppState>,
    cohort_name: String,
    class_name: String,
    grade_name: Option<String>,
    school_name: Option<String>,
    head_teacher: Option<String>,
    admission_year: Option<i64>,
    graduation_year: Option<i64>,
    semester: Option<String>,
    remark: Option<String>,
) -> Result<Cohort, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let is_current = count.0 == 0;
    sqlx::query_as::<_, Cohort>(
        "INSERT INTO cohort (cohort_name, class_name, grade_name, school_name, head_teacher, admission_year, graduation_year, semester, status, is_current, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '使用中', ?9, ?10, ?11, ?11) RETURNING *")
    .bind(&cohort_name).bind(&class_name).bind(&grade_name).bind(&school_name)
    .bind(&head_teacher).bind(admission_year).bind(graduation_year).bind(&semester)
    .bind(is_current).bind(&remark).bind(&now)
    .fetch_one(pool).await.map_err(|e| format!("创建届次失败: {}", e))
}

#[tauri::command]
pub async fn update_cohort(
    state: State<'_, AppState>,
    id: i64,
    cohort_name: Option<String>,
    class_name: Option<String>,
    grade_name: Option<String>,
    school_name: Option<String>,
    head_teacher: Option<String>,
    admission_year: Option<i64>,
    graduation_year: Option<i64>,
    semester: Option<String>,
    remark: Option<String>,
) -> Result<Cohort, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let cohort = get_cohort_internal(pool, id).await?;
    if cohort.status == "已归档" {
        return Err("已归档届次不可编辑".to_string());
    }
    sqlx::query("UPDATE cohort SET cohort_name = COALESCE(?1, cohort_name), class_name = COALESCE(?2, class_name), grade_name = COALESCE(?3, grade_name), school_name = COALESCE(?4, school_name), head_teacher = COALESCE(?5, head_teacher), admission_year = COALESCE(?6, admission_year), graduation_year = COALESCE(?7, graduation_year), semester = COALESCE(?8, semester), remark = COALESCE(?9, remark), updated_at = ?10 WHERE id = ?11")
    .bind(&cohort_name).bind(&class_name).bind(&grade_name).bind(&school_name)
    .bind(&head_teacher).bind(admission_year).bind(graduation_year).bind(&semester).bind(&remark).bind(&now).bind(id)
    .execute(pool).await.map_err(|e| format!("更新届次失败: {}", e))?;
    get_cohort_internal(pool, id).await
}

#[tauri::command]
pub async fn archive_cohort(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let cohort = get_cohort_internal(pool, id).await?;
    if cohort.status == "已归档" {
        return Err("届次已归档".to_string());
    }
    if cohort.is_current {
        if let Ok(Some((other_id,))) = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM cohort WHERE id != ?1 AND status = '使用中' ORDER BY id ASC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        {
            sqlx::query("UPDATE cohort SET is_current = 1 WHERE id = ?1")
                .bind(other_id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
        }
        sqlx::query("UPDATE cohort SET is_current = 0 WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    sqlx::query(
        "UPDATE cohort SET status = '已归档', archive_time = ?1, updated_at = ?1 WHERE id = ?2",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("归档届次失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn unarchive_cohort(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let cohort = get_cohort_internal(pool, id).await?;
    if cohort.status == "使用中" {
        return Err("届次未归档".to_string());
    }
    sqlx::query(
        "UPDATE cohort SET status = '使用中', archive_time = NULL, updated_at = ?1 WHERE id = ?2",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("解除归档失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn set_current_cohort(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let cohort = get_cohort_internal(pool, id).await?;
    if cohort.status == "已归档" {
        return Err("已归档届次不能设为当前".to_string());
    }
    sqlx::query("UPDATE cohort SET is_current = 0")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("UPDATE cohort SET is_current = 1 WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("设置当前届次失败: {}", e))?;
    Ok(())
}

pub async fn get_cohort_internal(pool: &SqlitePool, id: i64) -> Result<Cohort, String> {
    sqlx::query_as::<_, Cohort>("SELECT * FROM cohort WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取届次失败: {}", e))
}

pub async fn check_cohort_readonly(pool: &SqlitePool, cohort_id: i64) -> Result<(), String> {
    let cohort = sqlx::query_as::<_, Cohort>("SELECT * FROM cohort WHERE id = ?1")
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .map_err(|_| "届次不存在".to_string())?;
    if cohort.status == "已归档" {
        return Err("已归档届次不允许修改数据".to_string());
    }
    Ok(())
}
