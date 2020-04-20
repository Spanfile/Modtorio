#[macro_export]
macro_rules! field_deserializers {
    ( $map:ident, $([$name:ident, $type:ty, $field:ident]),+) => {
        $(
            let mut $name: Option<$type> = None;
        )*

        while let Some(key) = $map.next_key()? {
            match key {
                $(
                Field::$field => {
                    if $name.is_some() {
                        return Err(de::Error::duplicate_field("$name"));
                    }
                    $name = Some($map.next_value()?);
                }
                )*
            }
        }

        $(
            let $name = $name.ok_or_else(|| de::Error::missing_field("$name"))?;
        )*
    };
}
