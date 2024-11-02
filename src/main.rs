// main.rs

use actix_web::{
    middleware::Logger,
    web, App, Error, HttpResponse, HttpServer,
};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use sanitize_filename::sanitize;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use env_logger::Env;
use log::{error, info};
use actix_web::rt::time::timeout;

/// Handler to save the uploaded file with a timeout
async fn save_file(
    path: web::Path<(String, String)>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    // Define the timeout duration (1 hour)
    let upload_timeout = Duration::from_secs(3600);

    // Wrap the entire upload process within a timeout
    match timeout(upload_timeout, async {
        let (id, file_type) = path.into_inner();
        let uploads_dir = PathBuf::from("./uploads");

        // Create uploads directory if it doesn't exist
        if !uploads_dir.exists() {
            tokio::fs::create_dir_all(&uploads_dir)
                .await
                .map_err(|e| {
                    error!("Failed to create uploads directory: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to create uploads directory")
                })?;
        }

        let mut file_saved = false;

        // Iterate over multipart fields
        while let Some(mut field) = payload.try_next().await? { // Make `field` mutable
            let content_disposition = field.content_disposition();

            // Extract and sanitize the filename
            let filename = content_disposition
                .get_filename()
                .map(sanitize)
                .unwrap_or_else(|| "unnamed_file".to_string());

            // Sanitize `id` and `file_type`
            let sanitized_id = sanitize(&id);
            let sanitized_file_type = sanitize(&file_type);

            // Construct the new filename with id and type
            let new_filename = if let Some(ext) = PathBuf::from(&filename).extension() {
                format!(
                    "{}-{}-{}.{}",
                    PathBuf::from(&filename)
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .unwrap_or("unnamed_file"),
                    sanitized_id,
                    sanitized_file_type,
                    ext.to_string_lossy()
                )
            } else {
                format!("{}-{}-{}", filename, sanitized_id, sanitized_file_type)
            };

            let filepath = uploads_dir.join(new_filename.clone());

            // Create the file asynchronously
            let mut f = File::create(&filepath).await.map_err(|e| {
                error!("Failed to create file {:?}: {}", filepath, e);
                actix_web::error::ErrorInternalServerError("Failed to create file")
            })?;

            // Write the file data in chunks
            while let Some(chunk) = field.next().await {
                let data = chunk?;
                f.write_all(&data).await.map_err(|e| {
                    error!("Failed to write to file {:?}: {}", filepath, e);
                    actix_web::error::ErrorInternalServerError("Failed to write to file")
                })?;
            }

            file_saved = true;
        }

        if file_saved {
            Ok(HttpResponse::Ok().body("File uploaded successfully"))
        } else {
            Ok(HttpResponse::BadRequest().body("No file uploaded"))
        }
    }).await {
        Ok(result) => result,
        Err(_) => {
            // Handle timeout
            error!("File upload timed out");
            Err(actix_web::error::ErrorGatewayTimeout("Upload timed out"))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let uploads_dir = PathBuf::from("./uploads");
    if !uploads_dir.exists() {
        tokio::fs::create_dir_all(&uploads_dir).await?;
    }

    info!("Uploads directory: {:?}", uploads_dir.canonicalize()?);
    info!("Server starting at http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .app_data(
                web::PayloadConfig::new(10 * 1024 * 1024 * 1024) // Set payload limit to 10 GB
                    .limit(10 * 1024 * 1024 * 1024),
            )
            .route("/upload/{id}/{type}", web::put().to(save_file))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
