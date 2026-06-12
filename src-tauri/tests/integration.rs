// 集成测试：数据库隔离、归档只读、跨届校验、备份往返
// 使用方法：cargo test -- --nocapture

use calamine::{open_workbook, Reader, Xlsx};
use class_copilot_lib::commands::{
    affair, attendance, backup, cohort, exam, homework, stats, student,
};
use class_copilot_lib::AppState;
use rust_xlsxwriter::Workbook;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 创建测试用内存数据库，包含完整表结构
async fn create_test_db() -> SqlitePool {
    let db_path = std::env::temp_dir().join(format!("test_db_{}.db", unique_suffix()));
    let _ = std::fs::remove_file(&db_path);
    let conn_options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path.display()))
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(conn_options)
        .await
        .unwrap();

    let migrations = vec![
        "CREATE TABLE IF NOT EXISTS cohort (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_name TEXT NOT NULL, class_name TEXT NOT NULL,
            grade_name TEXT, school_name TEXT, head_teacher TEXT,
            admission_year INTEGER, graduation_year INTEGER, semester TEXT,
            status TEXT NOT NULL DEFAULT '使用中',
            is_current INTEGER NOT NULL DEFAULT 0,
            archive_time TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS student (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, name TEXT NOT NULL,
            student_no TEXT NOT NULL, gender TEXT, phone TEXT,
            parent_name TEXT, parent_phone TEXT, address TEXT,
            group_name TEXT, status TEXT NOT NULL DEFAULT '正常',
            is_focus INTEGER NOT NULL DEFAULT 0, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT,
            UNIQUE(cohort_id, student_no),
            FOREIGN KEY(cohort_id) REFERENCES cohort(id)
        );",
        "CREATE TABLE IF NOT EXISTS subject (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE, sort_order INTEGER DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS homework (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, title TEXT NOT NULL,
            subject_id INTEGER, subject_name TEXT, description TEXT,
            attachment_name TEXT, attachment_path TEXT,
            publish_date TEXT NOT NULL, deadline TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(subject_id) REFERENCES subject(id)
        );",
        "CREATE TABLE IF NOT EXISTS homework_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            homework_id INTEGER NOT NULL, student_id INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT '未登记', submit_time TEXT,
            remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            UNIQUE(homework_id, student_id),
            FOREIGN KEY(homework_id) REFERENCES homework(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS attendance (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, student_id INTEGER NOT NULL,
            attendance_date TEXT NOT NULL, status TEXT NOT NULL DEFAULT '正常',
            leave_type TEXT, leave_start_date TEXT, leave_end_date TEXT,
            reason TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            UNIQUE(student_id, attendance_date),
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS exam (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, name TEXT NOT NULL,
            exam_type TEXT, exam_date TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id)
        );",
        "CREATE TABLE IF NOT EXISTS exam_subject_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exam_id INTEGER NOT NULL,
            subject_id INTEGER NOT NULL,
            full_score REAL NOT NULL DEFAULT 100,
            pass_score REAL NOT NULL DEFAULT 60,
            excellent_score REAL NOT NULL DEFAULT 90,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(exam_id, subject_id),
            FOREIGN KEY(exam_id) REFERENCES exam(id),
            FOREIGN KEY(subject_id) REFERENCES subject(id)
        );",
        "CREATE TABLE IF NOT EXISTS score (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exam_id INTEGER NOT NULL, subject_id INTEGER NOT NULL,
            student_id INTEGER NOT NULL, score_value REAL,
            rank_no INTEGER, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            UNIQUE(exam_id, subject_id, student_id),
            FOREIGN KEY(exam_id) REFERENCES exam(id),
            FOREIGN KEY(subject_id) REFERENCES subject(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS notice (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, title TEXT NOT NULL,
            content TEXT, publish_date TEXT NOT NULL, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT
        );",
        "CREATE TABLE IF NOT EXISTS duty (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, duty_date TEXT NOT NULL,
            student_id INTEGER, group_name TEXT, duty_content TEXT,
            status TEXT NOT NULL DEFAULT '未完成', remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS behavior_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, student_id INTEGER NOT NULL,
            type TEXT NOT NULL, title TEXT NOT NULL,
            score INTEGER DEFAULT 0, description TEXT,
            record_date TEXT NOT NULL,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS system_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            config_key TEXT NOT NULL UNIQUE, config_value TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS class_fee (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL,
            fee_date TEXT NOT NULL,
            fee_type TEXT NOT NULL,
            category TEXT,
            title TEXT NOT NULL,
            amount REAL NOT NULL,
            student_id INTEGER,
            payment_status TEXT,
            voucher_path TEXT,
            remark TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT
        );",
    ];

    for m in &migrations {
        sqlx::query(m).execute(&pool).await.unwrap();
    }

    pool
}

fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn write_student_import_excel(
    slug: &str,
    rows: &[(&str, &str, &str, &str, &str, &str, &str)],
) -> String {
    let file_path =
        std::env::temp_dir().join(format!("student_import_{}_{}.xlsx", slug, unique_suffix()));
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = [
        "学号",
        "姓名",
        "性别",
        "联系电话",
        "家长姓名",
        "家长电话",
        "小组",
    ];
    for (idx, header) in headers.iter().enumerate() {
        worksheet.write_string(0, idx as u16, *header).unwrap();
    }
    for (row_idx, row) in rows.iter().enumerate() {
        let line = (row_idx + 1) as u32;
        worksheet.write_string(line, 0, row.0).unwrap();
        worksheet.write_string(line, 1, row.1).unwrap();
        worksheet.write_string(line, 2, row.2).unwrap();
        worksheet.write_string(line, 3, row.3).unwrap();
        worksheet.write_string(line, 4, row.4).unwrap();
        worksheet.write_string(line, 5, row.5).unwrap();
        worksheet.write_string(line, 6, row.6).unwrap();
    }
    workbook.save(&file_path).unwrap();
    file_path.to_string_lossy().to_string()
}

