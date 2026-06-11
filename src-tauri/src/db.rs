use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqlitePool};
use sqlx::ConnectOptions;
use std::path::PathBuf;
use std::str::FromStr;
use log::info;

const MIGRATIONS: &[&str] = &[
    // 初始版本 v1
    "CREATE TABLE IF NOT EXISTS cohort (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_name TEXT NOT NULL,
        class_name TEXT NOT NULL,
        grade_name TEXT,
        school_name TEXT,
        head_teacher TEXT,
        admission_year INTEGER,
        graduation_year INTEGER,
        semester TEXT,
        status TEXT NOT NULL DEFAULT '使用中',
        is_current INTEGER NOT NULL DEFAULT 0,
        archive_time TEXT,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS student (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_id INTEGER NOT NULL,
        name TEXT NOT NULL,
        student_no TEXT NOT NULL,
        gender TEXT,
        phone TEXT,
        parent_name TEXT,
        parent_phone TEXT,
        address TEXT,
        group_name TEXT,
        status TEXT NOT NULL DEFAULT '正常',
        is_focus INTEGER NOT NULL DEFAULT 0,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        deleted_at TEXT,
        UNIQUE(cohort_id, student_no),
        FOREIGN KEY(cohort_id) REFERENCES cohort(id)
    );",
    "CREATE TABLE IF NOT EXISTS subject (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE,
        sort_order INTEGER DEFAULT 0,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS homework (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_id INTEGER NOT NULL,
        title TEXT NOT NULL,
        subject_id INTEGER,
        subject_name TEXT,
        description TEXT,
        publish_date TEXT NOT NULL,
        deadline TEXT,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        deleted_at TEXT,
        FOREIGN KEY(cohort_id) REFERENCES cohort(id),
        FOREIGN KEY(subject_id) REFERENCES subject(id)
    );",
    "CREATE TABLE IF NOT EXISTS homework_record (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        homework_id INTEGER NOT NULL,
        student_id INTEGER NOT NULL,
        status TEXT NOT NULL DEFAULT '未登记',
        submit_time TEXT,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        UNIQUE(homework_id, student_id),
        FOREIGN KEY(homework_id) REFERENCES homework(id),
        FOREIGN KEY(student_id) REFERENCES student(id)
    );",
    "CREATE TABLE IF NOT EXISTS attendance (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_id INTEGER NOT NULL,
        student_id INTEGER NOT NULL,
        attendance_date TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT '正常',
        reason TEXT,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        UNIQUE(student_id, attendance_date),
        FOREIGN KEY(cohort_id) REFERENCES cohort(id),
        FOREIGN KEY(student_id) REFERENCES student(id)
    );",
    "CREATE TABLE IF NOT EXISTS exam (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_id INTEGER NOT NULL,
        name TEXT NOT NULL,
        exam_type TEXT,
        exam_date TEXT,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        deleted_at TEXT,
        FOREIGN KEY(cohort_id) REFERENCES cohort(id)
    );",
    "CREATE TABLE IF NOT EXISTS score (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        exam_id INTEGER NOT NULL,
        subject_id INTEGER NOT NULL,
        student_id INTEGER NOT NULL,
        score_value REAL,
        rank_no INTEGER,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        UNIQUE(exam_id, subject_id, student_id),
        FOREIGN KEY(exam_id) REFERENCES exam(id),
        FOREIGN KEY(subject_id) REFERENCES subject(id),
        FOREIGN KEY(student_id) REFERENCES student(id)
    );",
    "CREATE TABLE IF NOT EXISTS notice (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_id INTEGER NOT NULL,
        title TEXT NOT NULL,
        content TEXT,
        publish_date TEXT NOT NULL,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        deleted_at TEXT,
        FOREIGN KEY(cohort_id) REFERENCES cohort(id)
    );",
    "CREATE TABLE IF NOT EXISTS duty (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_id INTEGER NOT NULL,
        duty_date TEXT NOT NULL,
        student_id INTEGER,
        group_name TEXT,
        duty_content TEXT,
        status TEXT NOT NULL DEFAULT '未完成',
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY(cohort_id) REFERENCES cohort(id),
        FOREIGN KEY(student_id) REFERENCES student(id)
    );",
    "CREATE TABLE IF NOT EXISTS behavior_record (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort_id INTEGER NOT NULL,
        student_id INTEGER NOT NULL,
        type TEXT NOT NULL,
        title TEXT NOT NULL,
        score INTEGER DEFAULT 0,
        description TEXT,
        record_date TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY(cohort_id) REFERENCES cohort(id),
        FOREIGN KEY(student_id) REFERENCES student(id)
    );",
    "CREATE TABLE IF NOT EXISTS system_config (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        config_key TEXT NOT NULL UNIQUE,
        config_value TEXT,
        description TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );",
    // 索引
    "CREATE INDEX IF NOT EXISTS idx_student_cohort ON student(cohort_id);",
    "CREATE INDEX IF NOT EXISTS idx_student_name ON student(name);",
    "CREATE INDEX IF NOT EXISTS idx_homework_cohort ON homework(cohort_id);",
    "CREATE INDEX IF NOT EXISTS idx_attendance_cohort ON attendance(cohort_id);",
    "CREATE INDEX IF NOT EXISTS idx_attendance_date ON attendance(attendance_date);",
    "CREATE INDEX IF NOT EXISTS idx_exam_cohort ON exam(cohort_id);",
    "CREATE INDEX IF NOT EXISTS idx_score_student ON score(student_id);",
    "CREATE INDEX IF NOT EXISTS idx_behavior_cohort ON behavior_record(cohort_id);",
    "CREATE INDEX IF NOT EXISTS idx_duty_cohort ON duty(cohort_id);",
    "CREATE INDEX IF NOT EXISTS idx_notice_cohort ON notice(cohort_id);",
];

pub async fn init_db(db_path: &PathBuf) -> Result<SqlitePool, sqlx::Error> {
    // 确保父目录存在
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let database_url = format!("sqlite:{}", db_path.display());
    info!("Opening database at: {}", database_url);

    let conn_options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .log_statements(log::LevelFilter::Debug);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(conn_options)
        .await?;

    // 执行迁移
    run_migrations(&pool).await?;

    // 确保至少有一个当前届次
    ensure_current_cohort(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for (i, migration) in MIGRATIONS.iter().enumerate() {
        sqlx::query(migration).execute(pool).await.map_err(|e| {
            log::error!("Migration {} failed: {}", i, e);
            e
        })?;
    }

    // 检查并插入默认科目
    let default_subjects = ["语文", "数学", "英语", "物理", "化学", "生物", "历史", "地理", "政治"];
    for subject in default_subjects {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO subject (name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)"
        )
        .bind(subject)
        .bind(0)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    }

    info!("Database migrations completed successfully");
    Ok(())
}

async fn ensure_current_cohort(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 检查是否有当前届次
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cohort WHERE is_current = 1 AND status = '使用中'")
        .fetch_one(pool)
        .await?;

    if count.0 == 0 {
        // 如果有使用中的届次但没有标记为当前，将最早的一个设为当前
        let result = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM cohort WHERE status = '使用中' ORDER BY id ASC LIMIT 1"
        )
        .fetch_optional(pool)
        .await?;

        if let Some((id,)) = result {
            sqlx::query("UPDATE cohort SET is_current = 1 WHERE id = ?1")
                .bind(id)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}
