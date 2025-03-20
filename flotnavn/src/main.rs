use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use sqlx::{PgPool, Row};
use std::env;
use atlas::{bind, box_query};



/* #[get("/boundingbox")]
async fn get_all_records(pool: web::Data<PgPool>) -> impl Responder {
atlas::box_query(pool, ((1.0, 1.0),(1.0, 1.0)), None);
} */

#[get("/test")]
async fn testing123(pool: web::Data<PgPool>) -> impl Responder {
    let query_result = sqlx::query("SELECT * FROM public.roadname LIMIT 1")
        .fetch_one(pool.get_ref())
        .await;

    match query_result {
        Ok(row) => {
            let roadname: String = row.try_get("name").unwrap_or_else(|_| "Unknown".to_string());
            HttpResponse::Ok().json(roadname)
        }
        Err(e) => {
            eprintln!("Database error: {:?}", e); // Print error message in console
            HttpResponse::InternalServerError().body(format!("Database error: {:?}", e))
        }
    }
}




#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok(); // loads from .env file

    // Read individual environment variables
    let db_username = env::var("DB_USERNAME").expect("DB_USERNAME must be set");
    let db_password = env::var("DB_PASSWORD").expect("DB_PASSWORD must be set");
    let db_address = env::var("DB_ADDRESS").expect("DB_ADDRESS must be set");
    let db_name = env::var("DB_NAME").expect("DB_NAME must be set");
    let db_port = env::var("DB_PORT").expect("DB_PORT must be set");

    // Manually construct the DATABASE_URL using the variables
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_username, db_password, db_address, db_port, db_name
    );

    // Use the bind function to get a lazy database pool
    let pool = atlas::bind(&database_url, None).await.unwrap(); // This is using the `connect_lazy`

    println!("Successfully connected to Postgres");

    // Start the HTTP server asynchronously with Actix
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone())) // Share the pool across all routes
            .service(testing123)
    })
    .bind(("127.0.0.1", 8080))? // Bind to localhost:8080
    .run()
    .await
}