fn write_score_import_excel(slug: &str, rows: &[(&str, &str, &str)]) -> String {
    let file_path =
        std::env::temp_dir().join(format!("score_import_{}_{}.xlsx", slug, unique_suffix()));
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let headers = ["学号", "姓名", "成绩"];
    for (idx, header) in headers.iter().enumerate() {
        worksheet.write_string(0, idx as u16, *header).unwrap();
    }
    for (row_idx, row) in rows.iter().enumerate() {
        let line = (row_idx + 1) as u32;
        worksheet.write_string(line, 0, row.0).unwrap();
        worksheet.write_string(line, 1, row.1).unwrap();
        worksheet.write_string(line, 2, row.2).unwrap();
    }
    workbook.save(&file_path).unwrap();
    file_path.to_string_lossy().to_string()
}

fn xlsx_row_count(file_path: &str) -> usize {
    let mut workbook: Xlsx<_> = open_workbook(file_path).unwrap();
    let sheet_name = workbook.sheet_names().first().cloned().unwrap();
    let range = workbook.worksheet_range(&sheet_name).unwrap();
    range.rows().count()
}

/// 插入测试数据：届次 + 学生 + 作业 + 考勤
async fn seed_test_data(pool: &SqlitePool) -> (i64, i64) {
    let now = now_str();
    // 插入届次
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('2024级', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(pool)
    .await
    .unwrap();

    // 插入学生
    let student_id: (i64,) = sqlx::query_as(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, '测试学生', 'S001', '正常', 0, ?2, ?2) RETURNING id"
    ).bind(cohort_id.0).bind(&now).fetch_one(pool).await.unwrap();

    // 插入作业
    let hw_id: (i64,) = sqlx::query_as(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, '数学作业', ?2, ?3, ?3) RETURNING id",
    )
    .bind(cohort_id.0)
    .bind(&today_str())
    .bind(&now)
    .fetch_one(pool)
    .await
    .unwrap();

    // 插入作业记录
    sqlx::query(
        "INSERT INTO homework_record (homework_id, student_id, status, created_at, updated_at)
         VALUES (?1, ?2, '已完成', ?3, ?3)",
    )
    .bind(hw_id.0)
    .bind(student_id.0)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();

    // 插入考勤
    sqlx::query(
        "INSERT INTO attendance (cohort_id, student_id, attendance_date, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, '正常', ?4, ?4)"
    ).bind(cohort_id.0).bind(student_id.0).bind(&today_str()).bind(&now).execute(pool).await.unwrap();

    (cohort_id.0, student_id.0)
}

async fn insert_homework_with_records(
    pool: &SqlitePool,
    cohort_id: i64,
    title: &str,
    publish_date: &str,
) -> (i64, Vec<i64>) {
    let now = now_str();
    let homework_id: (i64,) = sqlx::query_as(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4) RETURNING id",
    )
    .bind(cohort_id)
    .bind(title)
    .bind(publish_date)
    .bind(&now)
    .fetch_one(pool)
    .await
    .unwrap();

    let student_ids: Vec<i64> = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY student_no ASC"
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.0)
    .collect();

    for student_id in &student_ids {
        sqlx::query(
            "INSERT INTO homework_record (homework_id, student_id, status, created_at, updated_at)
             VALUES (?1, ?2, '未登记', ?3, ?3)",
        )
        .bind(homework_id.0)
        .bind(student_id)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();
    }

    (homework_id.0, student_ids)
}

async fn insert_student(pool: &SqlitePool, cohort_id: i64, name: &str, student_no: &str) -> i64 {
    let now = now_str();
    let student_id: (i64,) = sqlx::query_as(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, ?2, ?3, '正常', 0, ?4, ?4) RETURNING id"
    )
    .bind(cohort_id)
    .bind(name)
    .bind(student_no)
    .bind(&now)
    .fetch_one(pool)
    .await
    .unwrap();
    student_id.0
}

/// 验证数据完整性
async fn verify_data_integrity(pool: &SqlitePool, cohort_id: i64) -> bool {
    let c: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort WHERE id = ?1")
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .unwrap();
    if c.0 != 1 {
        return false;
    }
    let s: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student WHERE cohort_id = ?1")
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .unwrap();
    if s.0 != 1 {
        return false;
    }
    let h: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM homework WHERE cohort_id = ?1")
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .unwrap();
    if h.0 != 1 {
        return false;
    }
    let hr: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM homework_record hr JOIN homework h ON hr.homework_id = h.id WHERE h.cohort_id = ?1"
    ).bind(cohort_id).fetch_one(pool).await.unwrap();
    if hr.0 != 1 {
        return false;
    }
    let a: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM attendance WHERE cohort_id = ?1")
        .bind(cohort_id)
        .fetch_one(pool)
        .await
        .unwrap();
    if a.0 != 1 {
        return false;
    }
    true
}

/// 从 SqlitePool 构造 tauri::State<AppState>（仅测试用）
/// tauri v2 中 State 没有公开构造函数，通过 transmute 绕过
fn make_app_state(pool: SqlitePool) -> tauri::State<'static, AppState> {
    let leaked: &'static AppState = Box::leak(Box::new(AppState {
        db: pool,
        app_handle: Arc::new(tokio::sync::Mutex::new(None)),
    }));
    // SAFETY: State 内部只是一个引用包装，leaked 生命周期为 'static，使用安全
    unsafe { std::mem::transmute(leaked) }
}

