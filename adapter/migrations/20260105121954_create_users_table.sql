-- Add migration script here
-- 設定保存用テーブル
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- 初期値として 'maintenance_mode' を 'false' (OFF) で登録
INSERT INTO app_settings (key, value) VALUES ('maintenance_mode', 'false');
