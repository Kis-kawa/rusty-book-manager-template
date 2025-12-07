-- Add migration script here
ALTER TABLE operational_statuses ADD CONSTRAINT unique_trip_status UNIQUE (trip_id);
