use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::{Location, LocationSource, Quote, Route, Trip};
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
pub trait QuoteAPI {
    async fn create_quote(&self, route_token: Uuid) -> Result<Quote, Error>;
    async fn find_quote(&self, token: Uuid) -> Result<Quote, Error>;
}

#[async_trait]
pub trait TripAPI {
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error>;
    async fn create_trip(&self, quote_token: Uuid, passenger_id: Uuid) -> Result<Trip, Error>;
    // async fn assign_driver(&self, id: Uuid) -> Result<Trip, Error>;
}

pub trait API: LocationAPI + RouteAPI + TripAPI {}
