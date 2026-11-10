-- A signed in user who adds a second provider. The app asks for the link with
-- its session before it opens the provider, and the login that comes back for
-- this challenge then lands in that user, see `store::link_pending`.
ALTER TABLE pending_logins ADD COLUMN link_user UUID REFERENCES users (id) ON DELETE CASCADE;
