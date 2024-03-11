mod model;
mod schema;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{get, HttpResponse, Responder};
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use env_logger;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::model::*;

#[get("/")]
async fn index() -> impl Responder {
    "Hello, World!"
}

#[get("/vehicles")]
async fn get_vehicles(data: web::Data<AppState>) -> impl Responder {
    let vehicles: Vec<VehicleModel> = sqlx::query_as!(
        VehicleModel,
        r#"SELECT id, "name", description FROM public.vehicles;"#,
    )
    .fetch_all(&data.db)
    .await
    .unwrap();

    let json_response = serde_json::json!({
        "rows": vehicles.len(),
        "vehicles": vehicles
    });
    HttpResponse::Ok().json(json_response)
}

#[get("/vehicles/{id}")]
async fn get_vehicle_by_id(data: web::Data<AppState>, path: web::Path<(i32,)>) -> impl Responder {
    let vehicle_id = path.into_inner().0;
    let vehicle: Option<VehicleModel> = sqlx::query_as!(
        VehicleModel,
        r#"SELECT id, "name", description FROM public.vehicles WHERE id = $1"#,
        vehicle_id,
    )
    .fetch_optional(&data.db)
    .await
    .unwrap();

    HttpResponse::Ok().json(vehicle)
}

#[get("/vehicles/{id}/odometer")]
async fn get_vehicle_odometer_by_id(
    data: web::Data<AppState>,
    path: web::Path<(i32,)>,
) -> impl Responder {
    let vehicle_id = path.into_inner().0;
    let odometer_latest: Option<OdometerLatestModel> = sqlx::query_as!(
        OdometerLatestModel,
        r#"
        SELECT o.vehicle_id, v.name AS vehicle_name, o.odometer, o.timestamp
        FROM vehicle_odometer o
        INNER JOIN vehicles v ON o.vehicle_id = v.id
        WHERE o.vehicle_id = $1
        ORDER BY o.timestamp DESC
        LIMIT 1
        "#,
        vehicle_id,
    )
    .fetch_optional(&data.db)
    .await
    .unwrap();

    // Check if a row was returned
    HttpResponse::Ok().json(odometer_latest)
}

pub struct AppState {
    db: PgPool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    println!("🚀 Server started successfully");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { db: pool.clone() }))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allow_any_method()
                    .supports_credentials(),
            )
            .wrap(Logger::default())
            .service(index)
            .service(get_vehicles)
            .service(get_vehicle_by_id)
            .service(get_vehicle_odometer_by_id)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
