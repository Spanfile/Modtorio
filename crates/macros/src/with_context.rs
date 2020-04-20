#[macro_export]
macro_rules! with_context {
    ($cont:expr,$inner:block) => {{
        use anyhow::Context;
        Ok($inner.context($cont)?)
    }};
}
