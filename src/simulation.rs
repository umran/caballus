// use async_channel::{Receiver, Sender};
// use chrono::Utc;
// use rand_distr::{Binomial, Distribution, Normal, Uniform};
// use std::collections::{HashMap, HashSet};
// use std::sync::Arc;
// use tokio::sync::Mutex;
// use uuid::Uuid;

// use crate::api::{DriverAPI, LocationAPI, QuoteAPI, RouteAPI, TripAPI};
// use crate::engine::Engine;
// use crate::entities::{DriverStatus, Location, LocationSource, TripStatus};
// use crate::error::Error;

// async fn place_id_to_location(engine: &Engine, place_id: &str) -> Location {
//     let source = LocationSource::GooglePlaces {
//         place_id: place_id.into(),
//         session_token: "".into(),
//     };

//     engine.create_location(source).await.unwrap()
// }

// fn sample_binomial(n: u64, p: f64) -> u64 {
//     let bin = Binomial::new(n, p).unwrap();
//     bin.sample(&mut rand::thread_rng())
// }

// fn handle_invocation_error<T>(result: Result<T, Error>) {
//     match result {
//         Ok(_) => {}
//         Err(err) => {
//             if err.code != 100 {
//                 panic!("unexpected error");
//             }

//             tracing::warn!("invalid invocation error");
//         }
//     }
// }

// struct Simulation {
//     e: Engine,
//     locations: Mutex<HashMap<i64, Location>>,
//     driver_ids: Mutex<HashSet<Uuid>>,
//     trip_ids: Mutex<HashSet<Uuid>>,
// }

// impl Simulation {
//     #[tracing::instrument(name = "Simulation::new")]
//     async fn new(e: Engine) -> Self {
//         macro_rules! collection {
//             ($($k:expr => $v:expr),* $(,)?) => {{
//                 core::convert::From::from([$(($k, place_id_to_location(&e, $v).await),)*])
//             }};
//         }

//         let locations: HashMap<i64, Location> = collection! {
//             0 => "ChIJgT_rKAB_PzsRBttnRY6jpz8",
//             1 => "ChIJkRUcFP9-PzsRPJOvVDoV9Q4",
//             2 => "ChIJAwkZQ1R-PzsRhv0Dv_s7BBA",
//             3 => "ChIJm4S3B1d-PzsRP51UdJChpok",
//             4 => "ChIJU356hvh-PzsR7rZPICsgENY",
//             5 => "ChIJQ4KuXP9-PzsRCTwPeuSM84w",
//             6 => "ChIJre6GSxB_PzsRSlNfQQKAuN0",
//             7 => "ChIJXf6ODFN-PzsR7kNO7_1U7VE",
//             8 => "ChIJVZxeT4h_PzsR0_zSiiOd9QA",
//             9 => "ChIJ5bA0oqp_PzsRZCBEwNpttnI"
//         };

//         Self {
//             e,
//             locations: Mutex::new(locations),
//             driver_ids: Mutex::new(HashSet::new()),
//             trip_ids: Mutex::new(HashSet::new()),
//         }
//     }

//     #[tracing::instrument(skip(self))]
//     fn sample_rate(&self) -> (f64, f64) {
//         let mut rng = rand::thread_rng();
//         let min_fare_dist = Normal::new(15.0, 3.0).unwrap();
//         let rate_dist = Normal::new(0.15, 0.05).unwrap();

//         let min_fare = min_fare_dist.sample(&mut rng);
//         let rate = rate_dist.sample(&mut rng);

//         (min_fare, rate)
//     }

//     #[tracing::instrument(skip(self))]
//     async fn sample_location(&self) -> Location {
//         let die = Uniform::from(0..9);
//         let location_index: i64 = die.sample(&mut rand::thread_rng());

//         let location = self
//             .locations
//             .lock()
//             .await
//             .get(&location_index)
//             .unwrap()
//             .clone();

//         location
//     }

//     #[tracing::instrument(skip(self))]
//     async fn add_driver(&self) {
//         let user_id = Uuid::new_v4();

//         tracing::info!("creating driver for user_id: {:?}", &user_id);

//         // create driver
//         let mut driver = self.e.create_driver(user_id).await.unwrap();

//         tracing::info!("created driver with id: {:?}", &driver.id);

//         // update rate
//         let (min_fare, rate) = self.sample_rate();

//         self.e
//             .update_driver_rate(driver.id.clone(), min_fare, rate)
//             .await
//             .unwrap();

