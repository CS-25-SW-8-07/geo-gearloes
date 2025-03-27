use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use rusty_roads::Roads;
use sqlx::{PgPool, Row};
use std::env;
use atlas::{bind, box_query};
use comms::Parquet;

// ((11.537934, 55.2575578), (11.536422, 55.2506889)) :: osm_id = 96676840 :: id = 176513
// http://127.0.0.1:8080/get_roads_in_bbox.parquet?lon1=11.537934&lat1=55.2575578&lon2=11.536422&lat2=55.2506889
// http://127.0.0.1:8080/get_roads_in_bbox.parquet?lon1=11.537934&lat1=55.2575578&lon2=11.5175512&lat2=55.2537322

#[get("/get_roads_in_bbox.parquet")]
async fn get_roads_in_bbox(pool: web::Data<PgPool>, query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let lon1 = query.get("lon1").and_then(|lon1_str| lon1_str.parse::<f64>().ok()).unwrap_or_default();
    let lat1 = query.get("lat1").and_then(|lat1_str| lat1_str.parse::<f64>().ok()).unwrap_or_default();
    let lon2 = query.get("lon2").and_then(|lon2_str| lon2_str.parse::<f64>().ok()).unwrap_or_default();
    let lat2 = query.get("lat2").and_then(|lat2_str| lat2_str.parse::<f64>().ok()).unwrap_or_default();

    let bbox = ((lon1, lat1), (lon2, lat2));
    let conn = pool.acquire().await.unwrap();

    match atlas::box_query(conn, bbox, None).await {
        Ok(roads) => {
            let result = roads.into_iter().collect::<Roads>().to_parquet().expect("Could not compile to parquet");
            HttpResponse::Ok().content_type("text/plain").body(result)
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

// ##[get("/get_visits_in_bbox.parquet")]

#[get("/test")]
async fn testing123(pool: web::Data<PgPool>) -> impl Responder {
    let query_result = sqlx::query("SELECT * FROM public.roadname LIMIT 1")
        .fetch_one(pool.get_ref())
        .await;

    match query_result {
        Ok(row) => {
            let roadname: String = row.try_get("name").unwrap_or_else(|_| "Unknown".to_string());
            HttpResponse::Ok().json(roadname) // Json sucks apparently
        }
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Database error: {:?}", e))
        }
    }
}