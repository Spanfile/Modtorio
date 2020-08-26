CREATE TABLE "options" (
	"field" TEXT NOT NULL,
	"value" TEXT,
	PRIMARY KEY("field")
);

CREATE TABLE "factorio_mod" (
	"name" TEXT NOT NULL,
	"author" TEXT NOT NULL,
	"contact" TEXT,
	"homepage" TEXT,
	"title" TEXT NOT NULL,
	"summary" TEXT,
	"description" TEXT NOT NULL,
	"changelog" TEXT,
	"last_updated" TEXT NOT NULL,
	PRIMARY KEY("name")
);

CREATE TABLE "game" (
	/* without AUTOINCREMENT, an integer primary key is aliased to SQLite's internal ROWID which functions better as a primary key and than autoincremented one */
	"id" INTEGER PRIMARY KEY,
	"path" TEXT NOT NULL,
	"settings_file" TEXT,
	"whitelist_file" TEXT,
	"banlist_file" TEXT,
	"adminlist_file" TEXT
);

CREATE TABLE "game_settings" (
	"game" INTEGER PRIMARY KEY,
	"file_last_mtime" TEXT NOT NULL,
	"name" TEXT NOT NULL,
	"description" TEXT NOT NULL,
	/* yeah yeah it's not very normalized to store the tags as just values separated with some separator but fuck it */
	"tags" TEXT NOT NULL,
	"max_players" INTEGER NOT NULL,
	"public_visibility" INTEGER NOT NULL,
	"lan_visibility" INTEGER NOT NULL,
	"username" TEXT NOT NULL,
	"password" TEXT NOT NULL,
	"token" TEXT NOT NULL,
	"game_password" TEXT NOT NULL,
	"require_user_verification" INTEGER NOT NULL,
	"max_upload_in_kilobytes_per_second" INTEGER NOT NULL,
	"max_upload_slots" INTEGER NOT NULL,
	"minimum_latency_in_ticks" INTEGER NOT NULL,
	"ignore_player_limit_for_returning_players" INTEGER NOT NULL,
	"allow_commands" TEXT NOT NULL,
	"autosave_interval" INTEGER NOT NULL,
	"autosave_slots" INTEGER NOT NULL,
	"afk_autokick_interval" INTEGER NOT NULL,
	"auto_pause" INTEGER NOT NULL,
	"only_admins_can_pause_the_game" INTEGER NOT NULL,
	"autosave_only_on_server" INTEGER NOT NULL,
	"non_blocking_saving" INTEGER NOT NULL,
	"minimum_segment_size" INTEGER NOT NULL,
	"minimum_segment_size_peer_count" INTEGER NOT NULL,
	"maximum_segment_size" INTEGER NOT NULL,
	"maximum_segment_size_peer_count" INTEGER NOT NULL,
	"bind_address_ip_version" INTEGER NOT NULL,
	"bind_address" BLOB NOT NULL,
	"bind_port" INTEGER NOT NULL,
	"save_name" TEXT NOT NULL,
	"start_behaviour" TEXT NOT NULL,
	"auto_start" INTEGER NOT NULL,
	"graceful_shutdown_timeout" INTEGER NOT NULL,
	FOREIGN KEY("game") REFERENCES "game"("id")
);

CREATE TABLE "game_mod" (
	"game" INTEGER NOT NULL,
	"factorio_mod" TEXT NOT NULL,
	"mod_version" TEXT NOT NULL,
	"mod_zip" TEXT NOT NULL,
	"zip_last_mtime" TEXT NOT NULL,
	PRIMARY KEY("game","factorio_mod"),
	FOREIGN KEY("factorio_mod") REFERENCES "factorio_mod"("name"),
	FOREIGN KEY("factorio_mod", "mod_version") REFERENCES "mod_release"("factorio_mod", "version"),
	FOREIGN KEY("game") REFERENCES "game"("id")
);

CREATE TABLE "release_dependency" (
	"release_mod_name" TEXT NOT NULL,
	"release_version" TEXT NOT NULL,
	"name" TEXT NOT NULL,
	"requirement" INTEGER NOT NULL,
	"version_req" TEXT,
	PRIMARY KEY("release_mod_name","release_version","name"),
	FOREIGN KEY("release_mod_name","release_version") REFERENCES "mod_release"("factorio_mod","version")
);

CREATE TABLE "mod_release" (
	"factorio_mod" TEXT NOT NULL,
	"version" TEXT NOT NULL,
	"download_url" TEXT NOT NULL,
	"released_on" TEXT NOT NULL,
	"sha1" TEXT NOT NULL,
	"factorio_version" TEXT NOT NULL,
	PRIMARY KEY("factorio_mod","version"),
	FOREIGN KEY("factorio_mod") REFERENCES "factorio_mod"("name")
);