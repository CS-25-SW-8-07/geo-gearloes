use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use rusty_roads::Roads;
use sqlx::{PgPool, Row};
use std::env;
use atlas::{bind, box_query};
use comms::Parquet;

// ((11.537934, 55.2575578), (11.536422, 55.2506889)) :: osm_id = 96676840 :: id = 176513
#[get("/boundingbox")]
async fn get_all_records(pool: web::Data<PgPool>) -> impl Responder {
    let bbox = ((11.537934, 55.2575578), (11.5175512, 55.2537322));
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