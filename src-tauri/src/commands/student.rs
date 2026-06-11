use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

use crate::AppState;
use super::cohort::check_cohort_readonly;
use calamine::Reader;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Student {
    pub id: i64,
    pub cohort_id: i64,
    pub name: String,
    pub student_no: String,
    pub gender: Option<String>,
    pub phone: Option<String>,
    pub parent_name: Option<String>,
    pub parent_phone: Option<String>,
    pub address: Option<String>,
    pub group_name: Option<String>,
    pub status: String,
    pub is_focus: bool,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResult<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[tauri::command]
pub async fn get_students(
    state: State<'_, AppState>,
    cohort_id: i64,
    search: Option<String>,
    gender: Option<String>,
    group_name: Option<String>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<PaginatedResult<Student>, String> {
    let pool = &state.db;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    let mut where_clauses = vec!["cohort_id = ?1".to_string(), "deleted_at IS NULL".to_string()];
    let mut params: Vec<String> = vec![cohort_id.to_string()];
    let mut param_idx = 2;

    if let Some(ref s) = search {
        where_clauses.push(format!("(name LIKE ?{} OR student_no LIKE ?{})", param_idx, param_idx));
        params.push(format!("%{}%", s));
        param_idx += 1;
    }
    if let Some(ref g) = gender {
        where_clauses.push(format!("gender = ?{}", param_idx));
        params.push(g.clone());
        param_idx += 1;
    }
    if let Some(ref grp) = group_name {
        where_clauses.push(format!("group_name = ?{}", param_idx));
        params.push(grp.clone());
        param_idx += 1;
    }
    if let Some(ref st) = status {
        where_clauses.push(format!("status = ?{}", param_idx));
        params.push(st.clone());
        param_idx += 1;
    }

    let where_clause = where_clauses.join(" AND ");

    // 查询总数
    let count_query = format!("SELECT COUNT(*) FROM student WHERE {}", where_clause);
    let mut count_stmt = sqlx::query_as::<_, (i64,)>(&count_query);
    for p in &params {
        count_stmt = count_stmt.bind(p);
    }
    let (total,): (i64,) = count_stmt.fetch_one(pool).await.map_err(|e| e.to_string())?;

    // 查询数据
    let data_query = format!(
        "SELECT * FROM student WHERE {} ORDER BY student_no ASC LIMIT ?{} OFFSET ?{}",
        where_clause, param_idx, param_idx + 1
    );
    let mut data_stmt = sqlx::query_as::<_, Student>(&data_query);
    for p in &params {
        data_stmt = data_stmt.bind(p);
    }
    data_stmt = data_stmt.bind(page_size).bind(offset);
    let data = data_stmt.fetch_all(pool).await.map_err(|e| e.to_string())?;

    Ok(PaginatedResult { data, total, page, page_size })
}

#[tauri::command]
pub async fn get_all_students(
    state: State<'_, AppState>,
    cohort_id: i64,
) -> Result<Vec<Student>, String> {
    sqlx::query_as::<_, Student>(
        "SELECT * FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY student_no ASC"
    )
    .bind(cohort_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("查询学生失败: {}", e))
}

#[tauri::command]
pub async fn get_student(state: State<'_, AppState>, id: i64) -> Result<Student, String> {
    sqlx::query_as::<_, Student>("SELECT * FROM student WHERE id = ?1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| format!("获取学生失败: {}", e))
}

#[tauri::command]
pub async fn create_student(
    state: State<'_, AppState>,
    cohort_id: i64,
    name: String,
    student_no: String,
    gender: Option<String>,
    phone: Option<String>,
    parent_name: Option<String>,
    parent_phone: Option<String>,
    address: Option<String>,
    group_name: Option<String>,
    status: Option<String>,
    is_focus: Option<bool>,
    remark: Option<String>,
) -> Result<Student, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let status = status.unwrap_or_else(|| "正常".to_string());
    let is_focus = is_focus.unwrap_or(false);

    sqlx::query_as::<_, Student>(
        "INSERT INTO student (cohort_id, name, student_no, gender, phone, parent_name, parent_phone, address, group_name, status, is_focus, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
         RETURNING *"
    )
    .bind(cohort_id)
    .bind(&name)
    .bind(&student_no)
    .bind(&gender)
    .bind(&phone)
    .bind(&parent_name)
    .bind(&parent_phone)
    .bind(&address)
    .bind(&group_name)
    .bind(&status)
    .bind(is_focus)
    .bind(&remark)
    .bind(&now)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "学号在当前届次中已存在".to_string()
        } else {
            format!("创建学生失败: {}", e)
        }
    })
}

