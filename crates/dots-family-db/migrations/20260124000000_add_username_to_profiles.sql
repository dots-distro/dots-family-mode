-- Add username column to profiles table for system user mapping
-- This allows proper session locking by mapping profiles to actual system usernames

-- Add username column (nullable for backward compatibility)
ALTER TABLE profiles ADD COLUMN username TEXT;

-- Create index for username lookups
CREATE INDEX idx_profiles_username ON profiles(username);

-- Update the unique constraint to include username if we want to ensure uniqueness
-- Note: username can be NULL for legacy profiles, but should be unique when set
-- CREATE UNIQUE INDEX idx_profiles_username_unique ON profiles(username) WHERE username IS NOT NULL;
