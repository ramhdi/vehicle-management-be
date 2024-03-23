mod model;
mod schema;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{delete, get, post, HttpResponse, Responder};
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
    let result = sqlx::query_as!(
        VehicleModel,
        r#"SELECT id, "name", description FROM public.vehicles;"#,
    )
    .fetch_all(&data.db)
    .await;

    match result {
        Ok(vehicles) => {
            let json_response = serde_json::json!({
                "rows": vehicles.len(),
                "vehicles": vehicles
            });
            HttpResponse::Ok().json(json_response)
        }
        Err(_) => HttpResponse::InternalServerError().body("Failed to query vehicles"),
    }
}

#[get("/vehicles/{id}")]
async fn get_vehicle_by_id(data: web::Data<AppState>, path: web::Path<(i32,)>) -> impl Responder {
    let vehicle_id = path.into_inner().0;
    let result = sqlx::query_as!(
        VehicleModel,
        r#"SELECT id, "name", description FROM public.vehicles WHERE id = $1"#,
        vehicle_id,
    )
    .fetch_optional(&data.db)
    .await;

    match result {
        Ok(Some(vehicle)) => HttpResponse::Ok().json(vehicle),
        Ok(None) => HttpResponse::NotFound().body("Vehicle not found"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to query vehicle"),
    }
}

#[post("/vehicles")]
async fn post_vehicle(
    data: web::Data<AppState>,
    request: web::Json<PostVehicle>,
) -> impl Responder {
    let result = sqlx::query_as!(
        Record,
        r#"
        INSERT INTO public.vehicles
        ("name", description)
        VALUES($1, $2)
        RETURNING id;
        "#,
        request.name,
        request.description
    )
    .fetch_one(&data.db)
    .await;

    match result {
        Ok(record) => HttpResponse::Ok().json(record.id),
        Err(_) => HttpResponse::InternalServerError().body("Failed to create vehicle"),
    }
}

#[delete("/vehicles/{id}")]
async fn delete_vehicle_by_id(
    data: web::Data<AppState>,
    path: web::Path<(i32,)>,
) -> impl Responder {
    let vehicle_id = path.into_inner().0;
    let result = sqlx::query_as!(
        Record,
        r#"
        DELETE FROM public.vehicles
        WHERE id=$1
        RETURNING id;
        "#,
        vehicle_id,
    )
    .fetch_optional(&data.db)
    .await;

    match result {
        Ok(Some(_)) => HttpResponse::Ok().body("Vehicle deleted"),
        Ok(None) => HttpResponse::NotFound().body("Vehicle not found"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to delete vehicle"),
    }
}

#[get("/vehicles/{id}/odometer")]
async fn get_vehicle_odometer_by_id(
    data: web::Data<AppState>,
    path: web::Path<(i32,)>,
) -> impl Responder {
    let vehicle_id = path.into_inner().0;
    let result = sqlx::query_as!(
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
    .await;

    match result {
        Ok(Some(odometer_latest)) => HttpResponse::Ok().json(odometer_latest),
        Ok(None) => HttpResponse::NotFound().body("No odometer record"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to query odometer"),
    }
}

#[post("/vehicles/{id}/odometer")]
async fn post_odometer(
    data: web::Data<AppState>,
    path: web::Path<(i32,)>,
    request: web::Json<PostOdometer>,
) -> impl Responder {
    let vehicle_id = path.into_inner().0;
    let odometer = request.into_inner().odometer;

    let result = sqlx::query!(
        r#"
        INSERT INTO public.vehicle_odometer
        (vehicle_id, odometer, "timestamp")
        VALUES($1, $2, (now() AT TIME ZONE 'UTC'::text));
        "#,
        vehicle_id,
        odometer
    )
    .execute(&data.db)
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().body("Odometer updated successfully"),
        Err(e) => {
            if e.to_string().contains("foreign key constraint") {
                HttpResponse::BadRequest().body("Invalid vehicle ID")
            } else {
                HttpResponse::InternalServerError().body("Internal Server Error")
            }
        }
    }
}

pub struct AppState {
    db: PgPool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port = std::env::var("HTTP_PORT")
        .expect("HTTP_PORT must be set")
        .parse::<u16>()
        .expect("HTTP_PORT must be a valid number");
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
            .service(post_vehicle)
            .service(delete_vehicle_by_id)
            .service(get_vehicle_odometer_by_id)
            .service(post_odometer)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