#[tauri::command]
pub async fn update_student(
    state: State<'_, AppState>,
    id: i64,
    name: Option<String>,
    student_no: Option<String>,
    gender: Option<String>,
    phone: Option<String>,
    parent_name: Option<String>,
    parent_phone: Option<String>,
    address: Option<String>,
    group_name: Option<String>,
    status: Option<String>,
    is_focus: Option<bool>,
    remark: Option<String>,
) -> Result<Student, String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 先获取学生信息以检查届次
    let student = get_student_internal(pool, id).await?;
    check_cohort_readonly(pool, student.cohort_id).await?;

    sqlx::query(
        "UPDATE student SET name = COALESCE(?1, name), student_no = COALESCE(?2, student_no),
         gender = COALESCE(?3, gender), phone = COALESCE(?4, phone), parent_name = COALESCE(?5, parent_name),
         parent_phone = COALESCE(?6, parent_phone), address = COALESCE(?7, address),
         group_name = COALESCE(?8, group_name), status = COALESCE(?9, status),
         is_focus = COALESCE(?10, is_focus), remark = COALESCE(?11, remark), updated_at = ?12
         WHERE id = ?13"
    )
    .bind(&name)
    .bind(&student_no)
    .bind(&gender)
    .bind(&phone)
    .bind(&parent_name)
    .bind(&parent_phone)
    .bind(&address)
    .bind(&group_name)
    .bind(&status)
    .bind(is_focus)
    .bind(&remark)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "学号在当前届次中已存在".to_string()
        } else {
            format!("更新学生失败: {}", e)
        }
    })?;

    get_student_internal(pool, id).await
}

#[tauri::command]
pub async fn delete_student(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = &state.db;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let student = get_student_internal(pool, id).await?;
    check_cohort_readonly(pool, student.cohort_id).await?;

    // 逻辑删除
    sqlx::query("UPDATE student SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("删除学生失败: {}", e))?;

    Ok(())
}

async fn get_student_internal(pool: &SqlitePool, id: i64) -> Result<Student, String> {
    sqlx::query_as::<_, Student>("SELECT * FROM student WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("获取学生失败: {}", e))
}

// Excel 导入预览（不写入数据库，返回解析结果供用户确认）
#[tauri::command]
pub async fn preview_students_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;

    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(&file_path)
        .map_err(|e| format!("无法打开 Excel 文件: {}", e))?;

    let sheet_name = workbook.sheet_names().first().cloned()
        .ok_or("Excel 文件没有工作表".to_string())?;

    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    let valid_genders = ["男", "女"];
    let mut preview_rows: Vec<serde_json::Value> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut valid_count = 0i64;

    // 检测文件内重复学号
    let mut file_student_nos: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; }

        let student_no = row.get(0).map(|c| c.to_string().trim().to_string()).unwrap_or_default();
        let name = row.get(1).map(|c| c.to_string().trim().to_string()).unwrap_or_default();

        if student_no.is_empty() && name.is_empty() { continue; }

        let mut row_errors: Vec<String> = Vec::new();

        if student_no.is_empty() {
            row_errors.push("学号为空".to_string());
        }
        if name.is_empty() {
            row_errors.push("姓名为空".to_string());
        }

        // 检测文件内重复学号
        if !student_no.is_empty() && !file_student_nos.insert(student_no.clone()) {
            row_errors.push(format!("学号'{}'在文件中重复出现", student_no));
        }

        let gender_val = row.get(2).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        if let Some(ref g) = gender_val {
            if !valid_genders.contains(&g.as_str()) {
                row_errors.push(format!("性别'{}'无效", g));
            }
        }

        // 检查数据库中是否已有该学号
        if !student_no.is_empty() {
            let exists: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM student WHERE cohort_id = ?1 AND student_no = ?2 AND deleted_at IS NULL"
            ).bind(cohort_id).bind(&student_no)
            .fetch_one(pool).await.map_err(|e| e.to_string())?;
            if exists.0 > 0 {
                row_errors.push(format!("学号'{}'已在数据库中存在", student_no));
            }
        }

        let phone = row.get(3).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        let parent_name = row.get(4).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        let parent_phone = row.get(5).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        let group_name = row.get(6).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());

        if row_errors.is_empty() {
            valid_count += 1;
        } else {
            errors.push(format!("第{}行: {}", row_idx + 1, row_errors.join(", ")));
        }

        preview_rows.push(serde_json::json!({
            "row": row_idx + 1,
            "student_no": student_no,
            "name": name,
            "gender": gender_val,
            "phone": phone,
            "parent_name": parent_name,
            "parent_phone": parent_phone,
            "group_name": group_name,
            "valid": row_errors.is_empty(),
            "errors": row_errors
        }));
    }

    let total_rows = preview_rows.len() as i64;

    Ok(serde_json::json!({
        "total_rows": total_rows,
        "valid_rows": valid_count,
        "error_rows": total_rows - valid_count,
        "rows": preview_rows,
        "errors": errors
    }))
}

