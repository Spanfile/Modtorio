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
    game_mod (id) {
        id -> Integer,
        mod_name -> Text,
        game -> Integer,
    }
}

table! {
    mod_release (id) {
        id -> Integer,
        mod_name -> Text,
        download_url -> Text,
        file_name -> Text,
        released_on -> Text,
        version -> Text,
        sha1 -> Text,
        factorio_version -> Text,
    }
}

table! {
    release_dependency (id) {
        id -> Integer,
        release -> Integer,
        name -> Text,
        requirement -> Integer,
        version_req -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    factorio_mod,
    game,
    game_mod,
    mod_release,
    release_dependency,
);
