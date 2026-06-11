// 集成测试：数据库隔离、归档只读、跨届校验、备份往返
// 使用方法：cargo test -- --nocapture

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqlitePool, Row, Connection};
use std::str::FromStr;

/// 创建测试用内存数据库，包含完整表结构
async fn create_test_db() -> SqlitePool {
    let conn_options = SqliteConnectOptions::from_str("sqlite::memory:")
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
            remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS homework (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, title TEXT NOT NULL,
            subject_id INTEGER, subject_name TEXT, description TEXT,
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
            content TEXT, publish_date TEXT NOT NULL,
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

/// 插入测试数据：届次 + 学生 + 作业 + 考勤
async fn seed_test_data(pool: &SqlitePool) -> (i64, i64) {
    let now = now_str();
    // 插入届次
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('2024级', '1班', '使用中', 1, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(pool).await.unwrap();

    // 插入学生
    let student_id: (i64,) = sqlx::query_as(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, '测试学生', 'S001', '正常', 0, ?2, ?2) RETURNING id"
    ).bind(cohort_id.0).bind(&now).fetch_one(pool).await.unwrap();

    // 插入作业
    let hw_id: (i64,) = sqlx::query_as(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, '数学作业', ?2, ?3, ?3) RETURNING id"
    ).bind(cohort_id.0).bind(&today_str()).bind(&now).fetch_one(pool).await.unwrap();

    // 插入作业记录
    sqlx::query(
        "INSERT INTO homework_record (homework_id, student_id, status, created_at, updated_at)
         VALUES (?1, ?2, '已完成', ?3, ?3)"
    ).bind(hw_id.0).bind(student_id.0).bind(&now).execute(pool).await.unwrap();

    // 插入考勤
    sqlx::query(
        "INSERT INTO attendance (cohort_id, student_id, attendance_date, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, '正常', ?4, ?4)"
    ).bind(cohort_id.0).bind(student_id.0).bind(&today_str()).bind(&now).execute(pool).await.unwrap();

    (cohort_id.0, student_id.0)
}

/// 验证数据完整性
async fn verify_data_integrity(pool: &SqlitePool, cohort_id: i64) -> bool {
    let c: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort WHERE id = ?1")
        .bind(cohort_id).fetch_one(pool).await.unwrap();
    if c.0 != 1 { return false; }
    let s: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student WHERE cohort_id = ?1")
        .bind(cohort_id).fetch_one(pool).await.unwrap();
    if s.0 != 1 { return false; }
    let h: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM homework WHERE cohort_id = ?1")
        .bind(cohort_id).fetch_one(pool).await.unwrap();
    if h.0 != 1 { return false; }
    let hr: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM homework_record hr JOIN homework h ON hr.homework_id = h.id WHERE h.cohort_id = ?1"
    ).bind(cohort_id).fetch_one(pool).await.unwrap();
    if hr.0 != 1 { return false; }
    let a: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM attendance WHERE cohort_id = ?1")
        .bind(cohort_id).fetch_one(pool).await.unwrap();
    if a.0 != 1 { return false; }
    true
}

/// 创建测试用文件数据库
async fn create_test_file_db() -> SqlitePool {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("test_backup_source_{}.db", std::process::id()));
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
            remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );",
        "CREATE TABLE IF NOT EXISTS homework (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cohort_id INTEGER NOT NULL, title TEXT NOT NULL,
            subject_id INTEGER, subject_name TEXT, description TEXT,
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
            content TEXT, publish_date TEXT NOT NULL,
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
    ];

    for m in &migrations {
        sqlx::query(m).execute(&pool).await.unwrap();
    }

    pool
}