//         tracing::info!("updated min_fare: {:?} and rate: {:?}", min_fare, rate);

//         // update location
//         let location = self.sample_location().await;

//         self.e
//             .update_driver_location(driver.id.clone(), location.coordinates)
//             .await
//             .unwrap();

//         tracing::info!("updated driver location: {:?}", location.description);

//         driver = self.e.start_driver(driver.id.clone()).await.unwrap();

//         tracing::info!("started driver");

//         self.driver_ids.lock().await.insert(driver.id.clone());
//     }

//     #[tracing::instrument(skip(self))]
//     async fn add_trip(&self) {
//         let user_id = Uuid::new_v4();

//         tracing::info!("attempting to create trip for user id {:?}", &user_id);

//         let origin = self.sample_location().await;
//         let destination = self.sample_location().await;

//         tracing::info!("creating route for trip");
//         let route = self
//             .e
//             .create_route(origin.token, destination.token)
//             .await
//             .unwrap();

//         tracing::info!("successfully created route for trip");

//         tracing::info!("attempting to create quote for trip");

//         let quote = self.e.create_quote(route.token).await.unwrap();

//         if let Some(quote) = quote {
//             tracing::info!("successfully received quote for trip: {:?}", &quote);

//             let trip = self.e.create_trip(quote.token, user_id).await.unwrap();
//             self.trip_ids.lock().await.insert(trip.id.clone());
//         } else {
//             tracing::warn!("failed to get quote for trip, no drivers nearby");
//         }
//     }
// }

// pub struct Executor {
//     s: Arc<Simulation>,
// }

// impl Executor {
//     #[tracing::instrument(name = "Executor::new")]
//     pub async fn new(e: Engine) -> Self {
//         Self {
//             s: Arc::new(Simulation::new(e).await),
//         }
//     }

//     #[tracing::instrument(skip(self))]
//     pub async fn run(&self) {
//         self.initialize_drivers().await;
//         self.initialize_trips().await;

//         let keepalive_drivers_handle = self.keepalive_drivers();
//         let keepalive_trips_handle = self.keepalive_trips();

//         tokio::join! {
//             keepalive_drivers_handle,
//             keepalive_trips_handle
//         };
//     }

//     #[tracing::instrument(skip(self))]
//     async fn initialize_drivers(&self) {
//         let (tx, rx): (Sender<()>, Receiver<()>) = async_channel::unbounded();

//         let mut handles = vec![];
//         for _ in 0..99 {
//             let rx = rx.clone();
//             let s = self.s.clone();

//             let handle = tokio::spawn(async move {
//                 while let Ok(_) = rx.recv().await {
//                     s.add_driver().await;
//                 }
//             });

//             handles.push(handle);
//         }

//         handles.push(tokio::spawn(async move {
//             for _ in 0..999 {
//                 tx.send(()).await.unwrap();
//             }
//         }));

//         futures::future::join_all(handles).await;
//     }

//     #[tracing::instrument(skip(self))]
//     async fn initialize_trips(&self) {
//         let (tx, rx): (Sender<()>, Receiver<()>) = async_channel::unbounded();

//         let mut handles = vec![];
//         for _ in 0..99 {
//             let rx = rx.clone();
//             let s = self.s.clone();

//             let handle = tokio::spawn(async move {
//                 while let Ok(_) = rx.recv().await {
//                     s.add_trip().await;
//                 }
//             });

//             handles.push(handle);
//         }

//         handles.push(tokio::spawn(async move {
//             for _ in 0..999 {
//                 tx.send(()).await.unwrap();
//             }
//         }));

//         futures::future::join_all(handles).await;
//     }

//     #[tracing::instrument(skip(self))]
//     async fn keepalive_drivers(&self) {
//         let (tx, rx): (Sender<Uuid>, Receiver<Uuid>) = async_channel::unbounded();

//         let mut handles = vec![];
//         for _ in 0..99 {
//             let rx = rx.clone();

//             let s = self.s.clone();

//             let handle = tokio::spawn(async move {
//                 while let Ok(driver_id) = rx.recv().await {
//                     let location = s.sample_location().await;

//                     tracing::info!("fetching driver with id: {:?}", &driver_id);

//                     let driver = s.e.find_driver(driver_id.clone()).await.unwrap();

