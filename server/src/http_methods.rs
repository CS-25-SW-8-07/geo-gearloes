use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    get, web, App, Error, HttpResponse, HttpServer, Responder,
};

use atlas::{create_pool, box_query};
use comms::Parquet;
use rusty_roads::Roads;
use sqlx::{PgPool, Row};
use std::env;

pub fn services<T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>>(
    app: App<T>,
) -> App<T> {
    app.service(testing123).service(get_roads_in_bbox)
}

// ((11.537934, 55.2575578), (11.536422, 55.2506889)) :: osm_id = 96676840 :: id = 176513
// http://127.0.0.1:8080/get_roads_in_bbox.parquet?lon1=11.537934&lat1=55.2575578&lon2=11.536422&lat2=55.2506889
// http://127.0.0.1:8080/get_roads_in_bbox.parquet?lon1=11.537934&lat1=55.2575578&lon2=11.5175512&lat2=55.2537322

fn get_bbox(query: &std::collections::HashMap<String, String>) -> ((f64, f64), (f64, f64)) {
    let get_coord = |key: &str| {
        query
            .get(key)
            .and_then(|val| val.parse::<f64>().ok())
            .unwrap_or_default()
    };

    let lon1 = get_coord("lon1");
    let lat1 = get_coord("lat1");
    let lon2 = get_coord("lon2");
    let lat2 = get_coord("lat2");

    ((lon1, lat1), (lon2, lat2))
}

#[get("/get_roads_in_bbox.parquet")]
async fn get_roads_in_bbox(
    pool: web::Data<PgPool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let bbox = get_bbox(&query);
    let conn = pool.acquire().await.unwrap();

    match atlas::box_query(conn, bbox, None).await {
        Ok(roads) => {
            let result = roads
                .into_iter()
                .collect::<Roads>()
                .to_parquet()
                .expect("Could not compile to parquet");
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(result) // return a binary type
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}
/*
#[get("/get_visits_in_bbox.parquet")]
async fn get_roads_in_bbox(pool: web::Data<PgPool>, query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let bbox = get_bbox(&query);
    let conn = pool.acquire().await.unwrap();

    match atlas::box_query(conn, bbox, None).await {
        Ok() => {

        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}
 */

#[get("/test")]
async fn testing123(pool: web::Data<PgPool>) -> impl Responder {
    let query_result = sqlx::query("SELECT * FROM public.roadname LIMIT 1")
        .fetch_one(pool.get_ref())
        .await;

    match query_result {
        Ok(row) => {
            let roadname: String = row
                .try_get("name")
                .unwrap_or_else(|_| "Unknown".to_string());
            HttpResponse::Ok().json(roadname) // Json sucks apparently
        }
        Err(e) => {
            eprintln!("Database error: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Database error: {:?}", e))
        }
    }
}