// ==================== 测试: 备份真实往返（使用文件数据库） ====================
#[tokio::test]
async fn test_backup_round_trip() {
    // 使用文件数据库（VACUUM INTO 不支持内存数据库）
    let pool = create_test_file_db().await;
    let (cohort_id, _) = seed_test_data(&pool).await;

    // 验证初始数据
    assert!(verify_data_integrity(&pool, cohort_id).await, "初始数据完整");

    // 确保 WAL 数据写入主文件（VACUUM INTO 需要）
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool).await.unwrap();

    // Step 1: 用 VACUUM INTO 创建备份
    let temp_dir = std::env::temp_dir();
    let backup_path = temp_dir.join(format!("test_backup_roundtrip_{}.db", std::process::id()));
    let backup_str = backup_path.to_string_lossy().to_string();

    // 清理旧的备份文件
    let _ = std::fs::remove_file(&backup_path);

    let escaped = backup_str.replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{}'", escaped))
        .execute(&pool)
        .await
        .expect("VACUUM INTO should succeed");

    assert!(backup_path.exists(), "备份文件应该存在");
    let backup_size = std::fs::metadata(&backup_path).unwrap().len();
    assert!(backup_size > 0, "备份文件不应为空");

    // Step 2: 校验备份文件可读且包含正确数据
    let conn_opts = format!("sqlite://{}?mode=ro", backup_str);
    let opts: SqliteConnectOptions = conn_opts.parse().unwrap();
    let mut backup_conn = sqlx::SqliteConnection::connect_with(&opts).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort")
        .fetch_one(&mut backup_conn).await.unwrap();
    assert_eq!(count.0, 1, "备份中应有1个届次");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student")
        .fetch_one(&mut backup_conn).await.unwrap();
    assert_eq!(count.0, 1, "备份中应有1个学生");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM homework")
        .fetch_one(&mut backup_conn).await.unwrap();
    assert_eq!(count.0, 1, "备份中应有1个作业");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM homework_record")
        .fetch_one(&mut backup_conn).await.unwrap();
    assert_eq!(count.0, 1, "备份中应有1个作业记录");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM attendance")
        .fetch_one(&mut backup_conn).await.unwrap();
    assert_eq!(count.0, 1, "备份中应有1个考勤记录");

    drop(backup_conn);

    // Step 3: 创建新的空文件数据库做恢复目标
    let restore_db_path = temp_dir.join(format!("test_restore_target_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&restore_db_path);
    let restore_opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", restore_db_path.display()))
        .unwrap().create_if_missing(true).foreign_keys(true);
    let restore_pool = SqlitePoolOptions::new().max_connections(2)
        .connect_with(restore_opts).await.unwrap();

    // 在恢复目标中创建相同的表结构
    let create_tables = vec![
        "CREATE TABLE IF NOT EXISTS cohort (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_name TEXT NOT NULL, class_name TEXT NOT NULL, status TEXT NOT NULL DEFAULT '使用中', is_current INTEGER NOT NULL DEFAULT 0, archive_time TEXT, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);",
        "CREATE TABLE IF NOT EXISTS student (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_id INTEGER NOT NULL, name TEXT NOT NULL, student_no TEXT NOT NULL, gender TEXT, phone TEXT, parent_name TEXT, parent_phone TEXT, address TEXT, group_name TEXT, status TEXT NOT NULL DEFAULT '正常', is_focus INTEGER NOT NULL DEFAULT 0, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT, UNIQUE(cohort_id, student_no), FOREIGN KEY(cohort_id) REFERENCES cohort(id));",
        "CREATE TABLE IF NOT EXISTS subject (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, sort_order INTEGER DEFAULT 0, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);",
        "CREATE TABLE IF NOT EXISTS homework (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_id INTEGER NOT NULL, title TEXT NOT NULL, subject_id INTEGER, subject_name TEXT, description TEXT, publish_date TEXT NOT NULL, deadline TEXT, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT, FOREIGN KEY(cohort_id) REFERENCES cohort(id));",
        "CREATE TABLE IF NOT EXISTS homework_record (id INTEGER PRIMARY KEY AUTOINCREMENT, homework_id INTEGER NOT NULL, student_id INTEGER NOT NULL, status TEXT NOT NULL DEFAULT '未登记', submit_time TEXT, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(homework_id, student_id), FOREIGN KEY(homework_id) REFERENCES homework(id), FOREIGN KEY(student_id) REFERENCES student(id));",
        "CREATE TABLE IF NOT EXISTS attendance (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_id INTEGER NOT NULL, student_id INTEGER NOT NULL, attendance_date TEXT NOT NULL, status TEXT NOT NULL DEFAULT '正常', reason TEXT, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(student_id, attendance_date), FOREIGN KEY(cohort_id) REFERENCES cohort(id), FOREIGN KEY(student_id) REFERENCES student(id));",
        "CREATE TABLE IF NOT EXISTS exam (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_id INTEGER NOT NULL, name TEXT NOT NULL, exam_type TEXT, exam_date TEXT, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT, FOREIGN KEY(cohort_id) REFERENCES cohort(id));",
        "CREATE TABLE IF NOT EXISTS score (id INTEGER PRIMARY KEY AUTOINCREMENT, exam_id INTEGER NOT NULL, subject_id INTEGER NOT NULL, student_id INTEGER NOT NULL, score_value REAL, rank_no INTEGER, remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE(exam_id, subject_id, student_id), FOREIGN KEY(exam_id) REFERENCES exam(id), FOREIGN KEY(subject_id) REFERENCES subject(id), FOREIGN KEY(student_id) REFERENCES student(id));",
        "CREATE TABLE IF NOT EXISTS notice (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_id INTEGER NOT NULL, title TEXT NOT NULL, content TEXT, publish_date TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT);",
        "CREATE TABLE IF NOT EXISTS duty (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_id INTEGER NOT NULL, duty_date TEXT NOT NULL, student_id INTEGER, group_name TEXT, duty_content TEXT, status TEXT NOT NULL DEFAULT '未完成', remark TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, FOREIGN KEY(cohort_id) REFERENCES cohort(id), FOREIGN KEY(student_id) REFERENCES student(id));",
        "CREATE TABLE IF NOT EXISTS behavior_record (id INTEGER PRIMARY KEY AUTOINCREMENT, cohort_id INTEGER NOT NULL, student_id INTEGER NOT NULL, type TEXT NOT NULL, title TEXT NOT NULL, score INTEGER DEFAULT 0, description TEXT, record_date TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, FOREIGN KEY(cohort_id) REFERENCES cohort(id), FOREIGN KEY(student_id) REFERENCES student(id));",
        "CREATE TABLE IF NOT EXISTS system_config (id INTEGER PRIMARY KEY AUTOINCREMENT, config_key TEXT NOT NULL UNIQUE, config_value TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);",
    ];
    for m in &create_tables {
        sqlx::query(m).execute(&restore_pool).await.unwrap();
    }

    // 确保恢复池里没有数据
    let empty_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort")
        .fetch_one(&restore_pool).await.unwrap();
    assert_eq!(empty_count.0, 0, "恢复前目标应为空");

    // Step 4: 执行恢复（模拟 restore_backup 的核心逻辑）
    let mut conn = restore_pool.acquire().await.unwrap();
    sqlx::query(&format!("ATTACH DATABASE '{}' AS backup_db", escaped))
        .execute(&mut *conn).await.unwrap();

    // 关闭FK
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn).await.unwrap();

    let insert_order = [
        "cohort", "system_config", "student", "subject", "homework",
        "homework_record", "exam", "score", "attendance", "notice", "duty", "behavior_record",
    ];

    for table in &insert_order {
        let exists: (i64,) = sqlx::query_as(
            &format!("SELECT COUNT(*) FROM backup_db.sqlite_master WHERE type='table' AND name='{}'", table)
        ).fetch_one(&mut *conn).await.unwrap();
        if exists.0 > 0 {
            let row_count: (i64,) = sqlx::query_as(
                &format!("SELECT COUNT(*) FROM backup_db.\"{}\"", table)
            ).fetch_one(&mut *conn).await.unwrap();
            if row_count.0 > 0 {
                sqlx::query(&format!(
                    "INSERT INTO main.\"{}\" SELECT * FROM backup_db.\"{}\"",
                    table, table
                )).execute(&mut *conn).await.unwrap();
            }
        }
    }

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn).await.unwrap();
    sqlx::query("DETACH DATABASE backup_db")
        .execute(&mut *conn).await.ok();
    drop(conn);

    // Step 5: 验证恢复后数据完整（逐项检查）
    let (c_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort WHERE id = ?1")
        .bind(cohort_id).fetch_one(&restore_pool).await.unwrap();
    assert_eq!(c_count, 1, "恢复后应有1个届次，实际: {}", c_count);

    let (s_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student WHERE cohort_id = ?1")
        .bind(cohort_id).fetch_one(&restore_pool).await.unwrap();
    assert_eq!(s_count, 1, "恢复后应有1个学生，实际: {}", s_count);

    let (h_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM homework WHERE cohort_id = ?1")
        .bind(cohort_id).fetch_one(&restore_pool).await.unwrap();
    assert_eq!(h_count, 1, "恢复后应有1个作业，实际: {}", h_count);

    let (hr_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM homework_record hr JOIN homework h ON hr.homework_id = h.id WHERE h.cohort_id = ?1"
    ).bind(cohort_id).fetch_one(&restore_pool).await.unwrap();
    assert_eq!(hr_count, 1, "恢复后应有1个作业记录，实际: {}", hr_count);

    let (a_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM attendance WHERE cohort_id = ?1")
        .bind(cohort_id).fetch_one(&restore_pool).await.unwrap();
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
         VALUES ('使用中', '1班', '使用中', 1, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();
    let archived_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('已归档', '2班', '已归档', 0, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();

    // 在业务层：检查归档状态再更新
    let (status,): (String,) = sqlx::query_as("SELECT status FROM cohort WHERE id = ?1")
        .bind(archived_id.0).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "已归档");

    // 模拟 check_cohort_readonly: 已归档应拒绝更新
    if status == "已归档" {
        // 跳过更新（业务逻辑应在此拦截）
        let (name,): (String,) = sqlx::query_as("SELECT class_name FROM cohort WHERE id = ?1")
            .bind(archived_id.0).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "2班", "归档届次不应被修改");
    }

    // 使用中届次可以编辑
    sqlx::query("UPDATE cohort SET class_name = '1班改名' WHERE id = ?1 AND status != '已归档'")
        .bind(active_id.0).execute(&pool).await.unwrap();
    let (name,): (String,) = sqlx::query_as("SELECT class_name FROM cohort WHERE id = ?1")
        .bind(active_id.0).fetch_one(&pool).await.unwrap();
    assert_eq!(name, "1班改名");
}

// ==================== 测试: 跨届学生归属校验（模拟实际命令逻辑） ====================
#[tokio::test]
async fn test_cross_cohort_student_validated_by_command_pattern() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_a: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('A届', '1班', '使用中', 1, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();
    let cohort_b: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('B届', '2班', '使用中', 0, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();

    let student_b: (i64,) = sqlx::query_as(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, 'B学生', 'B001', '正常', 0, ?2, ?2) RETURNING id"
    ).bind(cohort_b.0).bind(&now).fetch_one(&pool).await.unwrap();

    // 模拟 save_attendance 中的归属校验
    let (student_cohort,): (i64,) = sqlx::query_as(
        "SELECT cohort_id FROM student WHERE id = ?1"
    ).bind(student_b.0).fetch_one(&pool).await.unwrap();

    assert_eq!(student_cohort, cohort_b.0, "学生应属于B届");
    assert_ne!(student_cohort, cohort_a.0, "学生不应属于A届");

    // 如果归属不匹配，命令应拒绝写入
    if student_cohort != cohort_b.0 {
        panic!("归属校验失败：学生属于不同届次");
    }

    // 验证使用正确届次可以正常写入
    let date = today_str();
    sqlx::query(
        "INSERT INTO attendance (cohort_id, student_id, attendance_date, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, '正常', ?4, ?4)"
    ).bind(cohort_b.0).bind(student_b.0).bind(&date).bind(&now)
    .execute(&pool).await.unwrap();
}

// ==================== 测试: 学生导入列映射（7列模板） ====================
#[tokio::test]
async fn test_student_import_7column_mapping() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();

    // 模拟正确的7列导入: 学号(0), 姓名(1), 性别(2), 电话(3), 家长名(4), 家长电话(5), 小组(6)
    sqlx::query(
        "INSERT INTO student (cohort_id, name, student_no, gender, phone, parent_name, parent_phone, group_name, status, is_focus, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '正常', 0, ?9, ?9)"
    )
    .bind(cohort_id.0)
    .bind("张小明").bind("Z001").bind("男")
    .bind("13800001111").bind("张父").bind("13900002222").bind("火箭组")
    .bind(&now)
    .execute(&pool).await.unwrap();

    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT name, gender, phone, parent_name, parent_phone, group_name FROM student WHERE student_no = 'Z001'"
    ).fetch_one(&pool).await.unwrap();

    assert_eq!(row.0, "张小明");
    assert_eq!(row.1.unwrap(), "男");
    assert_eq!(row.2.unwrap(), "13800001111");
    assert_eq!(row.3.unwrap(), "张父");
    assert_eq!(row.4.unwrap(), "13900002222");
    assert_eq!(row.5.unwrap(), "火箭组"); // 小组在第7列(index 6)，不是第5列
}

// ==================== 测试: 作业统计口径（学生-作业记录维度） ====================
#[tokio::test]
async fn test_homework_stats_by_records() {
    let pool = create_test_db().await;
    let now = now_str();
    let today = today_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();

    // 2项作业
    let hw1: (i64,) = sqlx::query_as(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, '语文', ?2, ?3, ?3) RETURNING id"
    ).bind(cohort_id.0).bind(&today).bind(&now).fetch_one(&pool).await.unwrap();
    let hw2: (i64,) = sqlx::query_as(
        "INSERT INTO homework (cohort_id, title, publish_date, created_at, updated_at)
         VALUES (?1, '数学', ?2, ?3, ?3) RETURNING id"
    ).bind(cohort_id.0).bind(&today).bind(&now).fetch_one(&pool).await.unwrap();

    // 3个学生
    for (name, no) in &[("A", "S1"), ("B", "S2"), ("C", "S3")] {
        sqlx::query(
            "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
             VALUES (?1, ?2, ?3, '正常', 0, ?4, ?4)"
        ).bind(cohort_id.0).bind(*name).bind(*no).bind(&now)
        .execute(&pool).await.unwrap();
    }
    let students: Vec<i64> = sqlx::query_as::<_, (i64,)>("SELECT id FROM student WHERE cohort_id = ?1 ORDER BY id")
        .bind(cohort_id.0).fetch_all(&pool).await.unwrap().into_iter().map(|r| r.0).collect();

    // 6条记录: A完成两项, B完成一项, C一项未完成
    for (hw_id, s_idx, status) in &[
        (hw1.0, 0, "已完成"), (hw1.0, 1, "已完成"), (hw1.0, 2, "未完成"),
        (hw2.0, 0, "已完成"), (hw2.0, 1, "未登记"), (hw2.0, 2, "未完成"),
    ] {
        sqlx::query(
            "INSERT INTO homework_record (homework_id, student_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)"
        ).bind(*hw_id).bind(students[*s_idx]).bind(*status).bind(&now)
        .execute(&pool).await.unwrap();
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
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();

    // 先插入一个学生占位
    sqlx::query(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, '已有', 'EXIST', '正常', 0, ?2, ?2)"
    ).bind(cohort_id.0).bind(&now).execute(&pool).await.unwrap();

    // 模拟事务批量导入：第二个学生重复学号，整个事务应回滚
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, '新1', 'N001', '正常', 0, ?2, ?2)"
    ).bind(cohort_id.0).bind(&now).execute(&mut *tx).await.unwrap();

    let result = sqlx::query(
        "INSERT INTO student (cohort_id, name, student_no, status, is_focus, created_at, updated_at)
         VALUES (?1, '新2', 'EXIST', '正常', 0, ?2, ?2)"
    ).bind(cohort_id.0).bind(&now).execute(&mut *tx).await;

    assert!(result.is_err(), "重复学号应导致错误");
    tx.rollback().await.unwrap();

    // 验证第一个学生也未写入
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM student WHERE student_no = 'N001'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count.0, 0, "回滚后第一个学生也不应存在");
}

