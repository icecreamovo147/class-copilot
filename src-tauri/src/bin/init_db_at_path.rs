use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --bin init_db_at_path -- <db-path>");
        std::process::exit(2);
    };

    let db_path = PathBuf::from(path);
    let parent = db_path.parent().map(PathBuf::from);
    if let Some(parent) = parent {
        if let Err(error) = std::fs::create_dir_all(&parent) {
            eprintln!("failed to create parent dir {}: {}", parent.display(), error);
            std::process::exit(1);
        }
    }

    match class_copilot_lib::db::init_db(&db_path).await {
        Ok(pool) => {
            pool.close().await;
            println!("initialized db at {}", db_path.display());
        }
        Err(error) => {
            eprintln!("failed to initialize db {}: {}", db_path.display(), error);
            std::process::exit(1);
        }
    }
}
