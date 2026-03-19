// ✅ [H-7] URLs configurables via variables d'environnement compile-time (WASM safe)
pub const API_BASE_URL: &str = match option_env!("API_BASE_URL") {
    Some(v) => v,
    None => "http://localhost:3000",
};
pub const WS_BASE_URL: &str = match option_env!("WS_BASE_URL") {
    Some(v) => v,
    None => "ws://localhost:3000",
};