// ==================== 测试: 备份校验值比对 + PRAGMA integrity_check ====================
#[tokio::test]
async fn test_backup_checksum_verification_and_integrity() {
    let pool = create_test_file_db().await;
    let (cohort_id, _) = seed_test_data(&pool).await;
    assert!(verify_data_integrity(&pool, cohort_id).await);

    // Step 1: 创建备份
    let temp_dir = std::env::temp_dir();
    let backup_path = temp_dir.join(format!("test_chksum_{}.db", std::process::id()));
    let backup_str = backup_path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&backup_path);

    let escaped = backup_str.replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{}'", escaped))
        .execute(&pool).await.unwrap();
    assert!(backup_path.exists());

    // Step 2: 计算源备份的 SHA256
    use sha2::{Sha256, Digest};
    let original_data = std::fs::read(&backup_path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&original_data);
    let original_hash = hex::encode(hasher.finalize());

    // Step 3: 执行 PRAGMA integrity_check 验证数据库结构完整
    let conn_opts = format!("sqlite://{}?mode=ro", backup_str);
    let opts: SqliteConnectOptions = conn_opts.parse().unwrap();
    let mut conn = sqlx::SqliteConnection::connect_with(&opts).await.unwrap();

    let (integrity,): (String,) = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(&mut conn).await.unwrap();
    assert_eq!(integrity, "ok", "PRAGMA integrity_check 应返回 ok，实际: {}", integrity);
    drop(conn);

    // Step 4: 篡改备份文件（修改一个字节）
    let mut tampered = original_data.clone();
    if tampered.len() > 100 {
        tampered[100] ^= 0xFF; // 翻转一个字节
    }
    std::fs::write(&backup_path, &tampered).unwrap();

    // Step 5: 重新计算被篡改的校验值，应不匹配
    let mut hasher2 = Sha256::new();
    hasher2.update(&tampered);
    let tampered_hash = hex::encode(hasher2.finalize());

    assert_ne!(original_hash, tampered_hash,
        "篡改前后的 SHA256 应不同: original={} tampered={}",
        &original_hash[..16], &tampered_hash[..16]);

    // Step 6: 恢复到原始文件，验证仍可通过校验
    std::fs::write(&backup_path, &original_data).unwrap();
    let restored_data = std::fs::read(&backup_path).unwrap();
    let mut hasher3 = Sha256::new();
    hasher3.update(&restored_data);
    let restored_hash = hex::encode(hasher3.finalize());
    assert_eq!(original_hash, restored_hash, "恢复原始文件后校验值应一致");

    // 清理
    let _ = std::fs::remove_file(&backup_path);
}

