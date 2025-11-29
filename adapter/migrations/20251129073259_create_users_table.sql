-- Add migration script here
-- 既存データのクリア
DELETE FROM reservations;
DELETE FROM operational_statuses;
DELETE FROM trips;
DELETE FROM drivers;
DELETE FROM routes;
DELETE FROM vehicles;
DELETE FROM vehicle_types;
DELETE FROM bus_stops;

-- 1. バス停
INSERT INTO bus_stops (bus_stop_id, name, bus_stop_number) VALUES
('11111111-1111-1111-1111-111111111111', '品川キャンパス', 'No.1'),
('22222222-2222-2222-2222-222222222222', '荒川キャンパス', 'No.1');

-- 2. ルート
INSERT INTO routes (route_id, source_bus_stop_id, destination_bus_stop_id) VALUES
('33333333-3333-3333-3333-333333333333', '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222'),
('44444444-4444-4444-4444-444444444444', '22222222-2222-2222-2222-222222222222', '11111111-1111-1111-1111-111111111111');

-- 3. 車両タイプ
INSERT INTO vehicle_types (vehicle_type_id, maker, name, total_seats) VALUES
('55555555-5555-5555-5555-555555555555', 'Toyota', 'Coaster', 20);

-- 4. 車両
INSERT INTO vehicles (vehicle_id, vehicle_type_id, plate_number, vehicle_name) VALUES
('66666666-6666-6666-6666-666666666666', '55555555-5555-5555-5555-555555555555', '品川 500 あ 1234', '産技号 1');

-- 5. 運転手
INSERT INTO drivers (driver_id, name) VALUES
('77777777-7777-7777-7777-777777777777', '鈴木 一郎');

-- 6. 便/トリップ (明日の運行予定)
-- 品川発 -> 荒川行き (10:00発)
INSERT INTO trips (trip_id, route_id, vehicle_id, driver_id, trip_date, departure_datetime, arrival_datetime) VALUES
('88888888-8888-8888-8888-888888888888',
 '33333333-3333-3333-3333-333333333333',
 '66666666-6666-6666-6666-666666666666',
 '77777777-7777-7777-7777-777777777777',
 CURRENT_DATE + INTERVAL '1 day',
 CURRENT_DATE + INTERVAL '1 day' + INTERVAL '10 hours',
 CURRENT_DATE + INTERVAL '1 day' + INTERVAL '11 hours'
);

-- 荒川発 -> 品川行き (16:00発)
INSERT INTO trips (trip_id, route_id, vehicle_id, driver_id, trip_date, departure_datetime, arrival_datetime) VALUES
('99999999-9999-9999-9999-999999999999',
 '44444444-4444-4444-4444-444444444444',
 '66666666-6666-6666-6666-666666666666',
 '77777777-7777-7777-7777-777777777777',
 CURRENT_DATE + INTERVAL '1 day',
 CURRENT_DATE + INTERVAL '1 day' + INTERVAL '16 hours',
 CURRENT_DATE + INTERVAL '1 day' + INTERVAL '17 hours'
);