/// 创建测试用文件数据库（每个测试必须传入唯一的 name，避免并行测试竞争同一文件）
async fn create_test_file_db(name: &str) -> SqlitePool {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("test_backup_{}_{}.db", name, unique_suffix()));
    let _ = std::fs::remove_file(&db_path);

    let conn_options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path.display()))
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(conn_options)
        .await
        .unwrap();

    let migrations = vec![
        "CREATE TABLE IF NOT EXISTS cohort (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_name TEXT NOT NULL, class_name TEXT NOT NULL,
            grade_name TEXT, school_name TEXT, head_teacher TEXT,
            admission_year INTEGER, graduation_year INTEGER, semester TEXT,
            status TEXT NOT NULL DEFAULT '使用中',
            is_current INTEGER NOT NULL DEFAULT 0,
            archive_time TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS student (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, name TEXT NOT NULL,
            student_no TEXT NOT NULL, gender TEXT, phone TEXT,
            parent_name TEXT, parent_phone TEXT, address TEXT,
            group_name TEXT, status TEXT NOT NULL DEFAULT '正常',
            is_focus INTEGER NOT NULL DEFAULT 0, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT,
            UNIQUE(cohort_id, student_no),
            FOREIGN KEY(cohort_id) REFERENCES cohort(id)
        );",
        "CREATE TABLE IF NOT EXISTS subject (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE, sort_order INTEGER DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS homework (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, title TEXT NOT NULL,
            subject_id INTEGER, subject_name TEXT, description TEXT,
            attachment_name TEXT, attachment_path TEXT,
            publish_date TEXT NOT NULL, deadline TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(subject_id) REFERENCES subject(id)
        );",
        "CREATE TABLE IF NOT EXISTS homework_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            homework_id INTEGER NOT NULL, student_id INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT '未登记', submit_time TEXT,
            remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            UNIQUE(homework_id, student_id),
            FOREIGN KEY(homework_id) REFERENCES homework(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS attendance (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, student_id INTEGER NOT NULL,
            attendance_date TEXT NOT NULL, status TEXT NOT NULL DEFAULT '正常',
            leave_type TEXT, leave_start_date TEXT, leave_end_date TEXT,
            reason TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            UNIQUE(student_id, attendance_date),
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS exam (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, name TEXT NOT NULL,
            exam_type TEXT, exam_date TEXT, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id)
        );",
        "CREATE TABLE IF NOT EXISTS exam_subject_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exam_id INTEGER NOT NULL,
            subject_id INTEGER NOT NULL,
            full_score REAL NOT NULL DEFAULT 100,
            pass_score REAL NOT NULL DEFAULT 60,
            excellent_score REAL NOT NULL DEFAULT 90,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(exam_id, subject_id),
            FOREIGN KEY(exam_id) REFERENCES exam(id),
            FOREIGN KEY(subject_id) REFERENCES subject(id)
        );",
        "CREATE TABLE IF NOT EXISTS score (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exam_id INTEGER NOT NULL, subject_id INTEGER NOT NULL,
            student_id INTEGER NOT NULL, score_value REAL,
            rank_no INTEGER, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            UNIQUE(exam_id, subject_id, student_id),
            FOREIGN KEY(exam_id) REFERENCES exam(id),
            FOREIGN KEY(subject_id) REFERENCES subject(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS notice (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, title TEXT NOT NULL,
            content TEXT, publish_date TEXT NOT NULL, remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            deleted_at TEXT
        );",
        "CREATE TABLE IF NOT EXISTS duty (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, duty_date TEXT NOT NULL,
            student_id INTEGER, group_name TEXT, duty_content TEXT,
            status TEXT NOT NULL DEFAULT '未完成', remark TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS behavior_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, student_id INTEGER NOT NULL,
            type TEXT NOT NULL, title TEXT NOT NULL,
            score INTEGER DEFAULT 0, description TEXT,
            record_date TEXT NOT NULL,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            FOREIGN KEY(cohort_id) REFERENCES cohort(id),
            FOREIGN KEY(student_id) REFERENCES student(id)
        );",
        "CREATE TABLE IF NOT EXISTS system_config (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            config_key TEXT NOT NULL UNIQUE, config_value TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS class_fee (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL,
            fee_date TEXT NOT NULL,
            fee_type TEXT NOT NULL,
            category TEXT,
            title TEXT NOT NULL,
            amount REAL NOT NULL,
            student_id INTEGER,
            payment_status TEXT,
            voucher_path TEXT,
            remark TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT
        );",
    ];

    for m in &migrations {
        sqlx::query(m).execute(&pool).await.unwrap();
    }

    pool
}

// ==================== 阶段 13 核心流程验收测试 ====================
#[tokio::test]
async fn test_e2e_first_launch_create_cohort_import_students_enter_dashboard() {
    let pool = create_test_db().await;
    let state = make_app_state(pool.clone());
    let cohort_record = cohort::create_cohort(
        state,
        "2026届".to_string(),
        "1班".to_string(),
        None,
        None,
        None,
        Some(2026),
        Some(2029),
        Some("2026 秋季学期".to_string()),
        None,
    )
    .await
    .unwrap();

    let import_path = write_student_import_excel(
        "e2e_first_launch",
        &[
            ("S001", "张三", "男", "", "", "", "一组"),
            ("S002", "李四", "女", "", "", "", "二组"),
        ],
    );
    let preview_state = make_app_state(pool.clone());
    let preview =
        student::preview_students_excel(preview_state, cohort_record.id, import_path.clone())
            .await
            .unwrap();
    assert_eq!(preview["valid_rows"], 2);

    let import_state = make_app_state(pool.clone());
    let import_result =
        student::import_students_excel(import_state, cohort_record.id, import_path.clone())
            .await
            .unwrap();
    assert_eq!(import_result["success"], 2);

    let current_state = make_app_state(pool.clone());
    let current = cohort::get_current_cohort(current_state)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.id, cohort_record.id);

    let dashboard_state = make_app_state(pool.clone());
    let dashboard = stats::get_dashboard_stats(dashboard_state, cohort_record.id)
        .await
        .unwrap();
    assert_eq!(dashboard["total_students"], 2);

    let _ = std::fs::remove_file(import_path);
}

