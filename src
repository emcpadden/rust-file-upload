use actix_web::{web, App, HttpServer, Error, HttpResponse, middleware::Logger};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use std::io::Write;
use sanitize_filename::sanitize;
use std::path::PathBuf;
use env_logger::Env;

async fn save_file(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition();

        let filename = content_type.get_filename().map(sanitize).unwrap_or_else(|| "unnamed_file".to_string());
        let filepath = PathBuf::from("./uploads").join(filename);

        // Use `move` to take ownership of `filepath`
        let mut f = web::block(move || std::fs::File::create(filepath)).await??;

        while let Some(chunk) = field.next().await {
            let data = chunk?;
            // Use `move` here as well
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
            .route("/upload", web::put().to(save_file))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
