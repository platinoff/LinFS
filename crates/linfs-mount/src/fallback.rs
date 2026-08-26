use std::sync::Arc;

use axum::{extract::Query, http::StatusCode, response::Json, routing::get, Router};
use serde::Deserialize;

#[derive(Deserialize)]
struct PathQuery {
    path: Option<String>,
}

/// Axum fallback server exposing `/api/fs/*` over 127.0.0.1:9998.
/// Used when WinFSP driver is absent — internal browser + Monaco editor consume it.
pub async fn serve(fs: Arc<dyn linfs_core::fs::FileSystem>, addr: &str) -> linfs_core::Result<()> {
    let app = Router::new()
        .route(
            "/api/fs/readdir",
            get({
                let fs = fs.clone();
                move |Query(q): Query<PathQuery>| {
                    let fs = fs.clone();
                    async move { handle_readdir(fs, q).await }
                }
            }),
        )
        .route(
            "/api/fs/stat",
            get({
                let fs = fs.clone();
                move |Query(q): Query<PathQuery>| {
                    let fs = fs.clone();
                    async move { handle_stat(fs, q).await }
                }
            }),
        )
        .route(
            "/",
            get(|| async { "LinFS fallback 9998 — /api/fs/readdir?path=/etc" }),
        );

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        linfs_core::Error::Io(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            format!("bind {addr}: {e}"),
        ))
    })?;
    axum::serve(listener, app)
        .await
        .map_err(|e| linfs_core::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    Ok(())
}

async fn handle_readdir(
    fs: Arc<dyn linfs_core::fs::FileSystem>,
    q: PathQuery,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let path = q.path.unwrap_or_else(|| "/".to_string());
    // Resolve path via lookup chain: split and walk from root ino 2
    let ino = resolve_path(&fs, &path).map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let entries = fs
        .readdir(ino)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let json_entries: Vec<serde_json::Value> = entries
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "ino": e.ino,
                "name": String::from_utf8_lossy(&e.name).to_string(),
                "is_dir": e.is_dir
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "path": path,
        "ino": ino,
        "entries": json_entries
    })))
}

async fn handle_stat(
    fs: Arc<dyn linfs_core::fs::FileSystem>,
    q: PathQuery,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let path = q.path.unwrap_or_else(|| "/".to_string());
    let ino = resolve_path(&fs, &path).map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let attr = fs
        .getattr(ino)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({
        "path": path,
        "ino": attr.ino,
        "mode": format!("{:o}", attr.mode),
        "size": attr.size,
        "is_dir": attr.is_dir,
        "is_symlink": attr.is_symlink,
        "nlink": attr.nlink,
        "mtime": attr.mtime
    })))
}

fn resolve_path(fs: &Arc<dyn linfs_core::fs::FileSystem>, path: &str) -> linfs_core::Result<u64> {
    let mut ino = 2u64; // root
    if path == "/" || path.is_empty() {
        return Ok(ino);
    }
    for comp in path.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if comp == ".." {
            // Clamp at root for MVP
            if ino != 2 {
                // Would read parent; for MVP stay at root
            }
            continue;
        }
        ino = fs.lookup(ino, comp.as_bytes())?;
    }
    Ok(ino)
}
