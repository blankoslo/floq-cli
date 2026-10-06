use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use oauth2::{EndpointMaybeSet, EndpointNotSet, EndpointSet, ExtraTokenFields, StandardTokenResponse, TokenType};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient};
use openidconnect::reqwest;
use openidconnect::{
    AuthorizationCode, ClientId, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge, RedirectUrl,
    RefreshToken, RevocationUrl, Scope,
};
use rouille::Response;
use serde::{Deserialize, Serialize};

const FLOQ_ISSUER: Option<&str> = option_env!("FLOQ_ISSUER");
const CLIENT_ID: Option<&str> = option_env!("CLIENT_ID");

pub fn floq_issuer() -> &'static str {
    FLOQ_ISSUER.unwrap_or("https://test.floq.no")
}

pub fn client_id() -> &'static str {
    CLIENT_ID.unwrap_or("745f8da8135720902b0a164f59f59318")
}

#[derive(Debug, Deserialize, Serialize)]
struct AuthorizeResponse {
    state: CsrfToken,
    code: AuthorizationCode,
}

#[derive(Debug)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<Duration>,
    pub scopes: Option<Vec<String>>,
}

// Teach openidconnect about the revocation_endpoint in the OpenID Discovery
// response that we can use as the RFC 7009 OAuth 2.0 Token Revocation endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct RevocationEndpointProviderMetadata {
    revocation_endpoint: String,
}
impl openidconnect::AdditionalProviderMetadata for RevocationEndpointProviderMetadata {}
type FloqProviderMetadata = openidconnect::ProviderMetadata<
    RevocationEndpointProviderMetadata,
    openidconnect::core::CoreAuthDisplay,
    openidconnect::core::CoreClientAuthMethod,
    openidconnect::core::CoreClaimName,
    openidconnect::core::CoreClaimType,
    openidconnect::core::CoreGrantType,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJweKeyManagementAlgorithm,
    openidconnect::core::CoreJsonWebKey,
    openidconnect::core::CoreResponseMode,
    openidconnect::core::CoreResponseType,
    openidconnect::core::CoreSubjectIdentifierType,
>;

type FloqClient = openidconnect::core::CoreClient<
    EndpointSet,      // Authorization URL
    EndpointNotSet,   // Device Authorization URL
    EndpointNotSet,   // Introspection URL
    EndpointSet,      // Revocation URL
    EndpointMaybeSet, // Token URL
    EndpointMaybeSet, // User Info URL
>;

#[derive(Debug, Clone)]
pub struct FloqAuth {
    http_client: reqwest::Client,
    issuer: IssuerUrl,
    client_id: ClientId,
}

impl Default for FloqAuth {
    fn default() -> Self {
        Self::new(floq_issuer().to_string(), client_id().to_string()).unwrap()
    }
}

impl FloqAuth {
    pub fn new(issuer: String, client_id: String) -> Result<Self> {
        Ok(Self {
            http_client: reqwest::ClientBuilder::new()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("Client should build"),
            issuer: IssuerUrl::new(issuer)?,
            client_id: ClientId::new(client_id),
        })
    }

    pub fn issuer(&self) -> &str {
        self.issuer.as_str()
    }

    async fn oauth_client(&self) -> Result<FloqClient> {
        let provider_metadata = FloqProviderMetadata::discover_async(self.issuer.clone(), &self.http_client)
            .await
            .context("Failed to discover OpenID provider metadata from Floq")?;

        let revocation_url =
            RevocationUrl::new(provider_metadata.additional_metadata().revocation_endpoint.to_string())?;

        let oauth_client: FloqClient =
            CoreClient::from_provider_metadata(provider_metadata, self.client_id.clone(), None)
                .set_revocation_url(revocation_url);
        Ok(oauth_client)
    }

    pub async fn authenticate<OUT: Write + Send>(&self, out: &mut OUT) -> Result<AuthResponse> {
        let oauth_client = self.oauth_client().await?;

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (tx, rx) = mpsc::sync_channel::<Result<AuthorizeResponse>>(0);

        let server = rouille::Server::new("0.0.0.0:0", move |request| {
            match serde_urlencoded::from_str::<AuthorizeResponse>(request.raw_query_string())
                .context("Failed to parse callback query string")
            {
                Ok(authorize_response) => {
                    tx.send(Ok(authorize_response)).unwrap();
                    // TODO: Fully process the authorization response before sending a confirmation to the user
                    Response::text("Flott, da er du logget inn i floq cli!\n\n(Bare å lukke denne fanen)")
                }
                Err(e) => {
                    tx.send(Err(e)).unwrap();
                    Response::text(
                        "An error occurred while trying to handle Auth callback, see command output for more details",
                    )
                }
            }
        })
        .map_err(|e| anyhow!("{}", e))?;
        let port = server.server_addr().port();
        let callback_url = format!("http://localhost:{}", port);

        let oauth_client = oauth_client.set_redirect_uri(RedirectUrl::new(callback_url)?);

        let (auth_url, csrf_token, _nonce) = oauth_client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("offline_access".to_string()))
            .add_scope(Scope::new("role:employee".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        open::that(auth_url.as_str())?;

        writeln!(out)?;
        writeln!(
            out,
            "Vennligst åpne denne lenken i nettleseren din hvis det ikke skjedde automatisk:"
        )?;
        writeln!(out, "{}", auth_url)?;
        writeln!(out)?;

        let authorize_response = loop {
            match rx.try_iter().next() {
                Some(Ok(authorize_response)) => break Ok(authorize_response),
                Some(Err(e)) => break Err(e),
                None => {
                    std::thread::sleep(Duration::from_millis(250));
                    server.poll();
                }
            }
        }?;
        drop(server);

        // NOTE: not using constant-time comparison for CSRF token
        if authorize_response.state.secret() != csrf_token.secret() {
            return Err(anyhow!("state mismatch"));
        }

        let token_response = oauth_client
            .exchange_code(authorize_response.code)?
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.http_client)
            .await
            .context("Failed to exchange authorization code for tokens")?;

        Ok(to_auth_response(token_response))
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<AuthResponse> {
        let oauth_client = self.oauth_client().await?;

        let refresh_token = RefreshToken::new(refresh_token.to_string());
        let token_response = oauth_client
            .exchange_refresh_token(&refresh_token)
            .context("Failed to create refresh token request")?
            .request_async(&self.http_client)
            .await
            .context("Failed to refresh access token")?;

        Ok(to_auth_response(token_response))
    }

    pub async fn revoke(&self, refresh_token: &str) -> Result<()> {
        let oauth_client = self.oauth_client().await?;

        let refresh_token = RefreshToken::new(refresh_token.to_string());
        oauth_client
            .revoke_token(openidconnect::core::CoreRevocableToken::RefreshToken(refresh_token))
            .context("Failed to create revoke token request")?
            .request_async(&self.http_client)
            .await
            .context("Failed to revoke refresh token")?;

        Ok(())
    }
}

fn to_auth_response<EF, TT>(res: StandardTokenResponse<EF, TT>) -> AuthResponse
where
    EF: ExtraTokenFields,
    TT: TokenType,
{
    AuthResponse {
        access_token: res.access_token().secret().to_string(),
        refresh_token: res.refresh_token().map(|t| t.secret().to_string()),
        expires_in: res.expires_in(),
        scopes: res
            .scopes()
            .map(|s| s.iter().map(|scope| scope.as_str().to_string()).collect()),
    }
}
