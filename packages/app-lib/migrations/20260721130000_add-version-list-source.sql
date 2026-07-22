-- Add version_list_source setting to separate version list sources from file sources
-- 0=Mirror (BMCLAPI first), 1=Auto (official first, fallback to BMCLAPI), 2=Official only
ALTER TABLE settings ADD COLUMN version_list_source INTEGER NOT NULL DEFAULT 1;
