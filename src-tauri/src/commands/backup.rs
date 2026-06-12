use chrono::Local;
use rust_xlsxwriter::Workbook;
use sha2::{Digest, Sha256};
use sqlx::Connection;
use sqlx::Row;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::db;
use crate::AppState;

/// 备份文件中的元数据表名
const BACKUP_META_TABLE: &str = "_backup_meta";
const BACKUP_DB_ENTRY: &str = "backup.sqlite";
const BACKUP_MANIFEST_ENTRY: &str = "manifest.json";
const BACKUP_FORMAT_VERSION: &str = "2.0.0";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BackupManifest {
    format_version: String,
    app_version: String,
    backup_version: String,
    backup_time: String,
    table_count: i64,
    checksum: String,
}

struct PreparedBackup {
    sqlite_path: String,
    cleanup_paths: Vec<String>,
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

/// 计算整个文件的 SHA256
async fn compute_file_sha256(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    let data = std::fs::read(path).map_err(|e| format!("无法读取文件用于计算校验值: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hex::encode(hasher.finalize());
    log::info!("File SHA256 ({}): {}", file_path, hash);
    Ok(hash)
}

fn is_zip_backup(file_path: &str) -> Result<bool, String> {
    let mut file =
        std::fs::File::open(file_path).map_err(|e| format!("无法打开备份文件: {}", e))?;
    let mut magic = [0u8; 4];
    let read = file
        .read(&mut magic)
        .map_err(|e| format!("无法读取备份文件头: {}", e))?;
    Ok(read == 4 && &magic == b"PK\x03\x04")
}

fn create_backup_package(
    sqlite_path: &str,
    file_path: &str,
    manifest: &BackupManifest,
) -> Result<(), String> {
    let file = std::fs::File::create(file_path).map_err(|e| format!("创建备份包失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<'_, ()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let sqlite_bytes =
        std::fs::read(sqlite_path).map_err(|e| format!("读取临时备份数据库失败: {}", e))?;
    zip.start_file(BACKUP_DB_ENTRY, options)
        .map_err(|e| format!("写入备份数据库条目失败: {}", e))?;
    zip.write_all(&sqlite_bytes)
        .map_err(|e| format!("写入备份数据库失败: {}", e))?;

    let manifest_bytes =
        serde_json::to_vec_pretty(manifest).map_err(|e| format!("序列化备份清单失败: {}", e))?;
    zip.start_file(BACKUP_MANIFEST_ENTRY, options)
        .map_err(|e| format!("写入备份清单条目失败: {}", e))?;
    zip.write_all(&manifest_bytes)
        .map_err(|e| format!("写入备份清单失败: {}", e))?;

    zip.finish().map_err(|e| format!("完成备份包失败: {}", e))?;
    Ok(())
}

fn extract_packaged_backup(file_path: &str) -> Result<PreparedBackup, String> {
    let file = std::fs::File::open(file_path).map_err(|e| format!("无法打开备份包: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("备份包已损坏，无法解析: {}", e))?;

    let mut manifest_entry = archive
        .by_name(BACKUP_MANIFEST_ENTRY)
        .map_err(|_| "备份包已损坏：缺少 manifest.json".to_string())?;
    let mut manifest_text = String::new();
    manifest_entry
        .read_to_string(&mut manifest_text)
        .map_err(|e| format!("读取备份清单失败: {}", e))?;
    drop(manifest_entry);
    let manifest: BackupManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("备份包已损坏：解析备份清单失败: {}", e))?;

    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(format!("不支持的备份格式版本: {}", manifest.format_version));
    }

    let temp_path =
        std::env::temp_dir().join(format!("class_copilot_restore_{}.sqlite", unique_suffix()));
    let temp_path_str = temp_path.to_string_lossy().to_string();

    let mut db_entry = archive
        .by_name(BACKUP_DB_ENTRY)
        .map_err(|_| "备份包已损坏：缺少 backup.sqlite".to_string())?;
    let mut output =
        std::fs::File::create(&temp_path).map_err(|e| format!("创建临时恢复文件失败: {}", e))?;
    std::io::copy(&mut db_entry, &mut output)
        .map_err(|e| format!("备份包已损坏：解压备份数据库失败: {}", e))?;

    let checksum = compute_file_sha256_sync(&temp_path_str)?;
    if checksum != manifest.checksum {
        let _ = std::fs::remove_file(&temp_path);
        return Err(
            "备份包校验失败：数据库内容与清单记录不一致，文件可能已损坏或被篡改".to_string(),
        );
    }

    Ok(PreparedBackup {
        sqlite_path: temp_path_str.clone(),
        cleanup_paths: vec![temp_path_str],
    })
}

fn compute_file_sha256_sync(file_path: &str) -> Result<String, String> {
    let data = std::fs::read(file_path).map_err(|e| format!("读取临时备份数据库失败: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

async fn verify_legacy_checksum_file(db_path: &str) -> Result<String, String> {
    let checksum_path = format!("{}.sha256", db_path);
    let path = Path::new(&checksum_path);
    if !path.exists() {
        return Err("旧版备份缺少 .sha256 校验文件，出于安全原因拒绝恢复".to_string());
    }
    let stored = std::fs::read_to_string(path)
        .map_err(|e| format!("读取校验值文件失败: {}", e))?
        .trim()
        .to_string();
    if stored.is_empty() {
        return Err("旧版备份的 .sha256 校验文件为空，拒绝恢复".to_string());
    }
    let current = compute_file_sha256(db_path).await?;
    if stored != current {
        return Err(format!(
            "备份文件校验失败：当前 SHA256 ({}) 与校验文件 ({}) 不匹配，文件可能已损坏或被篡改",
            &current[..16],
            &stored[..16]
        ));
    }
    Ok(current)
}

/// 校验备份文件是否为有效的 SQLite 数据库，并检查基本结构
async fn validate_sqlite_backup_file(
    file_path: &str,
    require_legacy_checksum: bool,
) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err("备份文件不存在".to_string());
    }
    let metadata = std::fs::metadata(path).map_err(|e| format!("无法读取备份文件: {}", e))?;
    if metadata.len() == 0 {
        return Err("备份文件为空".to_string());
    }

    // 校验是否为有效的 SQLite 数据库文件
    let header = std::fs::read(path).map_err(|e| format!("无法读取备份文件: {}", e))?;
    if header.len() < 16 || &header[0..16] != b"SQLite format 3\0" {
        return Err("备份文件不是有效的 SQLite 数据库".to_string());
    }

    if require_legacy_checksum {
        let checksum = verify_legacy_checksum_file(file_path).await?;
        log::info!("Legacy checksum verified: {}", &checksum[..16]);
    }

    // 计算当前文件 SHA256 用于返回
    let hash = compute_file_sha256(file_path).await?;
    log::info!("Backup file SHA256: {}", hash);

    // 校验备份元数据
    let conn_opts = format!("sqlite://{}?mode=ro", file_path);
    let opts: sqlx::sqlite::SqliteConnectOptions = conn_opts
        .parse()
        .map_err(|e| format!("无法解析备份数据库连接: {}", e))?;
    let mut conn = sqlx::SqliteConnection::connect_with(&opts)
        .await
        .map_err(|e| format!("无法连接备份数据库进行校验: {}", e))?;

    // 检查是否有_备份元数据
    let meta_exists: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1")
            .bind(BACKUP_META_TABLE)
            .fetch_one(&mut conn)
            .await
            .map_err(|e| format!("校验元数据失败: {}", e))?;

    if meta_exists.0 == 0 {
        log::warn!("备份文件缺少元数据表（可能是旧版本备份），跳过元数据校验");
        return Ok(hash);
    }

    // 读取元数据（版本、时间、表数量）
    let meta: (String, String, i64) =
        sqlx::query_as("SELECT backup_version, backup_time, table_count FROM _backup_meta LIMIT 1")
            .fetch_one(&mut conn)
            .await
            .map_err(|e| format!("读取备份元数据失败: {}", e))?;

    log::info!(
        "Backup version: {}, time: {}, tables: {}",
        meta.0,
        meta.1,
        meta.2
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
        "cohort",
        "student",
        "subject",
        "homework",
        "homework_record",
        "attendance",
        "exam",
        "score",
        "notice",
        "duty",
        "behavior_record",
        "system_config",
    ];

    for table in &required_tables {
        let exists: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1")
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
        let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM \"{}\"", table))
            .fetch_one(&mut conn)
            .await
            .map_err(|e| format!("统计表 {} 行数失败: {}", table, e))?;
        table_row_counts.push((table.to_string(), count.0));
    }

    let total_rows: i64 = table_row_counts.iter().map(|(_, c)| c).sum();
    log::info!(
        "Backup validation passed. Total rows: {}, Tables: {:?}",
        total_rows,
        table_row_counts
            .iter()
            .filter(|(_, c)| *c > 0)
            .map(|(t, c)| format!("{}={}", t, c))
            .collect::<Vec<_>>()
    );

    Ok(hash)
}

async fn prepare_backup_for_restore(file_path: &str) -> Result<PreparedBackup, String> {
    if is_zip_backup(file_path)? {
        let prepared = extract_packaged_backup(file_path)?;
        if let Err(err) = validate_sqlite_backup_file(&prepared.sqlite_path, false).await {
            for cleanup_path in &prepared.cleanup_paths {
                let _ = std::fs::remove_file(cleanup_path);
            }
            return Err(err);
        }
        return Ok(prepared);
    }

    validate_sqlite_backup_file(file_path, true).await?;
    Ok(PreparedBackup {
        sqlite_path: file_path.to_string(),
        cleanup_paths: Vec::new(),
    })
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

/// 在临时 SQLite 备份数据库中写入元数据（备份版本、时间、表数量）
async fn write_backup_meta(pool: &sqlx::SqlitePool, file_path: &str) -> Result<(), String> {
    let conn_opts = format!("sqlite://{}", file_path);
    let opts: sqlx::sqlite::SqliteConnectOptions = conn_opts
        .parse()
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
    let temp_backup_path =
        std::env::temp_dir().join(format!("class_copilot_backup_{}.sqlite", unique_suffix()));
    let temp_backup_str = temp_backup_path.to_string_lossy().to_string();

    // Step 1: 使用 VACUUM INTO 创建临时 SQLite 备份文件
    let escaped_path = temp_backup_str.replace('\'', "''");
    let backup_sql = format!("VACUUM INTO '{}'", escaped_path);
    sqlx::query(&backup_sql)
        .execute(pool)
        .await
        .map_err(|e| format!("创建临时备份失败: {}", e))?;

    let path = Path::new(&temp_backup_str);
    if !path.exists() {
        return Err("临时备份文件未生成".to_string());
    }
    let metadata = std::fs::metadata(path).map_err(|e| format!("无法读取临时备份文件: {}", e))?;
    if metadata.len() == 0 {
        return Err("临时备份文件为空".to_string());
    }

    // Step 2: 写入元数据（版本/时间/表数量）
    write_backup_meta(pool, &temp_backup_str).await?;

    // Step 3: 对临时数据库计算 SHA256
    let checksum = compute_file_sha256(&temp_backup_str).await?;

    // Step 4: 生成单文件备份包
    let table_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_backup%'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("统计表数量失败: {}", e))?;
    let manifest = BackupManifest {
        format_version: BACKUP_FORMAT_VERSION.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        backup_version: "1.0.0".to_string(),
        backup_time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        table_count: table_count.0,
        checksum: checksum.clone(),
    };
    let package_result = create_backup_package(&temp_backup_str, &file_path, &manifest);
    let _ = std::fs::remove_file(&temp_backup_path);
    package_result?;

    log::info!(
        "Backup package created successfully at: {} (checksum: {})",
        file_path,
        checksum
    );
    Ok(())
}

#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, file_path: String) -> Result<(), String> {
    let pool = &state.db;

    db::run_migrations(pool)
        .await
        .map_err(|e| format!("恢复前初始化数据库结构失败: {}", e))?;

    // 1. 校验备份文件完整性（含结构校验）
    let prepared = prepare_backup_for_restore(&file_path).await?;
    log::info!("Backup validated: sqlite_path={}", prepared.sqlite_path);

    // 2. 自动备份当前数据库到临时目录
    let auto_backup_path = format!(
        "{}/class_copilot_auto_backup_before_restore_{}.db",
        std::env::temp_dir().display(),
        unique_suffix()
    );
    let _ = std::fs::remove_file(&auto_backup_path);
    let escaped_auto = auto_backup_path.replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{}'", escaped_auto))
        .execute(pool)
        .await
        .map_err(|e| format!("自动备份当前数据库失败: {}", e))?;

    log::info!("Auto backup saved to: {}", auto_backup_path);

    // 3. 使用单一专用连接执行恢复（确保 ATTACH 和事务在同一连接上）
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let escaped_path = prepared.sqlite_path.replace('\'', "''");

    // 在事务外执行 ATTACH（SQLite 要求 ATTACH 不能在事务内）
    sqlx::query(&format!("ATTACH DATABASE '{}' AS backup_db", escaped_path))
        .execute(&mut *conn)
        .await
        .map_err(|e| format!("附加备份数据库失败: {}", e))?;

    // 4. 在事务内执行删除和导入
    let result: Result<(), String> = async {
        // 预校验：检查备份数据库表结构与主数据库一致
        let expected_tables = get_insert_order();
        for table in expected_tables {
            let exists: (i64,) = sqlx::query_as(&format!(
                "SELECT COUNT(*) FROM backup_db.sqlite_master WHERE type='table' AND name='{}'",
                table
            ))
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
            let count: (i64,) =
                sqlx::query_as(&format!("SELECT COUNT(*) FROM backup_db.\"{}\"", table))
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
            let main_columns = sqlx::query(&format!("PRAGMA main.table_info(\"{}\")", table))
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| format!("读取当前表 {} 结构失败: {}", table, e))?;
            let backup_columns =
                sqlx::query(&format!("PRAGMA backup_db.table_info(\"{}\")", table))
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(|e| format!("读取备份表 {} 结构失败: {}", table, e))?;

            let backup_column_names: std::collections::HashSet<String> = backup_columns
                .iter()
                .map(|row| row.get::<String, _>("name"))
                .collect();
            let shared_columns: Vec<String> = main_columns
                .iter()
                .map(|row| row.get::<String, _>("name"))
                .filter(|name| backup_column_names.contains(name))
                .collect();

            if shared_columns.is_empty() {
                return Err(format!(
                    "表 {} 在当前数据库与备份之间不存在可恢复的同名列",
                    table
                ));
            }

            let column_list = shared_columns
                .iter()
                .map(|name| format!("\"{}\"", name))
                .collect::<Vec<_>>()
                .join(", ");
            let import_sql = format!(
                "INSERT INTO main.\"{}\" ({}) SELECT {} FROM backup_db.\"{}\"",
                table, column_list, column_list, table
            );
            sqlx::query(&import_sql)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("导入表 {} 失败: {}", table, e))?;
        }

        // 恢复后校验：检查导入行数与备份一致
        for (table, expected_count) in &backup_counts {
            let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM \"{}\"", table))
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

        tx.commit()
            .await
            .map_err(|e| format!("提交事务失败: {}", e))?;
        Ok(())
    }
    .await;

    // 5. 无论如何都要 DETACH（不在事务内，失败也忽略）
    sqlx::query("DETACH DATABASE backup_db")
        .execute(&mut *conn)
        .await
        .ok();

    // 6. 如果恢复失败，报告错误（自动备份文件已保存，可供手动恢复）
    if let Err(ref e) = result {
        for cleanup_path in &prepared.cleanup_paths {
            let _ = std::fs::remove_file(cleanup_path);
        }
        log::error!("恢复失败: {}。自动备份保存在: {}", e, auto_backup_path);
        return Err(format!(
            "恢复失败: {}。原数据库已自动备份到: {}",
            e, auto_backup_path
        ));
    }

    // 7. 释放连接
    drop(conn);

    for cleanup_path in prepared.cleanup_paths {
        let _ = std::fs::remove_file(cleanup_path);
    }

    log::info!("数据库恢复成功，所有表行数校验通过");
    Ok(())
}

#[tauri::command]
pub async fn export_cohort(
    state: State<'_, AppState>,
    cohort_id: i64,
    file_path: String,
) -> Result<(), String> {
    let pool = &state.db;
    use std::io::Write;
    let file = std::fs::File::create(&file_path).map_err(|e| format!("创建导出文件失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<'_, ()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 导出学生
    let students = sqlx::query_as::<_, super::student::Student>(
        "SELECT * FROM student WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY student_no",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(&students).map_err(|e| e.to_string())?;
    zip.start_file("students.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    let student_count = students.len() as i64;

    // 导出作业
    let homeworks = sqlx::query_as::<_, super::homework::Homework>(
        "SELECT * FROM homework WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY id",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    zip.start_file("homeworks.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        serde_json::to_string_pretty(&homeworks)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    let homework_count = homeworks.len() as i64;
    let mut attachment_count = 0_i64;
    if let Some(app_handle) = state.app_handle.lock().await.clone() {
        let app_data_dir = crate::resolve_app_data_dir(&app_handle)?;
        for homework in &homeworks {
            if let (Some(attachment_name), Some(attachment_path)) =
                (&homework.attachment_name, &homework.attachment_path)
            {
                let absolute_path = app_data_dir.join(attachment_path);
                if absolute_path.exists() {
                    attachment_count += 1;
                    let attachment_bytes = std::fs::read(&absolute_path)
                        .map_err(|e| format!("读取作业附件失败: {}", e))?;
                    zip.start_file(format!("attachments/homework/{}", attachment_name), options)
                        .map_err(|e| e.to_string())?;
                    zip.write_all(&attachment_bytes)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    // 导出作业完成记录
    let hw_ids: Vec<i64> = homeworks.iter().map(|h| h.id).collect();
    let mut homework_record_count = 0_i64;
    if !hw_ids.is_empty() {
        let records = sqlx::query_as::<_, super::homework::HomeworkRecord>(
            "SELECT * FROM homework_record WHERE homework_id IN (SELECT id FROM homework WHERE cohort_id = ?1)")
            .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
        homework_record_count = records.len() as i64;
        if !records.is_empty() {
            zip.start_file("homework_records.json", options)
                .map_err(|e| e.to_string())?;
            zip.write_all(
                serde_json::to_string_pretty(&records)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // 导出考勤
    let attendance = sqlx::query_as::<_, super::attendance::Attendance>(
        "SELECT * FROM attendance WHERE cohort_id = ?1 ORDER BY attendance_date",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let attendance_count = attendance.len() as i64;
    if !attendance.is_empty() {
        zip.start_file("attendance.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            serde_json::to_string_pretty(&attendance)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    }

    // 导出考试和成绩
    let exams = sqlx::query_as::<_, super::exam::Exam>(
        "SELECT * FROM exam WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY id",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let exam_count = exams.len() as i64;
    let mut score_count = 0_i64;
    if !exams.is_empty() {
        zip.start_file("exams.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            serde_json::to_string_pretty(&exams)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;

        let scores = sqlx::query_as::<_, super::exam::Score>(
            "SELECT * FROM score WHERE exam_id IN (SELECT id FROM exam WHERE cohort_id = ?1)",
        )
        .bind(cohort_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
        score_count = scores.len() as i64;
        if !scores.is_empty() {
            zip.start_file("scores.json", options)
                .map_err(|e| e.to_string())?;
            zip.write_all(
                serde_json::to_string_pretty(&scores)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // 导出事务
    let notices = sqlx::query_as::<_, super::affair::Notice>(
        "SELECT * FROM notice WHERE cohort_id = ?1 AND deleted_at IS NULL ORDER BY id",
    )
    .bind(cohort_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let notice_count = notices.len() as i64;
    if !notices.is_empty() {
        zip.start_file("notices.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            serde_json::to_string_pretty(&notices)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    }

    // 导出值日
    let duties = sqlx::query(
        "SELECT d.*, s.name as student_name FROM duty d LEFT JOIN student s ON d.student_id = s.id WHERE d.cohort_id = ?1 ORDER BY d.duty_date")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    let duty_count = duties.len() as i64;
    if !duties.is_empty() {
        let duties_json: Vec<serde_json::Value> = duties
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.get::<i64, _>("id"), "cohort_id": r.get::<i64, _>("cohort_id"),
                    "duty_date": r.get::<String, _>("duty_date"),
                    "student_id": r.get::<Option<i64>, _>("student_id"),
                    "group_name": r.get::<Option<String>, _>("group_name"),
                    "duty_content": r.get::<Option<String>, _>("duty_content"),
                    "status": r.get::<String, _>("status"),
                    "remark": r.get::<Option<String>, _>("remark"),
                })
            })
            .collect();
        zip.start_file("duties.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            serde_json::to_string_pretty(&duties_json)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    }

    // 导出奖惩
    let behaviors = sqlx::query(
        "SELECT b.*, s.name as student_name, s.student_no FROM behavior_record b JOIN student s ON b.student_id = s.id WHERE b.cohort_id = ?1 ORDER BY b.record_date")
        .bind(cohort_id).fetch_all(pool).await.map_err(|e| e.to_string())?;
    let behavior_count = behaviors.len() as i64;
    if !behaviors.is_empty() {
        let behaviors_json: Vec<serde_json::Value> = behaviors
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.get::<i64, _>("id"), "cohort_id": r.get::<i64, _>("cohort_id"),
                    "student_id": r.get::<i64, _>("student_id"),
                    "type": r.get::<String, _>("type"), "title": r.get::<String, _>("title"),
                    "score": r.get::<i64, _>("score"),
                    "description": r.get::<Option<String>, _>("description"),
                    "record_date": r.get::<String, _>("record_date"),
                })
            })
            .collect();
        zip.start_file("behavior_records.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            serde_json::to_string_pretty(&behaviors_json)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    }

    // 导出元数据
    let cohort_info =
        sqlx::query_as::<_, super::cohort::Cohort>("SELECT * FROM cohort WHERE id = ?1")
            .bind(cohort_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
    let summary = serde_json::json!({
        "student_count": student_count,
        "homework_count": homework_count,
        "homework_record_count": homework_record_count,
        "attendance_count": attendance_count,
        "exam_count": exam_count,
        "score_count": score_count,
        "notice_count": notice_count,
        "duty_count": duty_count,
        "behavior_count": behavior_count,
        "attachment_count": attachment_count
    });
    let meta = serde_json::json!({
        "export_time": Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "app_version": "1.0.0",
        "cohort": cohort_info,
        "summary": summary
    });
    zip.start_file("metadata.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        serde_json::to_string_pretty(&meta)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;

    let mut workbook = Workbook::new();
    let summary_sheet = workbook.add_worksheet();
    summary_sheet
        .write_string(0, 0, "届次数据导出摘要")
        .map_err(|e| e.to_string())?;
    summary_sheet
        .write_string(1, 0, "届次")
        .map_err(|e| e.to_string())?;
    summary_sheet
        .write_string(
            1,
            1,
            &format!("{} {}", cohort_info.cohort_name, cohort_info.class_name),
        )
        .map_err(|e| e.to_string())?;
    summary_sheet
        .write_string(2, 0, "导出时间")
        .map_err(|e| e.to_string())?;
    summary_sheet
        .write_string(2, 1, &Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
        .map_err(|e| e.to_string())?;
    for (idx, (label, value)) in [
        ("学生数量", student_count),
        ("作业数量", homework_count),
        ("作业记录数量", homework_record_count),
        ("考勤记录数量", attendance_count),
        ("考试数量", exam_count),
        ("成绩记录数量", score_count),
        ("通知数量", notice_count),
        ("值日数量", duty_count),
        ("奖惩数量", behavior_count),
        ("附件数量", attachment_count),
    ]
    .iter()
    .enumerate()
    {
        let row = (4 + idx) as u32;
        summary_sheet
            .write_string(row, 0, *label)
            .map_err(|e| e.to_string())?;
        summary_sheet
            .write_number(row, 1, *value as f64)
            .map_err(|e| e.to_string())?;
    }

    let mut bytes = workbook
        .save_to_buffer()
        .map_err(|e| format!("生成 Excel 摘要失败: {}", e))?;
    zip.start_file("export_summary.xlsx", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&mut bytes).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| format!("完成导出失败: {}", e))?;
    Ok(())
}
