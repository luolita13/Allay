-- Add curseforge_source setting to allow independent control of CurseForge download source
-- 0=Mirror (MCIMirror first), 1=Auto (official first, fallback to MCIMirror), 2=Official only
ALTER TABLE settings ADD COLUMN curseforge_source INTEGER NOT NULL DEFAULT 1;
