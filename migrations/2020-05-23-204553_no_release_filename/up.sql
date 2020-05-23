CREATE TABLE mod_release_backup (
    factorio_mod TEXT NOT NULL,
    download_url TEXT NOT NULL,
    released_on TEXT NOT NULL,
    version TEXT NOT NULL,
    sha1 TEXT NOT NULL,
    factorio_version TEXT NOT NULL,
    PRIMARY KEY (factorio_mod, version)
);

INSERT INTO mod_release_backup SELECT factorio_mod, download_url, released_on, version, sha1, factorio_version FROM mod_release;
DROP TABLE mod_release;

CREATE TABLE mod_release (
    factorio_mod TEXT NOT NULL,
    download_url TEXT NOT NULL,
    released_on TEXT NOT NULL,
    version TEXT NOT NULL,
    sha1 TEXT NOT NULL,
    factorio_version TEXT NOT NULL,
    PRIMARY KEY (factorio_mod, version),
    FOREIGN KEY (factorio_mod) REFERENCES factorio_mod(name)
);

INSERT INTO mod_release SELECT factorio_mod, download_url, released_on, version, sha1, factorio_version FROM mod_release_backup;
DROP TABLE mod_release_backup;
