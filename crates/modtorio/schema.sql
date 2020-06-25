CREATE TABLE IF NOT EXISTS "mod_release" (
	"factorio_mod"	TEXT NOT NULL,
	"download_url"	TEXT NOT NULL,
	"released_on"	TEXT NOT NULL,
	"version"	TEXT NOT NULL,
	"sha1"	TEXT NOT NULL,
	"factorio_version"	TEXT NOT NULL,
	FOREIGN KEY("factorio_mod") REFERENCES "factorio_mod"("name"),
	PRIMARY KEY("factorio_mod","version")
);
CREATE TABLE IF NOT EXISTS "release_dependency" (
	"release_mod_name"	TEXT NOT NULL,
	"release_version"	TEXT NOT NULL,
	"name"	TEXT NOT NULL,
	"requirement"	INTEGER NOT NULL,
	"version_req"	TEXT,
	FOREIGN KEY("release_mod_name","release_version") REFERENCES "mod_release"("factorio_mod","version"),
	PRIMARY KEY("release_mod_name","release_version","name")
);
CREATE TABLE IF NOT EXISTS "game_mod" (
	"game"	INTEGER NOT NULL,
	"factorio_mod"	TEXT NOT NULL,
	FOREIGN KEY("game") REFERENCES "game"("id"),
	FOREIGN KEY("factorio_mod") REFERENCES "factorio_mod"("name"),
	PRIMARY KEY("factorio_mod","game")
);
CREATE TABLE IF NOT EXISTS "factorio_mod" (
	"name"	TEXT NOT NULL,
	"summary"	TEXT,
	"last_updated"	TEXT NOT NULL,
	PRIMARY KEY("name")
);
CREATE TABLE IF NOT EXISTS "game" (
	"id"	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	"path"	TEXT NOT NULL
);
