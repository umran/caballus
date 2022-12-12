use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::User;
use crate::entities::{Coordinates, Driver, Location, LocationSource, Quote, Route, Trip};
use crate::error::Error;

#[async_trait]
pub trait LocationAPI {
    async fn create_location(&self, user: User, source: LocationSource) -> Result<Location, Error>;
    async fn find_location(&self, user: User, token: Uuid) -> Result<Location, Error>;
}

#[async_trait]
pub trait RouteAPI {
    async fn create_route(
        &self,
        user: User,
        origin_token: Uuid,
        destination_token: Uuid,
    ) -> Result<Route, Error>;
    async fn find_route(&self, user: User, token: Uuid) -> Result<Route, Error>;
}

#[async_trait]
pub trait QuoteAPI {
    async fn create_quote(&self, user: User, route_token: Uuid) -> Result<Option<Quote>, Error>;
    async fn find_quote(&self, user: User, token: Uuid) -> Result<Quote, Error>;
}

#[async_trait]
pub trait TripAPI {
    async fn create_trip(&self, user: User, quote_token: Uuid) -> Result<Trip, Error>;
    async fn find_trip(&self, user: User, id: Uuid) -> Result<Trip, Error>;
    async fn request_driver(&self, user: User, id: Uuid) -> Result<Option<Trip>, Error>;
    async fn release_driver(&self, user: User, id: Uuid, driver_id: Uuid) -> Result<Trip, Error>;
    async fn accept_trip(&self, user: User, id: Uuid) -> Result<Trip, Error>;
    async fn reject_trip(&self, user: User, id: Uuid) -> Result<Trip, Error>;
    async fn cancel_trip(&self, user: User, id: Uuid) -> Result<Trip, Error>;
}

#[async_trait]
pub trait DriverAPI {
    async fn create_driver(&self, user: User) -> Result<Driver, Error>;
    async fn find_driver(&self, user: User, id: Uuid) -> Result<Driver, Error>;
    async fn start_driver(&self, user: User, id: Uuid) -> Result<Driver, Error>;
    async fn stop_driver(&self, user: User, id: Uuid) -> Result<Driver, Error>;
    async fn update_driver_rate(
        &self,
        user: User,
        id: Uuid,
        min_fare: f64,
        rate: f64,
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait DriverLocationAPI {
    async fn update_driver_location(
        &self,
        user: User,
        id: Uuid,
        coordinates: Coordinates,
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait DriverSearchAPI {
    async fn synchronize_drivers(&self, user: User, drivers: Vec<Driver>) -> Result<(), Error>;
    async fn find_drivers(&self, user: User, trip: Trip) -> Result<Vec<(Uuid, f64)>, Error>;
}

// service boundaries
pub trait LocationService: LocationAPI {}

pub trait RouteService: RouteAPI {}

pub trait BookingService: TripAPI + DriverAPI {}

pub trait DriverSearchService: DriverSearchAPI + DriverLocationAPI + QuoteAPI {}

// complete api
pub trait API: LocationAPI + RouteAPI + QuoteAPI + TripAPI + DriverAPI + DriverLocationAPI {}
