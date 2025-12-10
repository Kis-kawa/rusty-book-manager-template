-- Add migration script here
ALTER TABLE trips ADD COLUMN notification_sent BOOLEAN NOT NULL DEFAULT FALSE;
