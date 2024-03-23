use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct Record {
    pub id: i32,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct VehicleModel {
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct PostVehicle {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct OdometerLatestModel {
    pub vehicle_id: i32,
    pub vehicle_name: String,
    pub odometer: i32,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct PostOdometer {
    pub odometer: i32,
}
