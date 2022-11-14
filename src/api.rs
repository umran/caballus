use async_trait::async_trait;

use crate::bid::Bid;
use crate::error::Error;
use crate::route::{Place, Route};
use crate::trip::Trip;

#[async_trait(?Send)]
pub trait RouteAPI {
    async fn find_route(&self, id: &str) -> Result<Route, Error>;
    async fn create_route(&self, origin: Place, destination: Place) -> Result<Route, Error>;
}

#[async_trait(?Send)]
pub trait TripAPI {
    async fn find_trip(&self, id: &str) -> Result<Trip, Error>;

    async fn create_trip(&self, route_id: String, passenger_id: String) -> Result<Trip, Error>;

    async fn expand_search(&self, id: String) -> Result<Trip, Error>;

    async fn evaluate_bids(&self, id: String) -> Result<Trip, Error>;

    async fn submit_bid(&self, bid: Bid) -> Result<(), Error>;
}

pub trait API: RouteAPI + TripAPI {}
