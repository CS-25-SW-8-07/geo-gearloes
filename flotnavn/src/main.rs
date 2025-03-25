use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use rusty_roads::Roads;
use sqlx::{PgPool, Row};
use std::env;
use atlas::{bind, box_query};
use comms::Parquet;

// ((11.537934, 55.2575578), (11.536422, 55.2506889)) :: osm_id = 96676840 :: id = 176513
#[get("/boundingbox")]
async fn get_all_records(pool: web::Data<PgPool>) -> impl Responder {
    let bbox = ((11.537934, 55.2575578), (11.536422, 55.2506889));
    let conn = pool.acquire().await.unwrap();

    match atlas::box_query(conn, bbox, None).await {
        Ok(roads) => {
            let result = roads.into_iter().collect::<Roads>().to_parquet().expect("Could not compile to parquet");
                /* 
                .map(|road| {
                    let direction_str = match road.direction {
                        rusty_roads::Direction::Forward => "Forward",
                        rusty_roads::Direction::Backward => "Backward",
                        rusty_roads::Direction::Bidirectional => "Bidirectional",
                    };

                    format!(
                        "Road ID: {}\nOSM ID: {}\nCode: {}\nMaxSpeed: {}\nDirection: {}\nLayer: {}\nBridge: {}\nTunnel: {}\n\n", 
                        road.id, 
                        road.osm_id, 
                        road.code, 
                        road.maxspeed, 
                        direction_str,
                        road.layer,
                        road.bridge,
                        road.tunnel,
                    )
                })
                .collect::<String>();
            */

            HttpResponse::Ok().content_type("text/plain").body(result)
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}




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
            eprintln!("Database error: {:?}", e);
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
            .service(get_all_records)
    })
    .bind(("127.0.0.1", 8080))? // Bind to localhost:8080
    .run()
    .await
}