//                     match driver.status {
//                         DriverStatus::Requested { trip_id } => {
//                             // make decision to accept, reject or do nothing (using trip_id as seed for random output)
//                             if sample_binomial(1, 0.9) > 0 {
//                                 tracing::info!("attempting to assign driver...");
//                                 handle_invocation_error(
//                                     s.e.assign_driver(trip_id, driver.id.clone()).await,
//                                 );
//                             } else if sample_binomial(1, 0.05) > 0 {
//                                 tracing::info!("attempting to derequest driver...");
//                                 handle_invocation_error(
//                                     s.e.derequest_driver(trip_id, driver.id.clone(), true).await,
//                                 )
//                             }
//                         }
//                         DriverStatus::Assigned { trip_id } => {
//                             // make decision to cancel trip
//                             if sample_binomial(1, 0.5) > 0 {
//                                 tracing::info!("attempting to cancel trip...");
//                                 handle_invocation_error(
//                                     s.e.cancel_trip(trip_id, Some(driver.id.clone())).await,
//                                 );
//                             }
//                         }
//                         DriverStatus::Available => {
//                             tracing::warn!("driver is available: no trips requested or assigned");
//                         }
//                         _ => (),
//                     };

//                     tracing::info!(
//                         "updating location of driver {:?} to {:?}",
//                         &driver.id,
//                         &location.description
//                     );

//                     s.e.update_driver_location(driver.id.clone(), location.coordinates)
//                         .await
//                         .unwrap();

//                     tracing::info!("successfully updated driver location");
//                 }
//             });

//             handles.push(handle);
//         }

//         let s = self.s.clone();

//         handles.push(tokio::spawn(async move {
//             loop {
//                 for driver_id in s.driver_ids.lock().await.iter() {
//                     tx.send(driver_id.clone()).await.unwrap();
//                 }

//                 tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
//             }
//         }));

//         futures::future::join_all(handles).await;
//     }

//     #[tracing::instrument(skip(self))]
//     async fn keepalive_trips(&self) {
//         let (tx, rx): (Sender<Uuid>, Receiver<Uuid>) = async_channel::unbounded();

//         let mut handles = vec![];
//         for _ in 0..99 {
//             let rx = rx.clone();

//             let s = self.s.clone();

//             let handle = tokio::spawn(async move {
//                 while let Ok(trip_id) = rx.recv().await {
//                     let trip = s.e.find_trip(trip_id).await.unwrap();

//                     match trip.status {
//                         TripStatus::Searching => {
//                             tracing::info!("requesting driver");
//                             let result = s.e.request_driver(trip.id.clone()).await;
//                             if result.is_err() {
//                                 handle_invocation_error(result);
//                             } else if result.unwrap().is_none() {
//                                 tracing::warn!("no drivers found, attempting to cancel trip");
//                                 handle_invocation_error(
//                                     s.e.cancel_trip(trip.id.clone(), None).await,
//                                 );
//                             } else {
//                                 tracing::info!("successfully requested driver!");
//                             }
//                         }
//                         TripStatus::PendingAssignment {
//                             deadline,
//                             driver_id,
//                             fare: _,
//                         } => {
//                             if Utc::now() >= deadline {
//                                 tracing::info!(
//                                     "driver assign deadline reached, derequesting driver"
//                                 );
//                                 handle_invocation_error(
//                                     s.e.derequest_driver(trip.id.clone(), driver_id, false)
//                                         .await,
//                                 );
//                             }
//                         }
//                         TripStatus::DriverEnRoute { deadline } => {
//                             let mut cancel_probability = 0.2;

//                             if Utc::now() > deadline {
//                                 cancel_probability = 0.8;
//                             }

//                             if sample_binomial(1, cancel_probability) > 0 {
//                                 handle_invocation_error(
//                                     s.e.cancel_trip(
//                                         trip.id.clone(),
//                                         Some(trip.passenger_id.clone()),
//                                     )
//                                     .await,
//                                 );
//                             }
//                         }
//                         TripStatus::Cancelled { penalty_bearer: _ } => {
//                             s.trip_ids.lock().await.remove(&trip.id);
//                         }
//                         _ => (),
//                     }
//                 }
//             });

//             handles.push(handle);
//         }

//         let s = self.s.clone();

//         handles.push(tokio::spawn(async move {
//             while s.trip_ids.lock().await.len() > 0 {
//                 for trip_id in s.trip_ids.lock().await.iter() {
//                     tx.send(trip_id.clone()).await.unwrap();
//                 }

//                 tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
//             }
//         }));

//         futures::future::join_all(handles).await;
//     }
// }
