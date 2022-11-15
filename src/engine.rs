use async_trait::async_trait;
use futures::TryStreamExt;
use sqlx::{pool::PoolConnection, types::Json, Executor, Postgres, Row};

use crate::{
    api::{RouteAPI, TripAPI, API},
    bid::Bid,
    db::{DBHandle, TransactionError},
    driver::Driver,
    error::{self, invalid_input_error, Error},
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

    async fn get_db_conn(&self) -> Result<PoolConnection<Database>, Error> {
        self.db_handle
            .acquire_conn()
            .await
            .map_err(|err| error::database_error(err))
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

    async fn find_route(&self, id: String) -> Result<Route, Error> {
        let mut conn = self.get_db_conn().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM routes WHERE id = $1").bind(&id))
            .await
            .map_err(|err| error::database_error(err))?;

        if let Some(result) = maybe_result {
            let Json(route) = result
                .try_get("data")
                .map_err(|err| error::database_error(err))?;

            return Ok(route);
        }

        Err(error::invalid_input_error())
    }
}

#[async_trait(?Send)]
impl<T: DBHandle<DB = Database>> TripAPI for Engine<T> {
    async fn find_trip(&self, id: String) -> Result<Trip, Error> {
        let mut conn = self.get_db_conn().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT data FROM trips WHERE id = $1").bind(&id))
            .await
            .map_err(|err| error::database_error(err))?;

        if let Some(result) = maybe_result {
            let Json(trip) = result
                .try_get("data")
                .map_err(|err| error::database_error(err))?;
            return Ok(trip);
        }

        Err(error::invalid_input_error())
    }

    async fn create_trip(&self, route_id: String, passenger_id: String) -> Result<Trip, Error> {
        let mut conn = self.get_db_conn().await?;

        let maybe_result = conn
            .fetch_optional(sqlx::query("SELECT id FROM routes WHERE id = $1").bind(&route_id))
            .await
            .map_err(|err| error::database_error(err))?;

        if let Some(_) = maybe_result {
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

            return Ok(trip);
        }

        Err(error::invalid_input_error())
    }

    async fn expand_search(&self, id: String) -> Result<Trip, Error> {
        let trip = self
            .db_handle
            .exec_tx(|tx| {
                Box::pin(async move {
                    let maybe_result = tx
                        .fetch_optional(
                            sqlx::query("SELECT data FROM trips WHERE id = $1 FOR UPDATE")
                                .bind(&id),
                        )
                        .await
                        .map_err(|err| error::database_error(err))?;

                    if let Some(result) = maybe_result {
                        let Json::<Trip>(mut trip) = result
                            .try_get("data")
                            .map_err(|err| error::database_error(err))?;
                        trip.expand_search()?;

                        tx.execute(
                            sqlx::query("UPDATE trips SET data = $2 WHERE id = $1")
                                .bind(&id)
                                .bind(Json(&trip)),
                        )
                        .await
                        .map_err(|err| error::database_error(err))?;

                        return Ok(trip);
                    }

                    Err(invalid_input_error())
                })
            })
            .await
            .map_err(|err| match err {
                TransactionError::ApplicationError(err) => err,
                TransactionError::DBError(err) => error::database_error(err),
            })?;

        Ok(trip)
    }

    async fn evaluate_bids(&self, id: String) -> Result<Option<Trip>, Error> {
        let mut conn = self.get_db_conn().await?;

        let mut results = conn.fetch(
            sqlx::query("SELECT id, driver_id, fare FROM bids WHERE trip_id = $1").bind(&id),
        );

        while let Some(row) = results
            .try_next()
            .await
            .map_err(|err| error::database_error(err))?
        {
            let trip_id = id.clone();
            let bid_id: String = row
                .try_get("id")
                .map_err(|err| error::database_error(err))?;
            let driver_id: String = row
                .try_get("driver_id")
                .map_err(|err| error::database_error(err))?;

            let maybe_trip: Option<Trip> = self
                .db_handle
                .exec_tx(|tx| {
                    Box::pin(async move {
                        let driver_result = tx
                            .fetch_one(
                                sqlx::query("SELECT data FROM drivers WHERE id = $1 FOR UPDATE")
                                    .bind(&driver_id),
                            )
                            .await
                            .map_err(|err| error::database_error(err))?;
                        let Json::<Driver>(mut driver) = driver_result
                            .try_get("data")
                            .map_err(|err| error::database_error(err))?;

                        if !driver.is_available() {
                            return Ok(None);
                        }

                        driver.assign_trip(trip_id.clone())?;

                        tx.execute(
                            sqlx::query("UPDATE drivers SET status = $2, data = $3 WHERE id = $1")
                                .bind(&driver_id)
                                .bind(&driver.status_string())
                                .bind(Json(&driver)),
                        )
                        .await
                        .map_err(|err| error::database_error(err))?;

                        let trip_result = tx
                            .fetch_one(
                                sqlx::query("SELECT data FROM trips WHERE id = $1 FOR UPDATE")
                                    .bind(&trip_id),
                            )
                            .await
                            .map_err(|err| error::database_error(err))?;
                        let Json::<Trip>(mut trip) = trip_result
                            .try_get("data")
                            .map_err(|err| error::database_error(err))?;

                        trip.accept_bid(bid_id)?;

                        tx.execute(
                            sqlx::query("UPDATE trips SET status = $2, data = $3 WHERE id = $1")
                                .bind(&trip_id)
                                .bind(&trip.status_string())
                                .bind(Json(&trip)),
                        )
                        .await
                        .map_err(|err| error::database_error(err))?;

                        Ok(Some(trip))
                    })
                })
                .await
                .map_err(|err| match err {
                    TransactionError::ApplicationError(err) => err,
                    TransactionError::DBError(err) => error::database_error(err),
                })?;

            if maybe_trip.is_some() {
                return Ok(maybe_trip);
            }
        }

        Ok(None)
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