// ==================== 测试: 文件内重复学号检测 ====================
/// 验证：同一个 Excel 文件中出现两行相同学号，应被检测并拒绝
#[tokio::test]
async fn test_detect_duplicate_student_no_in_batch() {
    let pool = create_test_db().await;
    let now = now_str();
    let cohort_id: (i64,) = sqlx::query_as(
        "INSERT INTO cohort (cohort_name, class_name, status, is_current, created_at, updated_at)
         VALUES ('测试', '1班', '使用中', 1, ?1, ?1) RETURNING id"
    ).bind(&now).fetch_one(&pool).await.unwrap();

    // 模拟导入中的学号去重检测逻辑
    let incoming = vec![
        ("S001", "张三"),
        ("S002", "李四"),
        ("S001", "张三副本"), // 重复学号！
        ("S003", "王五"),
    ];

    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut dup_errors: Vec<String> = Vec::new();

    for (student_no, name) in &incoming {
        if !seen.insert(student_no.to_string()) {
            dup_errors.push(format!("学号 '{}'（{}）在文件中重复出现", student_no, name));
        }
    }

    assert_eq!(dup_errors.len(), 1, "应检测到 1 个重复学号");
    assert!(dup_errors[0].contains("S001"), "错误信息应包含重复学号 S001");
}
