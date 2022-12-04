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

has_role(user: User, name: String, _: Platform) if
    role in user.roles and
    role = name;

resource Trip {
    permissions = ["read", "request_driver", "release_driver", "accept", "reject", "cancel"];
    roles = ["passenger", "driver_candidate", "driver", "system"];
    relations = { parent: Platform };

    "read" if "passenger";
    "cancel" if "passenger";
    
    "read" if "driver_candidate";
    "accept" if "driver_candidate";
    "reject" if "driver_candidate";

    "read" if "driver";
    "cancel" if "driver";

    "read" if "system";
    "request_driver" if "system";
    "release_driver" if "system";

    "system" if "system" on "parent";
}

has_relation(_: Platform, "parent", _: Trip);

has_role(user: User, "passenger", trip: Trip) if
    user.id = trip.passenger_id;

has_role(user: User, "driver_candidate", trip: Trip) if
    trip.status.name = "PENDING_ASSIGNMENT" and
    user.id_equals_nullable_id(trip.status.driver_id);

has_role(user: User, "driver", trip: Trip) if
    user.id_equals_nullable_id(trip.driver_id);
        