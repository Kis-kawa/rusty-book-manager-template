-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TYPE user_role AS ENUM ('student', 'teacher', 'admin');

CREATE TABLE users (
    user_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name           TEXT NOT NULL,
    email          TEXT UNIQUE NOT NULL,
    phone_number   TEXT,
    password       TEXT NOT NULL,
    role           user_role NOT NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT NOW(),
    is_deleted     BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE bus_stops (
    bus_stop_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    bus_stop_number TEXT NOT NULL
);

CREATE TABLE routes (
    route_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_bus_stop_id UUID REFERENCES bus_stops(bus_stop_id),
    destination_bus_stop_id UUID REFERENCES bus_stops(bus_stop_id)
);

CREATE TABLE vehicle_types (
    vehicle_type_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    maker TEXT NOT NULL,
    name TEXT NOT NULL,
    total_seats INT NOT NULL
);


CREATE TABLE vehicles (
    vehicle_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vehicle_type_id UUID REFERENCES vehicle_types(vehicle_type_id),
    plate_number TEXT NOT NULL,
    vehicle_name TEXT NOT NULL
);


CREATE TABLE drivers (
    driver_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL
);

CREATE TABLE trips (
    trip_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    route_id UUID REFERENCES routes(route_id),
    vehicle_id UUID REFERENCES vehicles(vehicle_id),
    driver_id UUID REFERENCES drivers(driver_id),
    trip_date DATE NOT NULL,
    departure_datetime TIMESTAMP NOT NULL,
    arrival_datetime TIMESTAMP NOT NULL
);


CREATE TABLE operational_statuses (
    status_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trip_id UUID REFERENCES trips(trip_id),
    status TEXT NOT NULL,
    description TEXT,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);


CREATE TABLE reservations (
    reservation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trip_id UUID REFERENCES trips(trip_id),
    seat_number INT NOT NULL,
    user_id UUID REFERENCES users(user_id),
    UNIQUE (trip_id, seat_number)
);
