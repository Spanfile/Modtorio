#[derive(Debug)]
pub struct Config {
    pub portal: PortalConfig,
}

#[derive(Debug)]
pub struct PortalConfig {
    pub username: String,
    pub token: String,
}
