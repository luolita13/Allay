-- Add provider column and download summary columns for efficient querying
-- Events are stored inside the `state` JSONB column, no separate table needed.

ALTER TABLE install_jobs ADD COLUMN provider TEXT NOT NULL DEFAULT 'minecraft';
ALTER TABLE install_jobs ADD COLUMN files_total INTEGER NULL;
ALTER TABLE install_jobs ADD COLUMN files_completed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE install_jobs ADD COLUMN bytes_total INTEGER NULL;
ALTER TABLE install_jobs ADD COLUMN bytes_downloaded INTEGER NOT NULL DEFAULT 0;

CREATE INDEX install_jobs_provider ON install_jobs(provider);

-- Add install_job_items table for per-file tracking
CREATE TABLE IF NOT EXISTS install_job_items (
    id TEXT NOT NULL,
    job_id TEXT NOT NULL REFERENCES install_jobs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    project_id TEXT NULL,
    version_id TEXT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    bytes_total INTEGER NULL,
    bytes_downloaded INTEGER NOT NULL DEFAULT 0,
    attempt INTEGER NULL,
    max_attempts INTEGER NULL,
    error TEXT NULL,
    manual_url TEXT NULL,
    created INTEGER NOT NULL,
    modified INTEGER NOT NULL,
    finished INTEGER NULL,
    PRIMARY KEY (job_id, id)
);

CREATE INDEX install_job_items_job_id ON install_job_items(job_id);
CREATE INDEX install_job_items_status ON install_job_items(status);
