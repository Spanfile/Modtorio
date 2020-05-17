BEGIN TRANSACTION;
DROP TABLE IF EXISTS "mod_release";
CREATE TABLE IF NOT EXISTS "mod_release" (
	"id"	INTEGER PRIMARY KEY AUTOINCREMENT,
	"mod_name"	TEXT NOT NULL,
	"download_url"	TEXT NOT NULL,
	"file_name"	TEXT NOT NULL,
	"released_on"	TEXT NOT NULL,
	"version"	TEXT NOT NULL,
	"sha1"	TEXT NOT NULL,
	"factorio_version"	TEXT NOT NULL
);
DROP TABLE IF EXISTS "game";
CREATE TABLE IF NOT EXISTS "game" (
	"id"	INTEGER PRIMARY KEY AUTOINCREMENT,
	"path"	TEXT NOT NULL
);
DROP TABLE IF EXISTS "release_dependency";
CREATE TABLE IF NOT EXISTS "release_dependency" (
	"id"	INTEGER PRIMARY KEY AUTOINCREMENT,
	"release"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	"requirement"	INTEGER NOT NULL,
	"version_req"	TEXT
);
DROP TABLE IF EXISTS "mod";
CREATE TABLE IF NOT EXISTS "mod" (
	"name"	TEXT,
	"summary"	TEXT,
	"last_updated"	TEXT NOT NULL,
	PRIMARY KEY("name")
);
COMMIT;
