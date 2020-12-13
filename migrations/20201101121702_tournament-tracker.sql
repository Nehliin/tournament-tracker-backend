-- Add migration script here
-- How I start the local server: https://www.microfocus.com/documentation/idol/IDOL_12_0/MediaServer/Guides/html/English/Content/Getting_Started/Configure/_TRN_Set_up_PostgreSQL_Linux.htm
-- sudo -u postgres psql postgres
-- choose password
-- sudo su - postgres
-- createuser --interactive --pwprompt
-- createdb -O user dbname
-- switch back to your user
-- psql -u username -d dbname  (skip -u if same username as OS user)

-- Your SQL goes here
CREATE TABLE IF NOT EXISTS tournaments (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  start_date DATE NOT NULL,
  end_date DATE NOT NULL CHECK (start_date <= end_date)
);


CREATE TABLE IF NOT EXISTS players (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS matches (
   id SERIAL8 PRIMARY KEY,
   player_one BIGINT NOT NULL,
   player_two BIGINT NOT NULL,
   tournament_id INTEGER NOT NULL,
   class TEXT NOT NULL, -- THIS MUST BE CORRECTLY SET BY USER
   start_time TIMESTAMP NOT NULL,
   CONSTRAINT valid_tournament 
        FOREIGN KEY(tournament_id)
            REFERENCES tournaments(id)
            ON DELETE CASCADE,
    CONSTRAINT valid_players
        FOREIGN KEY(player_one)
            REFERENCES players(id)
            ON DELETE CASCADE,
        FOREIGN KEY(player_two)
            REFERENCES players(id)
            ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS court_queue (
   place_in_queue  TIMESTAMP NOT NULL,
   match_id BIGINT NOT NULL,
   tournament_id INT NOT NULL,
   PRIMARY KEY (tournament_id, match_id),
   CONSTRAINT valid_match
        FOREIGN KEY(match_id)
            REFERENCES matches(id)
            ON DELETE CASCADE, -- delete queue placement if match is removed
    CONSTRAINT valid_tournament
        FOREIGN KEY(tournament_id)
            REFERENCES tournaments(id)
            ON DELETE CASCADE -- delete queue placement if tournament is removed

);

CREATE TABLE IF NOT EXISTS tournament_court_allocation (
    court_name TEXT NOT NULL,
    tournament_id INTEGER NOT NULL,
    match_id BIGINT, -- null indicates it's free
    PRIMARY KEY (court_name, tournament_id),
    CONSTRAINT match_allocation
        FOREIGN KEY(match_id)
            REFERENCES matches(id)
            ON DELETE SET NULL, -- free up the court if match is deleted
    CONSTRAINT valid_tournament
        FOREIGN KEY(tournament_id)
            REFERENCES tournaments(id)
            ON DELETE CASCADE -- delete row if tournament is deleted
);

CREATE TABLE IF NOT EXISTS match_result (
    match_id BIGINT NOT NULL,
    result TEXT NOT NULL,
    winner BIGINT NOT NULL,
    PRIMARY KEY(match_id),
    CONSTRAINT valid_match
        FOREIGN KEY(match_id)
            REFERENCES matches(id)
            ON DELETE CASCADE,
    CONSTRAINT valid_winner
        FOREIGN KEY(winner)
            REFERENCES players(id)
            ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS register (
    player_id BIGINT NOT NULL,
    match_id BIGINT NOT NULL,
    time_registerd TIMESTAMP NOT NULL,
    registerd_by TEXT NOT NULL,
    PRIMARY KEY (player_id, match_id),
    CONSTRAINT valid_match
        FOREIGN KEY(match_id)
            REFERENCES matches(id)
            ON DELETE CASCADE,
    CONSTRAINT valid_player
        FOREIGN KEY(player_id)
            REFERENCES players(id)
            ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS users (
    email TEXT PRIMARY KEY,
    password TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL
);