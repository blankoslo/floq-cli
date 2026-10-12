#[derive(Debug)]
pub struct Env {
    pub issuer: &'static str,
    pub client_id: &'static str,
    pub api_domain: &'static str,
}

#[cfg(feature = "prod")]
pub const ENV: Env = Env {
    issuer: "https://inni.blank.no",
    client_id: "c7d39ede18d18bee3528b7f6c1962806",
    api_domain: "https://api-prod.floq.no",
};

#[cfg(not(feature = "prod"))]
pub const ENV: Env = Env {
    issuer: "https://test.floq.no",
    client_id: "745f8da8135720902b0a164f59f59318",
    api_domain: "https://api-test.floq.no",
};
