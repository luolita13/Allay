-- Add max_chunks_per_file setting for chunked (parallel Range) downloads
-- Default 8 chunks per file, matching PCL-CE's multi-thread download behavior
ALTER TABLE settings ADD COLUMN max_chunks_per_file INTEGER NOT NULL DEFAULT 8;
