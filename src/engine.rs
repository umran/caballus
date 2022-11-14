// SELECT data AS trip FROM trips WHERE id = trip_id AND status = 'SEARCHING'
// SELECT id, driver_id, amount FROM bids WHERE trip_id = trip_id ORDER BY amount ASC
// for each bid, while trip.driver_id == None:
// BEGIN
// SELECT * FROM drivers WHERE id = bid.driver_id AND status = 'AVAILABLE' FOR UPDATE
// if rows not empty:
// trip.accept_bid(bid)
// UPDATE trips SET (data) VALUES (trip) WHERE id = trip.id
// UPDATE drivers SET (status) VALUES ('ASSIGNED') WHERE id = bid.driver_id
// COMMIT

use async_trait::async_trait;
use sqlx::{types::Json, Executor, Postgres, Row};

use crate::{
    api::{RouteAPI, TripAPI, API},
    bid::Bid,
    db::DBHandle,
    error::{self, Error},
    route::{Place, Route},
    trip::Trip,
};

type Database = Postgres;

#[derive(Debug)]
pub struct Engine<T: DBHandle<DB = Database>> {
    db_handle: T,
}

impl<T: DBHandle<DB = Database>> Engine<T> {
    pub async fn new(db_handle: T) -> Self {
        Self { db_handle }
    }
}

#[async_trait(?Send)]
impl<T: DBHandle<DB = Database>> RouteAPI for Engine<T> {
    async fn create_route(&self, origin: Place, destination: Place) -> Result<Route, Error> {
        let id = "".to_string();
        Ok(Route {
            id,
            origin,
            destination,
            distance: 0.0,
        })
    }

    async fn find_route(&self, id: &str) -> Result<Route, Error> {
        let mut conn = self
            .db_handle
            .acquire_conn()
            .await
            .map_err(|err| error::database_error(err))?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM routes WHERE id = $1").bind(id))
            .await
            .map_err(|err| error::database_error(err))?;

        match maybe_result {
            Some(result) => {
                let Json(route) = result
                    .try_get("data")
                    .map_err(|err| error::database_error(err))?;

                Ok(route)
            }
            None => Err(error::invalid_input_error()),
        }
    }
}

#[async_trait(?Send)]
impl<T: DBHandle<DB = Database>> TripAPI for Engine<T> {
    async fn find_trip(&self, id: &str) -> Result<Trip, Error> {
        let mut conn = self
            .db_handle
            .acquire_conn()
            .await
            .map_err(|err| error::database_error(err))?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(id))
            .await
            .map_err(|err| error::database_error(err))?;

        match maybe_result {
            Some(result) => {
                let Json(trip) = result
                    .try_get("data")
                    .map_err(|err| error::database_error(err))?;
                Ok(trip)
            }
            None => Err(error::invalid_input_error()),
        }
    }

    async fn create_trip(&self, route_id: String, passenger_id: String) -> Result<Trip, Error> {
        let mut conn = self
            .db_handle
            .acquire_conn()
            .await
            .map_err(|err| error::database_error(err))?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT id FROM routes WHERE id = $1").bind(&route_id))
            .await
            .map_err(|err| error::database_error(err))?;

        match maybe_result {
            Some(_) => {
                let trip_id = "".to_string();
                let trip = Trip::new(trip_id, route_id, passenger_id);

                conn.execute(
                    sqlx::query("INSERT INTO trips (id, status, data) VALUES ($1, $2, $3)")
                        .bind(&trip.id)
                        .bind(&trip.status_string())
                        .bind(Json(&trip)),
                )
                .await
                .map_err(|err| error::database_error(err))?;

                Ok(trip)
            }
            None => Err(error::invalid_input_error()),
        }
    }

    async fn expand_search(&self, id: String) -> Result<Trip, Error> {
        Err(Error {
            code: 0,
            message: "unimplemented".to_string(),
        })
    }

    async fn evaluate_bids(&self, id: String) -> Result<Trip, Error> {
        Err(Error {
            code: 0,
            message: "unimplemented".to_string(),
        })
    }

    async fn submit_bid(&self, bid: Bid) -> Result<(), Error> {
        Err(Error {
            code: 0,
            message: "unimplemented".to_string(),
        })
    }
}

impl<T: DBHandle<DB = Database>> API for Engine<T> {}

#[test]
fn new_engine() {
    fn send_test<T: Send>(x: T) {}

    use crate::db::PgStore;
    use tokio_test::block_on;

    let pg_store = block_on(PgStore::new(
        "postgresql://caballus:caballus@localhost:5432/caballus",
        5,
    ))
    .unwrap();

    let engine = block_on(Engine::new(pg_store));

    let arc_engine = std::sync::Arc::new(engine);

    send_test(arc_engine);
}
