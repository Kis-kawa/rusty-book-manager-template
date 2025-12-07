-- Add migration script here
-- 1. 'delayed' と 'cancelled' だけの型を作成
CREATE TYPE trip_status AS ENUM ('delayed', 'cancelled');

-- 2. テーブルの型変換 (もしテーブルが既に存在する場合)
-- statusカラムを新しいENUM型に変更します
ALTER TABLE operational_statuses
  ALTER COLUMN status TYPE trip_status
  USING status::trip_status;
