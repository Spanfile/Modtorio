DROP TABLE IF EXISTS "_meta";
CREATE TABLE IF NOT EXISTS "_meta" (
	"field"	TEXT NOT NULL,
	"value"	TEXT,
	PRIMARY KEY("field")
);

DROP TABLE IF EXISTS "factorio_mod";
CREATE TABLE IF NOT EXISTS "factorio_mod" (
	"name"	TEXT NOT NULL,
	"author"	TEXT NOT NULL,
	"contact"	TEXT,
	"homepage"	TEXT,
	"title"	TEXT NOT NULL,
	"summary"	TEXT,
	"description"	TEXT NOT NULL,
	"changelog"	TEXT,
	"last_updated"	TEXT NOT NULL,
	PRIMARY KEY("name")
);

DROP TABLE IF EXISTS "game";
CREATE TABLE IF NOT EXISTS "game" (
	"id"	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	"path"	TEXT NOT NULL
);

DROP TABLE IF EXISTS "game_mod";
CREATE TABLE IF NOT EXISTS "game_mod" (
	"game"	INTEGER NOT NULL,
	"factorio_mod"	TEXT NOT NULL,
	"mod_version"	TEXT NOT NULL,
	"mod_zip"	TEXT UNIQUE NOT NULL,
	"zip_checksum"	TEXT NOT NULL,
	PRIMARY KEY("game","factorio_mod"),
	FOREIGN KEY("factorio_mod") REFERENCES "factorio_mod"("name"),
	FOREIGN KEY("factorio_mod", "mod_version") REFERENCES "mod_release"("factorio_mod", "version"),
	FOREIGN KEY("game") REFERENCES "game"("id")
);

DROP TABLE IF EXISTS "release_dependency";
CREATE TABLE IF NOT EXISTS "release_dependency" (
	"release_mod_name"	TEXT NOT NULL,
	"release_version"	TEXT NOT NULL,
	"name"	TEXT NOT NULL,
	"requirement"	INTEGER NOT NULL,
	"version_req"	TEXT,
	PRIMARY KEY("release_mod_name","release_version","name"),
	FOREIGN KEY("release_mod_name","release_version") REFERENCES "mod_release"("factorio_mod","version")
);

DROP TABLE IF EXISTS "mod_release";
CREATE TABLE IF NOT EXISTS "mod_release" (
	"factorio_mod"	TEXT NOT NULL,
	"version"	TEXT NOT NULL,
	"download_url"	TEXT NOT NULL,
	"released_on"	TEXT NOT NULL,
	"sha1"	TEXT NOT NULL,
	"factorio_version"	TEXT NOT NULL,
	PRIMARY KEY("factorio_mod","version"),
	FOREIGN KEY("factorio_mod") REFERENCES "factorio_mod"("name")
);