#[tokio::test]
async fn test_e2e_homework_batch_register_incomplete_export() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('作业验收', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    insert_student(&pool, cohort_id.0, "张三", "HW001").await;
    insert_student(&pool, cohort_id.0, "李四", "HW002").await;
    insert_student(&pool, cohort_id.0, "王五", "HW003").await;

    let (homework_id, record_student_ids) =
        insert_homework_with_records(&pool, cohort_id.0, "数学作业", &today_str()).await;

    let records_state = make_app_state(pool.clone());
    let records = homework::get_homework_records(records_state, homework_id)
        .await
        .unwrap();
    assert_eq!(records.len(), 3);
    let batch_state = make_app_state(pool.clone());
    homework::batch_update_homework_records(
        batch_state,
        homework_id,
        record_student_ids.iter().take(2).copied().collect(),
        "已完成".to_string(),
    )
    .await
    .unwrap();

    let export_path =
        std::env::temp_dir().join(format!("homework_incomplete_{}.xlsx", unique_suffix()));
    let export_state = make_app_state(pool.clone());
    homework::export_incomplete_homework(
        export_state,
        homework_id,
        export_path.to_string_lossy().to_string(),
    )
    .await
    .unwrap();
    assert_eq!(xlsx_row_count(export_path.to_string_lossy().as_ref()), 2);
    let _ = std::fs::remove_file(export_path);
}

#[tokio::test]
async fn test_e2e_attendance_abnormal_query_export() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('考勤验收', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    let student_a = insert_student(&pool, cohort_id.0, "张三", "AT001").await;
    let student_b = insert_student(&pool, cohort_id.0, "李四", "AT002").await;
    let date = today_str();

    let normal_state = make_app_state(pool.clone());
    attendance::set_all_attendance_normal(normal_state, cohort_id.0, date.clone())
        .await
        .unwrap();

    let abnormal_state = make_app_state(pool.clone());
    attendance::save_attendance(
        abnormal_state,
        cohort_id.0,
        date.clone(),
        vec![
            attendance::AttendanceRecord {
                student_id: student_a,
                status: "迟到".to_string(),
                leave_type: None,
                leave_start_date: None,
                leave_end_date: None,
                reason: Some("地铁晚点".to_string()),
                remark: None,
            },
            attendance::AttendanceRecord {
                student_id: student_b,
                status: "正常".to_string(),
                leave_type: None,
                leave_start_date: None,
                leave_end_date: None,
                reason: None,
                remark: None,
            },
        ],
    )
    .await
    .unwrap();

    let query_state = make_app_state(pool.clone());
    let query = attendance::query_attendance(
        query_state,
        cohort_id.0,
        Some(date.clone()),
        Some(date.clone()),
        None,
        Some("迟到".to_string()),
        Some(1),
        Some(20),
    )
    .await
    .unwrap();
    assert_eq!(query["total"], 1);

    let export_path =
        std::env::temp_dir().join(format!("attendance_export_{}.xlsx", unique_suffix()));
    let export_state = make_app_state(pool.clone());
    attendance::export_attendance_excel(
        export_state,
        cohort_id.0,
        export_path.to_string_lossy().to_string(),
        Some(date.clone()),
        Some(date.clone()),
    )
    .await
    .unwrap();
    assert!(xlsx_row_count(export_path.to_string_lossy().as_ref()) >= 2);
    let _ = std::fs::remove_file(export_path);
}

#[tokio::test]
async fn test_get_current_cohort_tolerates_duplicate_current_rows() {
    let pool = create_test_db().await;
    let now = now_str();

    sqlx::query(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('A届', '1班', '使用中', 1, ?1, ?1),
                ('B届', '2班', '使用中', 1, ?1, ?1),
                ('C届', '3班', '使用中', 1, ?1, ?1)",
    )
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    let current_state = make_app_state(pool.clone());
    let current = cohort::get_current_cohort(current_state)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(current.cohort_name, "A届");
    assert_eq!(current.class_name, "1班");
}

