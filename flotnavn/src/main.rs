use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use atlas::box_query;
use comms::Parquet;
use rusty_roads::Roads;
use sqlx::{PgPool, Row};
use std::env;

mod http_methods;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok(); // loads from .env file
    let db_username = env::var("DB_USERNAME").expect("DB_USERNAME must be set");
    let db_password = env::var("DB_PASSWORD").expect("DB_PASSWORD must be set");
    let db_address = env::var("DB_ADDRESS").expect("DB_ADDRESS must be set");
    let db_name = env::var("DB_NAME").expect("DB_NAME must be set");
    let db_port = env::var("DB_PORT").expect("DB_PORT must be set");

    //  construct the DATABASE_URL
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_username, db_password, db_address, db_port, db_name
    );

    // Use the bind function to get a lazy database pool
    let pool = atlas::create_pool(&database_url, None).await.unwrap(); // This is using the `connect_lazy`
    println!("Successfully connected to Postgres");

    // Start the HTTP server asynchronously with Actix
    println!("Starting server on 127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone())) // Share the pool across all routes
            .service(http_methods::testing123)
            .service(http_methods::get_roads_in_bbox)
    })
    .bind(("127.0.0.1", 8080))? // Bind to localhost:8080
    .run()
    .await
}
