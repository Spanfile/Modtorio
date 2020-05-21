table! {
    factorio_mod (name) {
        name -> Text,
        summary -> Nullable<Text>,
        last_updated -> Text,
    }
}

table! {
    game (id) {
        id -> Integer,
        path -> Text,
    }
}

table! {
    game_mod (game, factorio_mod) {
        game -> Integer,
        factorio_mod -> Text,
    }
}

table! {
    mod_release (factorio_mod, version) {
        factorio_mod -> Text,
        download_url -> Text,
        file_name -> Text,
        released_on -> Text,
        version -> Text,
        sha1 -> Text,
        factorio_version -> Text,
    }
}

table! {
    release_dependency (release_mod_name, release_version, name) {
        release_mod_name -> Text,
        release_version -> Text,
        name -> Text,
        requirement -> Integer,
        version_req -> Nullable<Text>,
    }
}

joinable!(game_mod -> factorio_mod (factorio_mod));
joinable!(game_mod -> game (game));
joinable!(mod_release -> factorio_mod (factorio_mod));

allow_tables_to_appear_in_same_query!(
    factorio_mod,
    game,
    game_mod,
    mod_release,
    release_dependency,
);
