use oso::{Oso, PolarClass};

use crate::auth::{Platform, User};
use crate::entities::{Driver, Trip};

pub fn new() -> Oso {
    let mut o = Oso::new();

    o.register_class(Platform::get_polar_class()).unwrap();
    o.register_class(User::get_polar_class()).unwrap();
    o.register_class(Driver::get_polar_class()).unwrap();
    o.register_class(Trip::get_polar_class()).unwrap();

    o.load_str(include_str!("rules.polar")).unwrap();

    o
}

#[test]
fn platform_trip_relation_test() {
    use crate::entities::{Coordinates, Location, Route, Trip};
    use uuid::Uuid;

    let authorizor = new();

    let origin = Location::new(Coordinates { lat: 0.0, lng: 0.0 }, "".into());
    let destination = origin.clone();
    let route = Route::new(origin, destination, serde_json::json!({}), 100.0);
    let trip = Trip::new(Uuid::new_v4(), route, 100.0);

    let result = authorizor.query_rule(
        "has_relation",
        (Platform::default(), "platform", trip.clone()),
    );
    assert!(result.unwrap().next().unwrap().is_ok());
}

#[test]
fn platform_role_test() {
    use uuid::Uuid;

    let authorizor = new();

    let system = User {
        id: Uuid::new_v4(),
        roles: vec!["system".into()],
    };

    let result = authorizor.query_rule("has_role", (system.clone(), "system", Platform::default()));
    assert!(result.unwrap().next().unwrap().is_ok());
}

#[test]
fn trip_passenger_role_test() {
    use crate::entities::{Coordinates, Location, Route, Trip};
    use uuid::Uuid;

    let authorizor = new();

    let passenger = User {
        id: Uuid::new_v4(),
        roles: vec![],
    };

    let origin = Location::new(Coordinates { lat: 0.0, lng: 0.0 }, "".into());
    let destination = origin.clone();
    let route = Route::new(origin, destination, serde_json::json!({}), 100.0);
    let trip = Trip::new(passenger.id.clone(), route, 100.0);

    let result = authorizor.query_rule("has_role", (passenger.clone(), "passenger", trip.clone()));
    assert!(result.unwrap().next().unwrap().is_ok());

    let result = authorizor.is_allowed(passenger.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(passenger.clone(), "cancel", trip.clone());
    assert_eq!(result.unwrap(), true);
}

#[test]
fn trip_driver_candidate_and_driver_role_test() {
    use crate::entities::{Coordinates, Location, Route, Trip};
    use uuid::Uuid;

    let authorizor = new();

    let driver = User {
        id: Uuid::new_v4(),
        roles: vec![],
    };

    let origin = Location::new(Coordinates { lat: 0.0, lng: 0.0 }, "".into());
    let destination = origin.clone();
    let route = Route::new(origin, destination, serde_json::json!({}), 100.0);
    let mut trip = Trip::new(Uuid::new_v4(), route, 100.0);

    // before driver is requested

    let result = authorizor.query_rule(
        "has_role",
        (driver.clone(), "driver_candidate", trip.clone()),
    );
    assert!(result.unwrap().next().is_none());

    let result = authorizor.query_rule("has_role", (driver.clone(), "driver", trip.clone()));
    assert!(result.unwrap().next().is_none());

    let result = authorizor.is_allowed(driver.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(driver.clone(), "accept", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(driver.clone(), "reject", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(driver.clone(), "cancel", trip.clone());
    assert_eq!(result.unwrap(), false);

    trip.request_driver(driver.id.clone(), 100.0).unwrap();

    // after driver is requested and before driver is assigned

    let result = authorizor.query_rule(
        "has_role",
        (driver.clone(), "driver_candidate", trip.clone()),
    );
    assert!(result.unwrap().next().unwrap().is_ok());

    let result = authorizor.query_rule("has_role", (driver.clone(), "driver", trip.clone()));
    assert!(result.unwrap().next().is_none());

    let result = authorizor.is_allowed(driver.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(driver.clone(), "accept", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(driver.clone(), "reject", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(driver.clone(), "cancel", trip.clone());
    assert_eq!(result.unwrap(), false);

    trip.assign_driver().unwrap();

    // after driver is assigned

    let result = authorizor.query_rule(
        "has_role",
        (driver.clone(), "driver_candidate", trip.clone()),
    );
    assert!(result.unwrap().next().is_none());

    let result = authorizor.query_rule("has_role", (driver.clone(), "driver", trip.clone()));
    assert!(result.unwrap().next().unwrap().is_ok());

    let result = authorizor.is_allowed(driver.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(driver.clone(), "accept", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(driver.clone(), "reject", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(driver.clone(), "cancel", trip.clone());
    assert_eq!(result.unwrap(), true);
}

#[test]
fn trip_system_role_test() {
    use crate::entities::{Coordinates, Location, Route, Trip};
    use uuid::Uuid;

    let authorizor = new();

    let unprivileged = User {
        id: Uuid::new_v4(),
        roles: vec![],
    };

    let system = User {
        id: Uuid::new_v4(),
        roles: vec!["system".into()],
    };

    let origin = Location::new(Coordinates { lat: 0.0, lng: 0.0 }, "".into());
    let destination = origin.clone();
    let route = Route::new(origin, destination, serde_json::json!({}), 100.0);
    let mut trip = Trip::new(Uuid::new_v4(), route, 100.0);

    // before request driver

    let result = authorizor.query_rule("has_role", (unprivileged.clone(), "system", trip.clone()));
    assert!(result.unwrap().next().is_none());

    let result = authorizor.query_rule("has_role", (system.clone(), "system", trip.clone()));
    assert!(result.unwrap().next().unwrap().is_ok());

    let result = authorizor.is_allowed(unprivileged.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(unprivileged.clone(), "request_driver", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(unprivileged.clone(), "release_driver", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(system.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(system.clone(), "request_driver", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(system.clone(), "release_driver", trip.clone());
    assert_eq!(result.unwrap(), true);

    trip.request_driver(Uuid::new_v4(), 100.0).unwrap();

    // after request driver

    let result = authorizor.query_rule("has_role", (unprivileged.clone(), "system", trip.clone()));
    assert!(result.unwrap().next().is_none());

    let result = authorizor.query_rule("has_role", (system.clone(), "system", trip.clone()));
    assert!(result.unwrap().next().unwrap().is_ok());

    let result = authorizor.is_allowed(unprivileged.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(unprivileged.clone(), "request_driver", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(unprivileged.clone(), "release_driver", trip.clone());
    assert_eq!(result.unwrap(), false);

    let result = authorizor.is_allowed(system.clone(), "read", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(system.clone(), "request_driver", trip.clone());
    assert_eq!(result.unwrap(), true);

    let result = authorizor.is_allowed(system.clone(), "release_driver", trip.clone());
    assert_eq!(result.unwrap(), true);
}
