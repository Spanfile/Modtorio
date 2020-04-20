#[macro_export]
macro_rules! with_context {
    ($cont:expr, $ret:ty: $fn:block) => {{
        use anyhow::Context;
        || -> anyhow::Result<$ret> { $fn }().context($cont)
    }};
}