#[tokio::test]
async fn test_get_homeworks_returns_zero_completion_rate_without_records() {
    let pool = create_test_db().await;
    let now = now_str();
    let today = today_str();

    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('作业测试', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, '无记录作业', ?2, ?3, ?3)",
    )
    .bind(cohort_id.0)
    .bind(&today)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    let state = make_app_state(pool.clone());
    let result = homework::get_homeworks(state, cohort_id.0, None, None, None, None, Some(1), Some(10))
        .await
        .unwrap();

    let data = result["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["completion_rate"].as_f64().unwrap(), 0.0);
    assert_eq!(data[0]["incomplete_count"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_e2e_score_import_rankings_and_trend() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('成绩验收', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    insert_student(&pool, cohort_id.0, "张三", "SC001").await;
    insert_student(&pool, cohort_id.0, "李四", "SC002").await;
    let subject_id: (i64,) = sqlx::query_as(
        "INSERT INTO subject (name, sort_order, is_active, created_at, updated_at)
         VALUES ('数学', 1, 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    let exam_state = make_app_state(pool.clone());
    let exam_row = exam::create_exam(
        exam_state,
        cohort_id.0,
        "期中考试".to_string(),
        Some("期中".to_string()),
        Some(today_str()),
        None,
    )
    .await
    .unwrap();

    let config_state = make_app_state(pool.clone());
    exam::save_exam_subject_configs(
        config_state,
        exam_row.id,
        vec![exam::ExamSubjectConfigInput {
            subject_id: subject_id.0,
            full_score: 120.0,
            pass_score: 72.0,
            excellent_score: 96.0,
            sort_order: Some(1),
        }],
    )
    .await
    .unwrap();

    let score_path = write_score_import_excel(
        "e2e_scores",
        &[("SC001", "张三", "110"), ("SC002", "李四", "92")],
    );
    let preview_state = make_app_state(pool.clone());
    let preview =
        exam::preview_scores_excel(preview_state, exam_row.id, subject_id.0, score_path.clone())
            .await
            .unwrap();
    assert_eq!(preview["valid_rows"], 2);

    let import_state = make_app_state(pool.clone());
    let import_result =
        exam::import_scores_excel(import_state, exam_row.id, subject_id.0, score_path.clone())
            .await
            .unwrap();
    assert_eq!(import_result["success"], 2);

    let ranking_state = make_app_state(pool.clone());
    let rankings = exam::score_rankings(ranking_state, exam_row.id)
        .await
        .unwrap();
    assert_eq!(rankings[0]["student_no"], "SC001");
    assert_eq!(rankings[0]["rank_no"], 1);

    let trend_state = make_app_state(pool.clone());
    let trend = stats::score_trend_statistics(trend_state, cohort_id.0)
        .await
        .unwrap();
    assert_eq!(trend.len(), 1);

    let _ = std::fs::remove_file(score_path);
}

#[tokio::test]
async fn test_e2e_affairs_records_creation() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('事务验收', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    let student_id = insert_student(&pool, cohort_id.0, "赵六", "AF001").await;

    let notice_state = make_app_state(pool.clone());
    affair::create_notice(
        notice_state,
        cohort_id.0,
        "班会通知".to_string(),
        Some("明天早上 8 点班会".to_string()),
        Some(today_str()),
        None,
    )
    .await
    .unwrap();

    let duty_state = make_app_state(pool.clone());
    affair::create_duty(
        duty_state,
        cohort_id.0,
        today_str(),
        None,
        Some(student_id),
        Some("教室卫生".to_string()),
        Some("未完成".to_string()),
        None,
    )
    .await
    .unwrap();

    let behavior_state = make_app_state(pool.clone());
    affair::create_behavior_record(
        behavior_state,
        cohort_id.0,
        student_id,
        "表扬".to_string(),
        "课堂表现优秀".to_string(),
        Some(2),
        Some("主动回答问题".to_string()),
        Some(today_str()),
    )
    .await
    .unwrap();

    let fee_state = make_app_state(pool.clone());
    affair::create_class_fee_record(
        fee_state,
        cohort_id.0,
        Some(today_str()),
        "收入".to_string(),
        Some("班费".to_string()),
        "班费缴纳".to_string(),
        200.0,
        Some(student_id),
        Some("已缴费".to_string()),
        None,
        None,
    )
    .await
    .unwrap();

    let query_state = make_app_state(pool.clone());
    let fees = affair::get_class_fee_records(
        query_state,
        cohort_id.0,
        None,
        None,
        None,
        Some(1),
        Some(20),
    )
    .await
    .unwrap();
    assert_eq!(fees["total"], 1);
    assert_eq!(fees["summary"]["balance"], 200.0);
}

#[tokio::test]
async fn test_e2e_archive_readonly_and_unarchive_recovery() {
    let pool = create_test_db().await;
    let state = make_app_state(pool.clone());
    let cohort_row = cohort::create_cohort(
        state,
        "归档验收".to_string(),
        "1班".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let archive_state = make_app_state(pool.clone());
    cohort::archive_cohort(archive_state, cohort_row.id)
        .await
        .unwrap();

    let archived_state = make_app_state(pool.clone());
    let archived_result = student::create_student(
        archived_state,
        cohort_row.id,
        "归档学生".to_string(),
        "ARCH100".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(archived_result.is_err());

    let unarchive_state = make_app_state(pool.clone());
    cohort::unarchive_cohort(unarchive_state, cohort_row.id)
        .await
        .unwrap();

    let restored_state = make_app_state(pool.clone());
    let restored_student = student::create_student(
        restored_state,
        cohort_row.id,
        "恢复学生".to_string(),
        "ARCH101".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(restored_student.student_no, "ARCH101");
}

#[tokio::test]
async fn test_e2e_switch_cohorts_and_verify_isolation() {
    let pool = create_test_db().await;
    let state_a = make_app_state(pool.clone());
    let cohort_a = cohort::create_cohort(
        state_a,
        "A届".to_string(),
        "1班".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let state_b = make_app_state(pool.clone());
    let cohort_b = cohort::create_cohort(
        state_b,
        "B届".to_string(),
        "2班".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    insert_student(&pool, cohort_a.id, "甲同学", "ISO001").await;
    insert_student(&pool, cohort_b.id, "乙同学", "ISO002").await;

    let current_state = make_app_state(pool.clone());
    cohort::set_current_cohort(current_state, cohort_b.id)
        .await
        .unwrap();

    let get_current_state = make_app_state(pool.clone());
    let current = cohort::get_current_cohort(get_current_state)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.id, cohort_b.id);

    let students_a_state = make_app_state(pool.clone());
    let students_a = student::get_students(
        students_a_state,
        cohort_a.id,
        None,
        None,
        None,
        None,
        None,
        Some(1),
        Some(20),
    )
    .await
    .unwrap();
    let students_b_state = make_app_state(pool.clone());
    let students_b = student::get_students(
        students_b_state,
        cohort_b.id,
        None,
        None,
        None,
        None,
        None,
        Some(1),
        Some(20),
    )
    .await
    .unwrap();
    assert_eq!(students_a.total, 1);
    assert_eq!(students_b.total, 1);
    assert_ne!(students_a.data[0].student_no, students_b.data[0].student_no);
}

#[tokio::test]
async fn test_e2e_backup_modify_restore_consistency() {
    let pool = create_test_file_db("backup_modify_restore").await;
    let (cohort_id, _) = seed_test_data(&pool).await;
    let backup_path = std::env::temp_dir().join(format!("backup_restore_{}.bak", unique_suffix()));
    let backup_str = backup_path.to_string_lossy().to_string();

    let backup_state = make_app_state(pool.clone());
    backup::create_backup(backup_state, backup_str.clone())
        .await
        .unwrap();

    let now = now_str();
    sqlx::query(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, '恢复前新增', 'RESTORE999', '正常', 0, ?2, ?2)"
    )
    .bind(cohort_id)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    let before_restore: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM student WHERE cohort_id = ?1")
            .bind(cohort_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before_restore.0, 2);

    let restore_state = make_app_state(pool.clone());
    backup::restore_backup(restore_state, backup_str.clone())
        .await
        .unwrap();

    let after_restore: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student WHERE cohort_id = ?1")
        .bind(cohort_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after_restore.0, 1);

    let _ = std::fs::remove_file(backup_path);
}

#[tokio::test]
async fn test_e2e_export_one_cohort_without_other_cohort_data() {
    let pool = create_test_file_db("export_isolation").await;
    let now = now_str();
    let cohort_a: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('导出A', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    let cohort_b: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('导出B', '2班', '使用中', 0, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    insert_student(&pool, cohort_a.0, "甲", "EXP001").await;
    insert_student(&pool, cohort_b.0, "乙", "EXP002").await;

    let export_path = std::env::temp_dir().join(format!("cohort_only_{}.zip", unique_suffix()));
    let export_state = make_app_state(pool.clone());
    backup::export_cohort(
        export_state,
        cohort_a.0,
        export_path.to_string_lossy().to_string(),
    )
    .await
    .unwrap();

    let file = std::fs::File::open(&export_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut students = archive.by_name("students.json").unwrap();
    let mut json = String::new();
    use std::io::Read;
    students.read_to_string(&mut json).unwrap();
    assert!(json.contains("EXP001"));
    assert!(!json.contains("EXP002"));

    let _ = std::fs::remove_file(export_path);
}

// ==================== 测试: 备份真实往返（调用生产命令 create_backup + restore_backup） ====================
#[tokio::test]
async fn test_backup_round_trip() {
    let pool = create_test_file_db("roundtrip").await;
    let (cohort_id, _) = seed_test_data(&pool).await;
    assert!(
        verify_data_integrity(&pool, cohort_id).await,
        "初始数据完整"
    );

    // 确保 WAL 数据写入主文件
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await
        .unwrap();

    let temp_dir = std::env::temp_dir();
    let backup_path = temp_dir.join(format!("test_backup_roundtrip_{}.bak", std::process::id()));
    let backup_str = backup_path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&backup_path);

    // Step 1: 调用生产命令 create_backup 创建备份
    let state = make_app_state(pool.clone());
    backup::create_backup(state, backup_str.clone())
        .await
        .expect("create_backup should succeed");

    assert!(backup_path.exists(), "备份文件应该存在");

    // Step 2: 验证备份包结构
    let file = std::fs::File::open(&backup_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert!(
        archive.by_name("backup.sqlite").is_ok(),
        "备份包中应包含 backup.sqlite"
    );
    let mut manifest_entry = archive.by_name("manifest.json").unwrap();
    let mut manifest_text = String::new();
    use std::io::Read;
    manifest_entry.read_to_string(&mut manifest_text).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest["format_version"], "2.0.0");
    assert!(manifest["checksum"].as_str().unwrap_or_default().len() > 10);

    // Step 3: 创建新的空文件数据库做恢复目标
    let restore_db_path = temp_dir.join(format!("test_restore_target_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&restore_db_path);
    let restore_opts =
        SqliteConnectOptions::from_str(&format!("sqlite:{}", restore_db_path.display()))
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);
    let restore_pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(restore_opts)
        .await
        .unwrap();

    // Step 4: 调用生产命令 restore_backup 执行恢复
    let restore_state = make_app_state(restore_pool.clone());
    backup::restore_backup(restore_state, backup_str.clone())
        .await
        .expect("restore_backup should succeed");

    // Step 5: 验证恢复后数据完整
    let (c_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort WHERE id = ?1")
        .bind(cohort_id)
        .fetch_one(&restore_pool)
        .await
        .unwrap();
    assert_eq!(c_count, 1, "恢复后应有1个届次，实际: {}", c_count);

    let (s_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student WHERE cohort_id = ?1")
        .bind(cohort_id)
        .fetch_one(&restore_pool)
        .await
        .unwrap();
    assert_eq!(s_count, 1, "恢复后应有1个学生，实际: {}", s_count);

    let (h_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM homework WHERE cohort_id = ?1")
        .bind(cohort_id)
        .fetch_one(&restore_pool)
        .await
        .unwrap();
    assert_eq!(h_count, 1, "恢复后应有1个作业，实际: {}", h_count);

    let (hr_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM homework_record hr JOIN homework h ON hr.homework_id = h.id WHERE h.cohort_id = ?1"
    ).bind(cohort_id).fetch_one(&restore_pool).await.unwrap();
    assert_eq!(hr_count, 1, "恢复后应有1个作业记录，实际: {}", hr_count);

    let (a_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM attendance WHERE cohort_id = ?1")
        .bind(cohort_id)
        .fetch_one(&restore_pool)
        .await
        .unwrap();
    assert_eq!(a_count, 1, "恢复后应有1个考勤记录，实际: {}", a_count);

    // 清理
    let _ = std::fs::remove_file(&backup_path);
    let _ = std::fs::remove_file(&restore_db_path);
}

// ==================== 测试: 归档届次不可修改 ====================
#[tokio::test]
async fn test_archived_cohort_rejects_writes() {
    let pool = create_test_db().await;
    let now = now_str();
    let active_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('使用中', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    let archived_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('已归档', '2班', '已归档', 0, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    let archived_err = cohort::check_cohort_readonly(&pool, archived_id.0)
        .await
        .unwrap_err();
    assert!(archived_err.contains("已归档"));

    let state = make_app_state(pool.clone());
    let result = student::create_student(
        state,
        archived_id.0,
        "归档学生".to_string(),
        "ARCH001".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;
    assert!(result.is_err(), "归档届次下创建学生应被拒绝");

    cohort::check_cohort_readonly(&pool, active_id.0)
        .await
        .unwrap();
}

// ==================== 测试: 跨届学生归属校验（真实命令） ====================
#[tokio::test]
async fn test_cross_cohort_student_validated_by_command_pattern() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_a: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('A届', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    let cohort_b: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('B届', '2班', '使用中', 0, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    let student_b: (i64,) = sqlx::query_as(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, 'B学生', 'B001', '正常', 0, ?2, ?2) RETURNING id"
    ).bind(cohort_b.0).bind(&now).fetch_one(&pool).await.unwrap();

    let state = make_app_state(pool.clone());
    let date = today_str();
    let bad_result = attendance::save_attendance(
        state,
        cohort_a.0,
        date.clone(),
        vec![attendance::AttendanceRecord {
            student_id: student_b.0,
            status: "正常".to_string(),
            leave_type: None,
            leave_start_date: None,
            leave_end_date: None,
            reason: None,
            remark: None,
        }],
    )
    .await;
    assert!(bad_result.is_err(), "跨届写入考勤应被拒绝");

    let good_state = make_app_state(pool.clone());
    attendance::save_attendance(
        good_state,
        cohort_b.0,
        date.clone(),
        vec![attendance::AttendanceRecord {
            student_id: student_b.0,
            status: "正常".to_string(),
            leave_type: None,
            leave_start_date: None,
            leave_end_date: None,
            reason: None,
            remark: None,
        }],
    )
    .await
    .unwrap();

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM attendance WHERE cohort_id = ?1 AND student_id = ?2")
            .bind(cohort_b.0)
            .bind(student_b.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);
}

// ==================== 测试: 学生导入列映射（7列模板） ====================
#[tokio::test]
async fn test_student_import_7column_mapping() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    let file_path = write_student_import_excel(
        "mapping",
        &[(
            "Z001",
            "张小明",
            "男",
            "13800001111",
            "张父",
            "13900002222",
            "火箭组",
        )],
    );
    let state = make_app_state(pool.clone());
    let result = student::import_students_excel(state, cohort_id.0, file_path.clone())
        .await
        .unwrap();
    assert_eq!(result["success"], 1);

    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT name, gender, phone, parent_name, parent_phone, group_name FROM student WHERE student_no = 'Z001'"
    ).fetch_one(&pool).await.unwrap();

    assert_eq!(row.0, "张小明");
    assert_eq!(row.1.unwrap(), "男");
    assert_eq!(row.2.unwrap(), "13800001111");
    assert_eq!(row.3.unwrap(), "张父");
    assert_eq!(row.4.unwrap(), "13900002222");
    assert_eq!(row.5.unwrap(), "火箭组");
    let _ = std::fs::remove_file(file_path);
}

// ==================== 测试: 作业统计口径（学生-作业记录维度） ====================
#[tokio::test]
async fn test_homework_stats_by_records() {
    let pool = create_test_db().await;
    let now = now_str();
    let today = today_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    // 2项作业
    let hw1: (i64,) = sqlx::query_as(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, '语文', ?2, ?3, ?3) RETURNING id",
    )
    .bind(cohort_id.0)
    .bind(&today)
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    let hw2: (i64,) = sqlx::query_as(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, '数学', ?2, ?3, ?3) RETURNING id",
    )
    .bind(cohort_id.0)
    .bind(&today)
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    // 3个学生
    for (name, no) in &[("A", "S1"), ("B", "S2"), ("C", "S3")] {
        sqlx::query(
            "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
             VALUES (?1, ?2, ?3, '正常', 0, ?4, ?4)"
        ).bind(cohort_id.0).bind(*name).bind(*no).bind(&now)
        .execute(&pool).await.unwrap();
    }
    let students: Vec<i64> =
        sqlx::query_as::<_, (i64,)>("SELECT id FROM student WHERE cohort_id = ?1 ORDER BY id")
            .bind(cohort_id.0)
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.0)
            .collect();

    // 6条记录: A完成两项, B完成一项, C一项未完成
    for (hw_id, s_idx, status) in &[
        (hw1.0, 0, "已完成"),
        (hw1.0, 1, "已完成"),
        (hw1.0, 2, "未完成"),
        (hw2.0, 0, "已完成"),
        (hw2.0, 1, "未登记"),
        (hw2.0, 2, "未完成"),
    ] {
        sqlx::query(
            "INSERT INTO homework_record (homework_id, student_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
        )
        .bind(*hw_id)
        .bind(students[*s_idx])
        .bind(*status)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    }

    // 按记录维度统计
    let (total, completed, rate): (i64, i64, f64) = sqlx::query_as(
        "SELECT COUNT(hr.id),
            COUNT(CASE WHEN hr.status = '已完成' THEN 1 END),
            CASE WHEN COUNT(hr.id) > 0 THEN CAST(COUNT(CASE WHEN hr.status = '已完成' THEN 1 END) AS REAL) / COUNT(hr.id) ELSE 0 END
         FROM homework_record hr JOIN homework h ON hr.homework_id = h.id
         WHERE h.cohort_id = ?1 AND h.publish_date = ?2 AND h.deleted_at IS NULL"
    ).bind(cohort_id.0).bind(&today).fetch_one(&pool).await.unwrap();

    assert_eq!(total, 6);
    assert_eq!(completed, 3);
    assert!((rate - 0.5).abs() < 0.001, "完成率应为50%");
}

// ==================== 测试: 原子导入 ====================
#[tokio::test]
async fn test_atomic_import_all_or_nothing() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    // 先插入一个学生占位
    sqlx::query(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, '已有', 'EXIST', '正常', 0, ?2, ?2)"
    ).bind(cohort_id.0).bind(&now).execute(&pool).await.unwrap();

    let file_path = write_student_import_excel(
        "atomic",
        &[
            ("N001", "新1", "", "", "", "", ""),
            ("EXIST", "新2", "", "", "", "", ""),
        ],
    );
    let state = make_app_state(pool.clone());
    let result = student::import_students_excel(state, cohort_id.0, file_path.clone())
        .await
        .unwrap();
    assert_eq!(result["success"], 0);
    assert!(
        result["errors"].as_array().unwrap().len() >= 1,
        "重复学号应导致整批失败"
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student WHERE student_no = 'N001'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0, "失败后第一个学生也不应存在");
    let _ = std::fs::remove_file(file_path);
}

// ==================== 测试: 备份校验值比对 + PRAGMA integrity_check ====================
#[tokio::test]
async fn test_backup_checksum_verification_and_integrity() {
    let pool = create_test_file_db("chksum").await;
    let (cohort_id, _) = seed_test_data(&pool).await;
    assert!(verify_data_integrity(&pool, cohort_id).await);

    // Step 1: 创建备份包
    let temp_dir = std::env::temp_dir();
    let backup_path = temp_dir.join(format!("test_chksum_{}.bak", std::process::id()));
    let backup_str = backup_path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&backup_path);
    let state = make_app_state(pool.clone());
    backup::create_backup(state, backup_str.clone())
        .await
        .unwrap();
    assert!(backup_path.exists());

    // Step 2: 篡改备份文件（修改一个字节）
    let original_data = std::fs::read(&backup_path).unwrap();
    let mut tampered = original_data.clone();
    let pivot = tampered.len() / 2;
    tampered[pivot] ^= 0xFF;
    std::fs::write(&backup_path, &tampered).unwrap();

    // Step 3: 还原时应被拒绝
    let restore_db_path = temp_dir.join(format!("test_chksum_restore_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&restore_db_path);
    let restore_opts =
        SqliteConnectOptions::from_str(&format!("sqlite:{}", restore_db_path.display()))
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);
    let restore_pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(restore_opts)
        .await
        .unwrap();
    let restore_state = make_app_state(restore_pool.clone());
    let err = backup::restore_backup(restore_state, backup_str.clone())
        .await
        .unwrap_err();
    assert!(err.contains("校验") || err.contains("损坏") || err.contains("篡改"));

    // 清理
    let _ = std::fs::remove_file(&backup_path);
    let _ = std::fs::remove_file(&restore_db_path);
}

#[tokio::test]
async fn test_export_cohort_contains_summary_workbook() {
    let pool = create_test_file_db("export_cohort").await;
    let (cohort_id, _) = seed_test_data(&pool).await;
    let state = make_app_state(pool.clone());
    let export_path =
        std::env::temp_dir().join(format!("test_cohort_export_{}.zip", unique_suffix()));
    let export_str = export_path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&export_path);

    backup::export_cohort(state, cohort_id, export_str.clone())
        .await
        .expect("export_cohort should succeed");

    assert!(export_path.exists(), "导出 ZIP 应存在");
    let file = std::fs::File::open(&export_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert!(archive.by_name("students.json").is_ok(), "应包含学生 JSON");
    assert!(archive.by_name("homeworks.json").is_ok(), "应包含作业 JSON");
    assert!(
        archive.by_name("export_summary.xlsx").is_ok(),
        "应包含 Excel 摘要"
    );
    let mut meta_entry = archive.by_name("metadata.json").unwrap();
    let mut meta_text = String::new();
    use std::io::Read;
    meta_entry.read_to_string(&mut meta_text).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&meta_text).unwrap();
    assert_eq!(meta["summary"]["student_count"], 1);
    assert_eq!(meta["summary"]["homework_count"], 1);

    let _ = std::fs::remove_file(&export_path);
}

#[tokio::test]
async fn test_pdf_exports_generate_non_empty_files() {
    let pool = create_test_file_db("pdf_exports").await;
    let (cohort_id, student_id) = seed_test_data(&pool).await;
    let exam_date = today_str();
    let now = now_str();
    let subject_id: (i64,) = sqlx::query_as(
        "INSERT INTO subject (name, sort_order, is_active, created_at, updated_at)
         VALUES ('数学', 1, 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    let exam_id: (i64,) = sqlx::query_as(
        "INSERT INTO exam (cohort_id, name, exam_date, created_at, updated_at)
         VALUES (?1, '期中考试', ?2, ?3, ?3) RETURNING id",
    )
    .bind(cohort_id)
    .bind(&exam_date)
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO score (exam_id, subject_id, student_id, score_value, rank_no, created_at, updated_at)
         VALUES (?1, ?2, ?3, 96, 1, ?4, ?4)"
    ).bind(exam_id.0).bind(subject_id.0).bind(student_id).bind(&now).execute(&pool).await.unwrap();

    let state_a = make_app_state(pool.clone());
    let stats_pdf = std::env::temp_dir().join(format!("cohort_stats_{}.pdf", unique_suffix()));
    let stats_pdf_str = stats_pdf.to_string_lossy().to_string();
    stats::export_cohort_statistics_pdf(state_a, cohort_id, stats_pdf_str.clone())
        .await
        .expect("export_cohort_statistics_pdf should succeed");
    let stats_size = std::fs::metadata(&stats_pdf).unwrap().len();
    assert!(stats_size > 1000, "统计 PDF 不应为空");

    let state_b = make_app_state(pool.clone());
    let growth_pdf = std::env::temp_dir().join(format!("growth_{}.pdf", unique_suffix()));
    let growth_pdf_str = growth_pdf.to_string_lossy().to_string();
    stats::export_student_growth_archive_pdf(state_b, student_id, growth_pdf_str.clone())
        .await
        .expect("export_student_growth_archive_pdf should succeed");
    let growth_size = std::fs::metadata(&growth_pdf).unwrap().len();
    assert!(growth_size > 1000, "成长档案 PDF 不应为空");

    let _ = std::fs::remove_file(&stats_pdf);
    let _ = std::fs::remove_file(&growth_pdf);
}

// ==================== 测试: 文件内重复学号检测 ====================
/// 验证：同一个 Excel 文件中出现两行相同学号，应被检测并拒绝
#[tokio::test]
async fn test_detect_duplicate_student_no_in_batch() {
    let pool = create_test_db().await;
    let now = now_str();
    let _cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id",
    )
    .bind(&now)
    .fetch_one(&pool)
    .await
    .unwrap();

    let file_path = write_student_import_excel(
        "duplicate",
        &[
            ("S001", "张三", "", "", "", "", ""),
            ("S002", "李四", "", "", "", "", ""),
            ("S001", "张三副本", "", "", "", "", ""),
            ("S003", "王五", "", "", "", "", ""),
        ],
    );
    let state = make_app_state(pool.clone());
    let preview = student::preview_students_excel(state, _cohort_id.0, file_path.clone())
        .await
        .unwrap();
    let errors = preview["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1, "应检测到 1 个重复学号");
    assert!(errors[0].as_str().unwrap().contains("S001"));
    let _ = std::fs::remove_file(file_path);
}
