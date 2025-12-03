-- Add migration script here
ALTER TABLE reservations ADD CONSTRAINT unique_user_per_trip UNIQUE (trip_id, user_id);
