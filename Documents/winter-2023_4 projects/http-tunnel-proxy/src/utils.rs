pub struct CLIConfig {
    pub name: String,
    pub proxy_address: String,
    pub destination_address: String,
    pub client_address: String,
}

// connection result
pub enum TunnelConnectionResult {
    Ok, // 200
    BadRequest, // 400
    Unauthorized, // 401
    Forbidden, // 403
    NotFound, // 404
    RequestTimeout, // 408
    InternalServerError, // 500
    BadGateway, // 502
    Error,
}