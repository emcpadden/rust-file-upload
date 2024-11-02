use actix_web::{web, App, HttpServer, Error, HttpResponse, middleware::Logger};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use std::io::Write;
use sanitize_filename::sanitize;
use std::path::PathBuf;
use env_logger::Env;
use std::ffi::OsStr;

async fn save_file(
    path: web::Path<(String, String)>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let (id, file_type) = path.into_inner();
    let uploads_dir = PathBuf::from("./uploads");
    if !uploads_dir.exists() {
        std::fs::create_dir_all(&uploads_dir)?;
    }

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();

        let filename = content_disposition
            .get_filename()
            .map(sanitize)
            .unwrap_or_else(|| "unnamed_file".to_string());

        // Split the filename into stem and extension
        let original_path = PathBuf::from(&filename);
        let stem = original_path
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("unnamed_file");
        let extension = original_path.extension().and_then(OsStr::to_str);

        // Construct the new filename with id and type
        let new_filename = if let Some(ext) = extension {
            format!("{}-{}-{}.{}", stem, id, file_type, ext)
        } else {
            format!("{}-{}-{}", stem, id, file_type)
        };

        let filepath = uploads_dir.join(new_filename);

        let mut f = web::block(move || std::fs::File::create(filepath)).await??;

        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f = web::block(move || f.write_all(&data).map(|_| f)).await??;
        }
    }
    Ok(HttpResponse::Ok().body("File uploaded successfully"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let uploads_dir = PathBuf::from("./uploads");
    if !uploads_dir.exists() {
        std::fs::create_dir_all(&uploads_dir)?;
    }

    println!("Uploads directory: {:?}", uploads_dir.canonicalize()?);
    println!("Server starting at http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .route("/upload/{id}/{type}", web::put().to(save_file))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
