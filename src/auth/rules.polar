allow(actor, action, resource) if
    has_permission(actor, action, resource);

actor User {}

resource Platform {
    permissions = ["create_member", "create_passenger", "create_driver", "create_trip"];
    roles = ["anonymous", "member", "passenger", "driver", "system"];

    "create_member" if "anonymous";

    "create_passenger" if "member";
    "create_driver" if "member";

    "create_trip" if "passenger";
}

has_role(user: User, role: String, platform: Platform) if
    user.has_role(role) and
    platform.id = Platform.default().id;

resource Trip {
    permissions = ["read", "request_driver", "release_driver", "accept", "reject", "cancel", "report_origin_arrival", "report_destination_arrival"];
    roles = ["passenger", "driver_candidate", "driver", "system"];
    relations = { platform: Platform };

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

has_relation(platform: Platform, "platform", _: Trip) if
    platform.id = Platform.default().id;

has_role(user: User, "passenger", trip: Trip) if
    user.id = trip.passenger_id;

has_role(user: User, "driver_candidate", trip: Trip) if
    trip.status.name = "PENDING_ASSIGNMENT" and
    user.id_equals_nullable_id(trip.status.driver_id);

has_role(user: User, "driver", trip: Trip) if
    user.id_equals_nullable_id(trip.driver_id);

has_role(user: User, "system", trip: Trip) if
    has_role(user, "system", Platform.default()) and
    has_relation(Platform.default(), "platform", trip);

# resource Member {
#     permissions = [];
#     roles = ["owner", "system"];
# }

# resource Passenger {
#     permissions = [];
#     roles = ["owner", "system"];
# }

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
    relations = { platform: Platform };

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

has_relation(platform: Platform, "platform", _: Driver) if
    platform.id = Platform.default().id;

has_role(user: User, "owner", driver: Driver) if
    user.id = driver.id;

has_role(user: User, "system", driver: Driver) if
    has_role(user, "system", Platform.default()) and
    has_relation(Platform.default(), "platform", driver);