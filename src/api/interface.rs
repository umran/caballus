use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::entities::{Bid, Place, Route, Trip};
use crate::error::Error;

#[async_trait]
pub trait RouteAPI {
    async fn find_route(&self, id: Uuid) -> Result<Route, Error>;
    async fn create_route(&self, origin: Place, destination: Place) -> Result<Route, Error>;
}

#[async_trait]
pub trait TripAPI {
    async fn find_trip(&self, id: Uuid) -> Result<Trip, Error>;
    async fn create_trip(&self, route_id: Uuid, passenger_id: Uuid) -> Result<Trip, Error>;
    async fn expand_search(&self, id: Uuid) -> Result<Trip, Error>;
    async fn evaluate_bids(&self, id: Uuid) -> Result<Option<Trip>, Error>;
    async fn submit_bid(&self, bid: Bid) -> Result<(), Error>;
}

pub trait API: RouteAPI + TripAPI {}

pub type DynAPI = Arc<dyn API + Send + Sync>;
