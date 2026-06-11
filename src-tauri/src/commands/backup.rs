use chrono::Local;
use sqlx::Row;
use sqlx::Connection;
use tauri::State;
use std::path::Path;
use sha2::{Sha256, Digest};
use std::io::Write;

use crate::AppState;

/// 备份文件中的元数据表名
const BACKUP_META_TABLE: &str = "_backup_meta";
/// 校验值文件后缀（独立于备份文件，避免自指问题）
const CHECKSUM_EXTENSION: &str = ".sha256";

/// 计算整个文件的 SHA256
async fn compute_file_sha256(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    let data = std::fs::read(path)
        .map_err(|e| format!("无法读取文件用于计算校验值: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hex::encode(hasher.finalize());
    log::info!("File SHA256 ({}): {}", file_path, hash);
    Ok(hash)
}

/// 将校验值写入独立文件（避免自指：校验值本身不能是备份文件的一部分）
fn write_checksum_file(db_path: &str, checksum: &str) -> Result<(), String> {
    let checksum_path = format!("{}{}", db_path, CHECKSUM_EXTENSION);
    let mut file = std::fs::File::create(&checksum_path)
        .map_err(|e| format!("无法创建校验值文件: {}", e))?;
    file.write_all(checksum.as_bytes())
        .map_err(|e| format!("写入校验值文件失败: {}", e))?;
    log::info!("Checksum written to: {}", checksum_path);
    Ok(())
}

/// 读取独立校验值文件并与当前文件对比
async fn verify_checksum_file(db_path: &str) -> Result<String, String> {
    let checksum_path = format!("{}{}", db_path, CHECKSUM_EXTENSION);
    let path = Path::new(&checksum_path);
    if !path.exists() {
        log::warn!("校验值文件不存在: {}（可能是旧版本备份），跳过校验比对", checksum_path);
        return Ok(String::new());
    }
    let stored = std::fs::read_to_string(path)
        .map_err(|e| format!("读取校验值文件失败: {}", e))?
        .trim()
        .to_string();
    if stored.is_empty() {
        log::warn!("校验值文件为空，跳过校验比对");
        return Ok(String::new());
    }
    // 重新计算当前文件的 SHA256
    let current = compute_file_sha256(db_path).await?;
    if stored != current {
        return Err(format!(
            "备份文件校验失败：当前 SHA256 ({}) 与校验文件 ({}) 不匹配，文件可能已损坏或被篡改",
            &current[..16], &stored[..16]
        ));
    }
    log::info!("Checksum verified: {} (matches .sha256 file)", &stored[..16]);
    Ok(current)
}

/// 清理校验值文件
fn remove_checksum_file(db_path: &str) {
    let checksum_path = format!("{}{}", db_path, CHECKSUM_EXTENSION);
    let _ = std::fs::remove_file(&checksum_path);
}

/// 校验备份文件是否为有效的 SQLite 数据库，并检查基本结构
/// 返回当前文件的 SHA256 值
async fn validate_backup_file(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err("备份文件不存在".to_string());
    }
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("无法读取备份文件: {}", e))?;
    if metadata.len() == 0 {
        return Err("备份文件为空".to_string());
    }

    // 校验是否为有效的 SQLite 数据库文件
    let header = std::fs::read(path)
        .map_err(|e| format!("无法读取备份文件: {}", e))?;
    if header.len() < 16 || &header[0..16] != b"SQLite format 3\0" {
        return Err("备份文件不是有效的 SQLite 数据库".to_string());
    }

    // 🔑 通过独立 .sha256 文件验证完整性（避免自指：校验值存储在备份文件外部）
    let checksum = verify_checksum_file(file_path).await?;
    if !checksum.is_empty() {
        log::info!("Checksum verified: {} (from .sha256 file)", &checksum[..16]);
    }

    // 计算当前文件 SHA256 用于返回
    let hash = compute_file_sha256(file_path).await?;
    log::info!("Backup file SHA256: {}", hash);

    // 校验备份元数据
    let conn_opts = format!("sqlite://{}?mode=ro", file_path);
    let opts: sqlx::sqlite::SqliteConnectOptions = conn_opts.parse()
        .map_err(|e| format!("无法解析备份数据库连接: {}", e))?;
    let mut conn = sqlx::SqliteConnection::connect_with(&opts)
        .await
        .map_err(|e| format!("无法连接备份数据库进行校验: {}", e))?;

    // 检查是否有_备份元数据
    let meta_exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1"
    )
    .bind(BACKUP_META_TABLE)
    .fetch_one(&mut conn)
    .await
    .map_err(|e| format!("校验元数据失败: {}", e))?;

    if meta_exists.0 == 0 {
        log::warn!("备份文件缺少元数据表（可能是旧版本备份），跳过元数据校验");
        return Ok(hash);
    }

    // 读取元数据（版本、时间、表数量）
    let meta: (String, String, i64) = sqlx::query_as(
        "SELECT backup_version, backup_time, table_count FROM _backup_meta LIMIT 1"
    )
    .fetch_one(&mut conn)
    .await
    .map_err(|e| format!("读取备份元数据失败: {}", e))?;

    log::info!(
        "Backup version: {}, time: {}, tables: {}",
        meta.0, meta.1, meta.2
    );

    // 🔑 执行 PRAGMA integrity_check 验证数据库内部结构完整性
    let integrity: (String,) = sqlx::query_as("PRAGMA integrity_check")
        .fetch_one(&mut conn)
        .await
        .map_err(|e| format!("执行完整性检查失败: {}", e))?;

    if integrity.0 != "ok" {
        return Err(format!("备份文件数据库完整性检查失败: {}", integrity.0));
    }
    log::info!("PRAGMA integrity_check: ok");

    // 检查必需的11张业务表是否都存在
    let required_tables = [
        "cohort", "student", "subject", "homework", "homework_record",
        "attendance", "exam", "score", "notice", "duty", "behavior_record",
        "system_config",
    ];

    for table in &required_tables {
        let exists: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1"
        )
        .bind(table)
        .fetch_one(&mut conn)
        .await
        .map_err(|e| format!("校验表 {} 是否存在时出错: {}", table, e))?;

        if exists.0 == 0 {
            return Err(format!("备份文件缺少必需的表: {}", table));
        }
    }

    // 检查每张表是否有数据
    let mut table_row_counts: Vec<(String, i64)> = Vec::new();
    for table in &required_tables {
        let count: (i64,) = sqlx::query_as(
            &format!("SELECT COUNT(*) FROM \"{}\"", table)
        )
        .fetch_one(&mut conn)
        .await
        .map_err(|e| format!("统计表 {} 行数失败: {}", table, e))?;
        table_row_counts.push((table.to_string(), count.0));
    }

    let total_rows: i64 = table_row_counts.iter().map(|(_, c)| c).sum();
    log::info!(
        "Backup validation passed. Total rows: {}, Tables: {:?}",
        total_rows,
        table_row_counts.iter().filter(|(_, c)| *c > 0).map(|(t, c)| format!("{}={}", t, c)).collect::<Vec<_>>()
    );

    Ok(hash)
}

