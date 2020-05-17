CREATE TABLE IF NOT EXISTS "mod_release" (
	"id"	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	"mod_name"	TEXT NOT NULL,
	"download_url"	TEXT NOT NULL,
	"file_name"	TEXT NOT NULL,
	"released_on"	TEXT NOT NULL,
	"version"	TEXT NOT NULL,
	"sha1"	TEXT NOT NULL,
	"factorio_version"	TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS "game" (
	"id"	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	"path"	TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS "release_dependency" (
	"id"	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	"release"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	"requirement"	INTEGER NOT NULL,
	"version_req"	TEXT
);
CREATE TABLE IF NOT EXISTS "game_mod" (
	"name"	TEXT NOT NULL,
	"summary"	TEXT,
	"last_updated"	TEXT NOT NULL,
	PRIMARY KEY("name")
);
