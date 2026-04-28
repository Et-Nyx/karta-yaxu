use axum::{
    extract::{Multipart, Path},
    response::IntoResponse,
    http::StatusCode,
    Json,
};
use serde_json::json;
use crate::auth::EditorSession;
use crate::filters;
use std::path::PathBuf;
use tokio::fs;

pub async fn upload_image(
    _session: EditorSession,
    Path(folder): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // Validate the folder strictly to prevent directory traversal
    if folder != "pokemon" && folder != "shiny" {
        return Err(crate::error::AppError::Internal("Pasta de upload inválida".into()));
    }

    let mut saved_filename = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| crate::error::AppError::Internal(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        
        if name == "file" {
            let original_filename = field.file_name().unwrap_or_default().to_string();
            
            // Clean up the filename
            let parts: Vec<&str> = original_filename.rsplitn(2, '.').collect();
            let safe_filename = if parts.len() == 2 {
                let ext = parts[0].to_lowercase();
                let stem = parts[1];
                let slugified_stem = filters::to_slug(stem);
                // Allow only specific extensions
                if ext != "png" && ext != "jpg" && ext != "jpeg" && ext != "gif" && ext != "webp" {
                    return Err(crate::error::AppError::Internal("Extensão de arquivo não permitida".into()));
                }
                format!("{}.{}", slugified_stem, ext)
            } else {
                return Err(crate::error::AppError::Internal("Arquivo sem extensão válida".into()));
            };

            let data = field.bytes().await.map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

            // Save the file
            let dir_path = format!("static/sprites/{}", folder);
            fs::create_dir_all(&dir_path).await.map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
            let file_path = PathBuf::from(dir_path).join(&safe_filename);
            
            // Ensure no path traversal in the final file_path
            fs::write(&file_path, &data).await.map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
            
            saved_filename = Some(safe_filename);
            break;
        }
    }

    if let Some(filename) = saved_filename {
        Ok(Json(json!({
            "success": true,
            "filename": filename
        })).into_response())
    } else {
        Err(crate::error::AppError::Internal("Nenhum arquivo enviado".into()))
    }
}
