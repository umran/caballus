use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::entities::{Bid, LocationSource, LocationToken, Route, Trip};
use crate::error::Error;

#[async_trait]
pub trait GeoAPI {
    async fn find_location_token(&self, id: Uuid) -> Result<LocationToken, Error>;
    async fn create_location_token(&self, source: LocationSource) -> Result<LocationToken, Error>;
}

#[async_trait]
pub trait RouteAPI {
    async fn find_route(&self, id: Uuid) -> Result<Route, Error>;
    async fn create_route(&self, origin_id: Uuid, destination_id: Uuid) -> Result<Route, Error>;
}

#[async_trait]
pub trait TripAPI {
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error>;
    async fn create_trip(&self, route_id: Uuid, passenger_id: Uuid) -> Result<Trip, Error>;
    async fn expand_search(&self, id: Uuid) -> Result<Trip, Error>;
    async fn evaluate_bids(&self, id: Uuid) -> Result<Option<Trip>, Error>;
    async fn submit_bid(&self, trip_id: Uuid, driver_id: Uuid, amount: i64) -> Result<Bid, Error>;
}

pub trait API: GeoAPI + RouteAPI + TripAPI {}

pub type DynAPI = Arc<dyn API + Send + Sync>;
