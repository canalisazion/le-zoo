/// ✅ [H-8] Trait d'extension pour activer credentials:include sur WASM
/// (no-op en compilation native pour que cargo check passe)
use reqwest::RequestBuilder;

pub trait WithCredentials: Sized {
    fn with_credentials(self) -> Self;
}

impl WithCredentials for RequestBuilder {
    fn with_credentials(self) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            self.fetch_credentials_include()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self
        }
    }
}