/// 获取正确的表删除顺序（子表先删，避免外键约束冲突）
fn get_delete_order() -> &'static [&'static str] {
    &[
        "homework_record",
        "score",
        "attendance",
        "homework",
        "exam",
        "notice",
        "duty",
        "behavior_record",
        "student",
        "subject",
        "system_config",
        "cohort",
    ]
}

/// 获取正确的表导入顺序（父表先导，必须与 delete_order 反向对应）
fn get_insert_order() -> &'static [&'static str] {
    &[
        "cohort",
        "system_config",
        "student",
        "subject",
        "homework",
        "homework_record",
        "exam",
        "score",
        "attendance",
        "notice",
        "duty",
        "behavior_record",
    ]
}

/// 在备份文件中写入元数据（备份版本、时间、表数量）
/// 校验值不写入备份文件本身，而是写入独立的 .sha256 文件，避免自指问题
async fn write_backup_meta(pool: &sqlx::SqlitePool, file_path: &str) -> Result<(), String> {
    let conn_opts = format!("sqlite://{}", file_path);
    let opts: sqlx::sqlite::SqliteConnectOptions = conn_opts.parse()
        .map_err(|e| format!("无法连接备份文件写入元数据: {}", e))?;
    let mut conn = sqlx::SqliteConnection::connect_with(&opts)
        .await
        .map_err(|e| format!("连接备份文件失败: {}", e))?;

    // 统计主数据库表数量
    let table_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_backup%'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("统计表数量失败: {}", e))?;

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {} (backup_version TEXT, backup_time TEXT, table_count INTEGER)",
        BACKUP_META_TABLE
    ))
    .execute(&mut conn)
    .await
    .map_err(|e| format!("创建元数据表失败: {}", e))?;

    sqlx::query(&format!(
        "INSERT INTO {} (backup_version, backup_time, table_count) VALUES (?1, ?2, ?3)",
        BACKUP_META_TABLE
    ))
    .bind("1.0.0")
    .bind(&now)
    .bind(table_count.0)
    .execute(&mut conn)
    .await
    .map_err(|e| format!("写入备份元数据失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn create_backup(state: State<'_, AppState>, file_path: String) -> Result<(), String> {
    let pool = &state.db;

    // Step 1: 使用 VACUUM INTO 创建备份文件（仅含业务数据）
    let escaped_path = file_path.replace('\'', "''");
    let backup_sql = format!("VACUUM INTO '{}'", escaped_path);
    sqlx::query(&backup_sql)
        .execute(pool)
        .await
        .map_err(|e| format!("备份失败: {}", e))?;

    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("备份文件未生成".to_string());
    }
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("无法读取备份文件: {}", e))?;
    if metadata.len() == 0 {
        return Err("备份文件为空".to_string());
    }

    // Step 2: 写入元数据（版本/时间/表数量，不含校验值）
    write_backup_meta(pool, &file_path).await?;

    // Step 3: 对整个文件（含 _backup_meta 表）计算 SHA256
    let checksum = compute_file_sha256(&file_path).await?;

    // Step 4: 将校验值写入独立文件 {file_path}.sha256
    //        校验值不在备份文件内部 → 写入校验值不会改备份文件 → 没有自指问题
    write_checksum_file(&file_path, &checksum)?;

    log::info!("Backup created successfully at: {} (checksum: {})", file_path, checksum);
    Ok(())
}

