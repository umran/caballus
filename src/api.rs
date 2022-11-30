use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::{Coordinates, Driver, Location, LocationSource, Quote, Route, Trip};
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
    async fn create_quote(&self, route_token: Uuid) -> Result<Option<Quote>, Error>;
    async fn find_quote(&self, token: Uuid) -> Result<Quote, Error>;
}

#[async_trait]
pub trait TripAPI {
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error>;
    async fn create_trip(&self, quote_token: Uuid, passenger_id: Uuid) -> Result<Trip, Error>;
    async fn request_driver(&self, id: Uuid) -> Result<Trip, Error>;
    async fn derequest_driver(&self, id: Uuid, rejected: bool) -> Result<Trip, Error>;
}

#[async_trait]
pub trait DriverAPI {
    async fn find_driver(&self, id: Uuid) -> Result<Driver, Error>;
    async fn create_driver(&self, user_id: Uuid) -> Result<Driver, Error>;
    async fn start_driver(&self, id: Uuid) -> Result<Driver, Error>;
    async fn stop_driver(&self, id: Uuid) -> Result<Driver, Error>;
    async fn update_driver_location(&self, id: Uuid, location: Coordinates) -> Result<(), Error>;
    async fn update_driver_rate(&self, id: Uuid, min_fare: f64, rate: f64) -> Result<(), Error>;
}

pub trait API: LocationAPI + RouteAPI + TripAPI {}
