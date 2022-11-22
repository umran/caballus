use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::{Bid, Location, LocationSource, Route, Trip};
use crate::error::Error;

#[async_trait]
pub trait LocationAPI {
    async fn find_location(&self, token: Uuid) -> Result<Location, Error>;
    async fn create_location(&self, source: LocationSource) -> Result<Location, Error>;
}

#[async_trait]
pub trait RouteAPI {
    async fn find_route(&self, token: Uuid) -> Result<Route, Error>;
    async fn create_route(
        &self,
        origin_token: Uuid,
        destination_token: Uuid,
    ) -> Result<Route, Error>;
}

#[async_trait]
pub trait TripAPI {
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error>;
    async fn create_trip(&self, route_token: Uuid, passenger_id: Uuid) -> Result<Trip, Error>;
    async fn expand_search(&self, id: Uuid) -> Result<Trip, Error>;
    async fn evaluate_bids(&self, id: Uuid) -> Result<Option<Trip>, Error>;
    async fn submit_bid(&self, trip_id: Uuid, driver_id: Uuid, amount: i64) -> Result<Bid, Error>;
}

pub trait API: LocationAPI + RouteAPI + TripAPI {}