#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, file_path: String) -> Result<(), String> {
    let pool = &state.db;

    // 1. 校验备份文件完整性（含结构校验）
    let checksum = validate_backup_file(&file_path).await?;
    log::info!("Backup validated: checksum={}", checksum);

    // 2. 自动备份当前数据库到临时目录
    let auto_backup_path = format!(
        "{}/class_copilot_auto_backup_before_restore_{}.db",
        std::env::temp_dir().display(),
        Local::now().format("%Y%m%d_%H%M%S")
    );
    let escaped_auto = auto_backup_path.replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{}'", escaped_auto))
        .execute(pool)
        .await
        .map_err(|e| format!("自动备份当前数据库失败: {}", e))?;

    log::info!("Auto backup saved to: {}", auto_backup_path);

    // 3. 使用单一专用连接执行恢复（确保 ATTACH 和事务在同一连接上）
    let mut conn = pool.acquire()
        .await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let escaped_path = file_path.replace('\'', "''");

    // 在事务外执行 ATTACH（SQLite 要求 ATTACH 不能在事务内）
    sqlx::query(&format!(
        "ATTACH DATABASE '{}' AS backup_db",
        escaped_path
    ))
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("附加备份数据库失败: {}", e))?;

    // 4. 在事务内执行删除和导入
    let result: Result<(), String> = async {
        // 预校验：检查备份数据库表结构与主数据库一致
        let expected_tables = get_insert_order();
        for table in expected_tables {
            let exists: (i64,) = sqlx::query_as(
                &format!("SELECT COUNT(*) FROM backup_db.sqlite_master WHERE type='table' AND name='{}'", table)
            )
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| format!("检查备份表 {} 失败: {}", table, e))?;
            if exists.0 == 0 {
                return Err(format!("备份文件缺少必需的表: {}", table));
            }
        }

        // 统计恢复前行数（用于恢复后校验）
        let mut backup_counts: Vec<(String, i64)> = Vec::new();
        for table in expected_tables {
            let count: (i64,) = sqlx::query_as(
                &format!("SELECT COUNT(*) FROM backup_db.\"{}\"", table)
            )
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| format!("统计备份表 {} 行数失败: {}", table, e))?;
            backup_counts.push((table.to_string(), count.0));
        }

        let mut tx = conn.begin().await.map_err(|e| e.to_string())?;

        // 关闭外键约束以便按正确顺序删除
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        // 按反向依赖顺序删除所有数据
        for table in get_delete_order() {
            sqlx::query(&format!("DELETE FROM {}", table))
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("清空表 {} 失败: {}", table, e))?;
        }

        // 重新开启外键约束
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        // 按正向依赖顺序导入数据
        for table in expected_tables {
            let import_sql = format!(
                "INSERT INTO main.\"{}\" SELECT * FROM backup_db.\"{}\"",
                table, table
            );
            sqlx::query(&import_sql)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("导入表 {} 失败: {}", table, e))?;
        }

        // 恢复后校验：检查导入行数与备份一致
        for (table, expected_count) in &backup_counts {
            let count: (i64,) = sqlx::query_as(
                &format!("SELECT COUNT(*) FROM \"{}\"", table)
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("校验导入表 {} 行数失败: {}", table, e))?;
            if count.0 != *expected_count {
                return Err(format!(
                    "表 {} 导入行数不一致: 期望 {}, 实际 {}",
                    table, expected_count, count.0
                ));
            }
        }

        tx.commit().await.map_err(|e| format!("提交事务失败: {}", e))?;
        Ok(())
    }.await;

    // 5. 无论如何都要 DETACH（不在事务内，失败也忽略）
    sqlx::query("DETACH DATABASE backup_db")
        .execute(&mut *conn)
        .await
        .ok();

    // 6. 如果恢复失败，报告错误（自动备份文件已保存，可供手动恢复）
    if let Err(ref e) = result {
        log::error!(
            "恢复失败: {}。自动备份保存在: {}",
            e,
            auto_backup_path
        );
        return Err(format!(
            "恢复失败: {}。原数据库已自动备份到: {}",
            e, auto_backup_path
        ));
    }

    // 7. 释放连接
    drop(conn);

    log::info!("数据库恢复成功，所有表行数校验通过");
    Ok(())
}