// Excel 导入
#[tauri::command]
pub async fn import_students_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let pool = &state.db;
    check_cohort_readonly(pool, cohort_id).await?;

    // 打开 Excel 文件
    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(&file_path)
        .map_err(|e| format!("无法打开 Excel 文件: {}", e))?;

    let sheet_name = workbook.sheet_names().first().cloned()
        .ok_or("Excel 文件没有工作表".to_string())?;

    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 阶段1: 预校验所有行，收集校验错误
    // 模板列: 0=学号, 1=姓名, 2=性别, 3=联系电话, 4=家长姓名, 5=家长电话, 6=小组
    let valid_genders = ["男", "女"];

    let mut parsed_rows: Vec<(String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut file_student_nos: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; } // 跳过标题行

        let student_no = row.get(0).map(|c| c.to_string().trim().to_string()).unwrap_or_default();
        let name = row.get(1).map(|c| c.to_string().trim().to_string()).unwrap_or_default();

        // 跳过全空行
        if student_no.is_empty() && name.is_empty() {
            continue;
        }

        // 必填项校验
        if student_no.is_empty() {
            errors.push(format!("第{}行: 学号为空", row_idx + 1));
            continue;
        }
        if name.is_empty() {
            errors.push(format!("第{}行: 姓名为空", row_idx + 1));
            continue;
        }

        // 检测文件内重复学号
        if !file_student_nos.insert(student_no.clone()) {
            errors.push(format!("第{}行: 学号 '{}' 在文件中重复出现", row_idx + 1, student_no));
            continue;
        }

        // 性别枚举校验
        let gender = row.get(2).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        if let Some(ref g) = gender {
            if !valid_genders.contains(&g.as_str()) {
                errors.push(format!("第{}行: 性别 '{}' 无效，只允许 '男' 或 '女'", row_idx + 1, g));
                continue;
            }
        }

        // 检查学号是否已存在
        let exists: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM student WHERE cohort_id = ?1 AND student_no = ?2 AND deleted_at IS NULL"
        )
        .bind(cohort_id)
        .bind(&student_no)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
        if exists.0 > 0 {
            errors.push(format!("第{}行: 学号 '{}' 在当前届次已存在", row_idx + 1, student_no));
            continue;
        }

        let phone = row.get(3).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        let parent_name = row.get(4).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        let parent_phone = row.get(5).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());
        let group_name = row.get(6).map(|c| c.to_string().trim().to_string()).filter(|s| !s.is_empty());

        parsed_rows.push((student_no, name, gender, phone, parent_name, parent_phone, group_name));
    }

    // 阶段2: 如果有任何校验错误，不写入任何数据
    if !errors.is_empty() {
        return Ok(serde_json::json!({
            "success": 0i64,
            "errors": errors
        }));
    }

    // 阶段3: 事务批量导入（原子性：全部成功或全部失败）
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for (student_no, name, gender, phone, parent_name, parent_phone, group_name) in &parsed_rows {
        sqlx::query(
            "INSERT INTO student (cohort_id, name, student_no, gender, phone, parent_name, parent_phone, group_name, status, is_focus, remark, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '正常', 0, NULL, ?9, ?9)"
        )
        .bind(cohort_id)
        .bind(name)
        .bind(student_no)
        .bind(gender)
        .bind(phone)
        .bind(parent_name)
        .bind(parent_phone)
        .bind(group_name)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("导入失败: {}", e))?;
    }

    tx.commit().await.map_err(|e| format!("提交导入数据失败: {}", e))?;

    let success_count = parsed_rows.len() as i64;
    let empty_errors: Vec<String> = Vec::new();
    Ok(serde_json::json!({
        "success": success_count,
        "errors": empty_errors
    }))
}

#[tauri::command]
pub async fn export_students_excel(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
) -> Result<(), String> {
    let students = sqlx::query_as::<_, Student>(
        "SELECT * FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY student_no ASC"
    )
    .bind(cohort_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    use rust_xlsxwriter::*;
    let mut workbook = Workbook::new();
    let mut sheet = Worksheet::new();
    let header = ["学号", "姓名", "性别", "联系电话", "家长姓名", "家长电话", "小组", "状态"];
    for (ci, h) in header.iter().enumerate() {
        sheet.write_string(0, ci as u16, *h).map_err(|e| e.to_string())?;
    }
    for (i, s) in students.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet.write_string(row, 0, &s.student_no).map_err(|e| e.to_string())?;
        sheet.write_string(row, 1, &s.name).map_err(|e| e.to_string())?;
        if let Some(ref v) = s.gender { sheet.write_string(row, 2, v).map_err(|e| e.to_string())?; }
        if let Some(ref v) = s.phone { sheet.write_string(row, 3, v).map_err(|e| e.to_string())?; }
        if let Some(ref v) = s.parent_name { sheet.write_string(row, 4, v).map_err(|e| e.to_string())?; }
        if let Some(ref v) = s.parent_phone { sheet.write_string(row, 5, v).map_err(|e| e.to_string())?; }
        if let Some(ref v) = s.group_name { sheet.write_string(row, 6, v).map_err(|e| e.to_string())?; }
        sheet.write_string(row, 7, &s.status).map_err(|e| e.to_string())?;
    }
    workbook.push_worksheet(sheet);
    workbook.save(&file_path).map_err(|e| format!("导出失败: {}", e))?;

    Ok(())
}
