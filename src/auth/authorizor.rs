use oso::{Oso, PolarClass};

use crate::auth::{Platform, User};
use crate::entities::{Driver, Trip};

pub fn new() -> Oso {
    let mut o = Oso::new();

    o.register_class(Platform::get_polar_class()).unwrap();
    o.register_class(User::get_polar_class()).unwrap();
    o.register_class(Driver::get_polar_class()).unwrap();
    o.register_class(Trip::get_polar_class()).unwrap();

    o
    .load_str(
        "
            allow(actor, action, resource) if
                has_permission(actor, action, resource);

            actor User {}

            resource Platform {
                permissions = [\"create_member\", \"create_passenger\", \"create_driver\", \"create_trip\"];
                roles = [\"anonymous\", \"member\", \"passenger\", \"driver\", \"system\"];

                \"create_member\" if \"anonymous\";

                \"create_passenger\" if \"member\";
                \"create_driver\" if \"member\";

                \"create_trip\" if \"passenger\";
            }

            has_role(user: User, name: String, platform: Platform) if
                user.has_role(name) and
                role in user.roles and
                role = name and
                platform.id = Platform.default().id;

            resource Trip {
                permissions = [\"read\", \"request_driver\", \"release_driver\", \"accept\", \"reject\", \"cancel\"];
                roles = [\"passenger\", \"driver_candidate\", \"driver\", \"system\"];
                relations = { platform: Platform };

                \"read\" if \"passenger\";
                \"cancel\" if \"passenger\";
                
                \"read\" if \"driver_candidate\";
                \"accept\" if \"driver_candidate\";
                \"reject\" if \"driver_candidate\";

                \"read\" if \"driver\";
                \"cancel\" if \"driver\";

                \"read\" if \"system\";
                \"request_driver\" if \"system\";
                \"release_driver\" if \"system\";
            }

            has_relation(platform: Platform, \"platform\", _: Trip) if
                platform.id = Platform.default().id;

            has_role(user: User, \"passenger\", trip: Trip) if
                user.id = trip.passenger_id;

            has_role(user: User, \"driver_candidate\", trip: Trip) if
                trip.status.name = \"PENDING_ASSIGNMENT\" and
                user.id_equals_nullable_id(trip.status.driver_id);

            has_role(user: User, \"driver\", trip: Trip) if
                user.id_equals_nullable_id(trip.driver_id);

            has_role(user: User, \"system\", trip: Trip) if
                has_role(user, \"system\", Platform.default()) and
                has_relation(Platform.default(), \"platform\", trip);
        ",
    )
    .unwrap();

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
