use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use sqlx::{PgPool, Row};
use dotenv::dotenv;
use std::env;
use atlas::


#[get("/namecolumn/{id}")]
// Using unwrap_or_else if the variables are not loaded. public is default for schema, users for table.
async fn get_name_by_id(id: web::Path<i32>, pool: web::Data<PgPool>) -> impl Responder {
    let schema = env::var("DB_SCHEMA").unwrap_or_else(|_| "public".to_string());
    let table = env::var("DB_TABLE").unwrap_or_else(|_| "users".to_string());
    let query = format!("SELECT name FROM \"{}\".\"{}\" WHERE id = $1", schema, table);

// Executing the query using sqlx. ok = when the match is found. none = name not found. err = Query execution failed    
    match sqlx::query(&query)
        .bind(id.into_inner())
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(Some(row)) => {
            if let Ok(name) = row.try_get::<String, _>("name") {
                HttpResponse::Ok().body(format!("Hello, {}!", name))
            } else {
                HttpResponse::InternalServerError().body("Failed to retrieve name")
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Name not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[get("/wholetable")]
async fn get_all_records(pool: web::Data<PgPool>) -> impl Responder {
    atlas::box_query(pool, ((1.0, 1.0),(1.0, 1.0)), None);
    let schema = env::var("DB_SCHEMA").unwrap_or_else(|_| "public".to_string());
    let table = env::var("DB_TABLE").unwrap_or_else(|_| "users".to_string());
    let query = format!("SELECT id, name, age FROM \"{}\".\"{}\"", schema, table);

    // Using is_empty instead of none here, since a table with no content would be an empty vector
    // Could maybe use fetch_optional instead of fetch_all ?
    match sqlx::query(&query).fetch_all(pool.get_ref()).await {
        Ok(rows) => {
            if rows.is_empty() {
                return HttpResponse::NoContent().finish();
            }

            let mut results = Vec::new();
            for row in rows {
                // Create vector with order (id, name, age)
                let mut row_vec = Vec::new();

                row_vec.push((
                    "id".to_string(),
                    row.try_get::<i32, _>("id").map(|v| v.to_string()).unwrap_or_else(|_| "NULL".to_string())
                ));
                row_vec.push((
                    "name".to_string(),
                    row.try_get::<String, _>("name").unwrap_or_else(|_| "NULL".to_string())
                ));
                row_vec.push((
                    "age".to_string(),
                    row.try_get::<i32, _>("age").map(|v| v.to_string()).unwrap_or_else(|_| "NULL".to_string())
                ));

                results.push(row_vec);
            }

            HttpResponse::Ok().json(results)
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok(); // loads from .env file

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = atlas::bind(database_url.as_str(), None).await?;
    // let pool = PgPool::connect_lazy(&database_url).expect("Failed to create database connection pool");

    println!("Successfully connected to Postgres");

    // Starting the server asynchronously with Actix_web. Have to .service to register all endpoints.
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool))
            .service(get_name_by_id)
            .service(get_all_records)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
