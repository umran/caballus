allow(actor, action, resource) if
    has_permission(actor, action, resource);

actor User {}

has_role(user: User, "system", _: Resource) if
    user.has_role("system");

resource Member {
    permissions = ["read", "update_profile", "create_passenger", "create_driver"];
    roles = ["owner", "system"];

    "read" if "owner";
    "update_profile" if "owner";
    "create_passenger" if "owner";
    "create_driver" if "owner";

    "read" if "system";
}

has_role(user: User, "owner", member: Member) if
    user.id = member.id;

resource Passenger {
    permissions = ["read", "create_trip"];
    roles = ["owner", "system"];

    "read" if "owner";
    "create_trip" if "owner";

    "read" if "system";
}

has_role(user: User, "owner", passenger: Passenger) if
    user.id = passenger.id;

resource Driver {
    permissions = [
        "read",
        "start",
        "stop",
        "update_rate",
        "update_location",

        # to be implemented
        "request_verification",
        "verify",
        "suspend",
        "unsuspend"
    ];
    
    roles = ["owner", "system"];

    "read" if "owner";
    "start" if "owner";
    "stop" if "owner";
    "update_rate" if "owner";
    "update_location" if "owner";
    "request_verification" if "owner";

    "read" if "system";
    "verify" if "system";
    "suspend" if "system";
    "unsuspend" if "system";
}

has_role(user: User, "owner", driver: Driver) if
    user.id = driver.id;

resource Trip {
    permissions = [
        "read",
        "request_driver",
        "release_driver",
        "accept",
        "reject",
        "cancel",
        "report_origin_arrival",
        "report_destination_arrival"
    ];
    
    roles = ["passenger", "driver_candidate", "driver", "system"];

    "read" if "passenger";
    "cancel" if "passenger";
    
    "read" if "driver_candidate";
    "accept" if "driver_candidate";
    "reject" if "driver_candidate";

    "read" if "driver";
    "cancel" if "driver";
    "report_origin_arrival" if "driver";
    "report_destination_arrival" if "driver";

    "read" if "system";
    "request_driver" if "system";
    "release_driver" if "system";
}

has_role(user: User, "passenger", trip: Trip) if
    user.id = trip.passenger_id;

has_role(user: User, "driver_candidate", trip: Trip) if
    trip.status.name = "pending_assignment" and
    user.id_equals_nullable_id(trip.status.driver_id);

has_role(user: User, "driver", trip: Trip) if
    user.id_equals_nullable_id(trip.driver_id);