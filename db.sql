-- schema
CREATE TABLE IF NOT EXISTS messages (
  id serial PRIMARY KEY,
  message text NOT NULL
);

-- add a message
INSERT INTO messages (id, message) VALUES (1, 'hello world from Database!') ON CONFLICT (id) DO NOTHING;