#[tauri::command]
pub async fn export_cohort(state: State<'_, AppState>, cohort_id: i64, file_path: String) -> Result<(), String> {
    let pool = &state.db;
    use std::io::Write;
    let file = std::fs::File::create(&file_path).map_err(|e| format!("创建导出文件失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<'_, ()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 导出学生
    let students = sqlx::query_as::<_, super::student::Student>(
        "SELECT * FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY student_no")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(&students).map_err(|e| e.to_string())?;
    zip.start_file("students.json", options).map_err(|e| e.to_string())?;
    zip.write_all(data.as_bytes()).map_err(|e| e.to_string())?;

    // 导出作业
    let homeworks = sqlx::query_as::<_, super::homework::Homework>(
        "SELECT * FROM homework WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY id")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    zip.start_file("homeworks.json", options).map_err(|e| e.to_string())?;
    zip.write_all(serde_json::to_string_pretty(&homeworks).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;

    // 导出作业完成记录
    let hw_ids: Vec<i64> = homeworks.iter().map(|h| h.id).collect();
    if !hw_ids.is_empty() {
        let records = sqlx::query_as::<_, super::homework::HomeworkRecord>(
            "SELECT * FROM homework_record WHERE homework_id IN (SELECT id FROM homework WHERE cohort_id = ?1)")
            .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if !records.is_empty() {
            zip.start_file("homework_records.json", options).map_err(|e| e.to_string())?;
            zip.write_all(serde_json::to_string_pretty(&records).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    // 导出考勤
    let attendance = sqlx::query_as::<_, super::attendance::Attendance>(
        "SELECT * FROM attendance WHERE cohort_id = ?1 ORDER BY attendance_date")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    if !attendance.is_empty() {
        zip.start_file("attendance.json", options).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&attendance).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
    }

    // 导出考试和成绩
    let exams = sqlx::query_as::<_, super::exam::Exam>(
        "SELECT * FROM exam WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY id")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    if !exams.is_empty() {
        zip.start_file("exams.json", options).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&exams).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;

        let scores = sqlx::query_as::<_, super::exam::Score>(
            "SELECT * FROM score WHERE exam_id IN (SELECT id FROM exam WHERE cohort_id = ?1)")
            .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if !scores.is_empty() {
            zip.start_file("scores.json", options).map_err(|e| e.to_string())?;
            zip.write_all(serde_json::to_string_pretty(&scores).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    // 导出事务
    let notices = sqlx::query_as::<_, super::affair::Notice>(
        "SELECT * FROM notice WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY id")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    if !notices.is_empty() {
        zip.start_file("notices.json", options).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&notices).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
    }

    // 导出值日
    let duties = sqlx::query(
        "SELECT d.*, s.name as student_name FROM duty d LEFT JOIN student s ON d.student_id = s.id WHERE d.cohort_id = ?1 ORDER BY d.duty_date")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    if !duties.is_empty() {
        let duties_json: Vec<serde_json::Value> = duties.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"), "cohort_id": r.get::<i64, _>("cohort_id"),
                "duty_date": r.get::<String, _>("duty_date"),
                "student_id": r.get::<Option<i64>, _>("student_id"),
                "group_name": r.get::<Option<String>, _>("group_name"),
                "duty_content": r.get::<Option<String>, _>("duty_content"),
                "status": r.get::<String, _>("status"),
                "remark": r.get::<Option<String>, _>("remark"),
            })
        }).collect();
        zip.start_file("duties.json", options).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&duties_json).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
    }

    // 导出奖惩
    let behaviors = sqlx::query(
        "SELECT b.*, s.name as student_name, s.student_no FROM behavior_record b JOIN student s ON b.student_id = s.id WHERE b.cohort_id = ?1 ORDER BY b.record_date")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    if !behaviors.is_empty() {
        let behaviors_json: Vec<serde_json::Value> = behaviors.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"), "cohort_id": r.get::<i64, _>("cohort_id"),
                "student_id": r.get::<i64, _>("student_id"),
                "type": r.get::<String, _>("type"), "title": r.get::<String, _>("title"),
                "score": r.get::<i64, _>("score"),
                "description": r.get::<Option<String>, _>("description"),
                "record_date": r.get::<String, _>("record_date"),
            })
        }).collect();
        zip.start_file("behavior_records.json", options).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&behaviors_json).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
    }

    // 导出元数据
    let cohort_info = sqlx::query_as::<_, super::cohort::Cohort>(
        "SELECT * FROM cohort WHERE id = ?1").bind(cohort_id)
        .fetch_one(pool).await.map_err(|e| e.to_string())?;
    let meta = serde_json::json!({
        "export_time": Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "app_version": "1.0.0",
        "cohort": cohort_info
    });
    zip.start_file("metadata.json", options).map_err(|e| e.to_string())?;
    zip.write_all(serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| format!("完成导出失败: {}", e))?;
    Ok(())
}
