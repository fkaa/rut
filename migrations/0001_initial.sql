CREATE TABLE users (
    id INTEGER PRIMARY KEY NOT NULL,
    username TEXT NOT NULL COLLATE NOCASE,
    password TEXT NOT NULL
) STRICT;

CREATE TABLE categories (
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    rules TEXT NOT NULL,
    is_public INTEGER NOT NULL,

    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE entries (
    id INTEGER PRIMARY KEY NOT NULL,
    category_id INTEGER NOT NULL,
    time INTEGER NOT NULL,
    value TEXT NOT NULL,

    FOREIGN KEY(category_id) REFERENCES categories(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_entry_category ON entries (category_id);
CREATE INDEX idx_entry_time ON entries (time);

INSERT INTO users (id, username, password) VALUES (0, 'tmtu', '00YWVBDdaaLD5DWlgrFnyTEEXbU7GETt6QIC3Y7zVh0=');
INSERT INTO categories (id, user_id, rules, name, is_public) VALUES (1, 0, 'method=sum;style=weekly-sum;from-col:#c5ffc2;to-col:#10bf00', 'pull-ups', 1);
INSERT INTO categories (id, user_id, rules, name, is_public) VALUES (2, 0, '','no-hangs', 1);
INSERT INTO categories (id, user_id, rules, name, is_public) VALUES (3, 0, '','bouldering', 1);
INSERT INTO entries (category_id, time, value) VALUES (1, 1704150000000, '10');
INSERT INTO entries (category_id, time, value) VALUES (1, 1704236400000, '20');
INSERT INTO entries (category_id, time, value) VALUES (1, 1704382812039, '24');
INSERT INTO entries (category_id, time, value) VALUES (2, 1704063600000, 'y');
INSERT INTO entries (category_id, time, value) VALUES (2, 1704150000000, 'y');
INSERT INTO entries (category_id, time, value) VALUES (2, 1704236400000, 'y');
INSERT INTO entries (category_id, time, value) VALUES (3, 1704150000000, 'y');
