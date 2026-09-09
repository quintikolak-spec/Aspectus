use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use keyring::Entry;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

const SERVICE: &str = "personal-dashboard";
const ACCOUNT: &str = "openai-credentials";

// "Sign in with ChatGPT" endpoints, matching the flow OpenAI's own Codex
// CLI uses (auth.openai.com /oauth/authorize + /oauth/token, PKCE,
// localhost loopback redirect). As of writing, this flow requires the
// requesting app to be registered with OpenAI via their developer
// interest form to receive a client_id -- general availability for
// arbitrary third-party desktop apps was still rolling out. Until this
// app has an approved client_id, prefer `sign_in_with_api_key` below,
// which works today for every developer via platform.openai.com/api-keys
// and satisfies section 18's "kein Passwort speichern" requirement just
// as well (only an API key is stored, never the ChatGPT password).
const AUTH_BASE: &str = "https://auth.openai.com";
const REDIRECT_PORT: u16 = 1455;
const CLIENT_ID: &str = "REPLACE_WITH_OPENAI_REGISTERED_CLIENT_ID"; // TODO: fill in once approved

#[derive(Serialize, Deserialize, Clone)]
struct StoredCredentials {
    kind: CredentialKind,
    access_token: String,
    refresh_token: Option<String>,
    /// RFC3339 timestamp; None means "does not expire" (plain API keys).
    expires_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum CredentialKind {
    #[serde(rename = "oauth")]
    OAuth,
    #[serde(rename = "api_key")]
    ApiKey,
}

fn entry() -> Result<Entry> {
    Ok(Entry::new(SERVICE, ACCOUNT)?)
}

fn load_credentials() -> Result<StoredCredentials> {
    let raw = entry()?.get_password().context("not signed in")?;
    Ok(serde_json::from_str(&raw)?)
}

fn store_credentials(creds: &StoredCredentials) -> Result<()> {
    entry()?.set_password(&serde_json::to_string(creds)?)?;
    Ok(())
}

pub fn sign_out() -> Result<()> {
    if let Ok(e) = entry() {
        let _ = e.delete_credential();
    }
    Ok(())
}

pub fn is_signed_in() -> bool {
    load_credentials().is_ok()
}

/// Practical, always-available sign-in path: the user pastes an API key
/// generated at platform.openai.com/api-keys. Stored in the OS keychain
/// only, never in plaintext settings -- same guarantee section 18 asks for
/// re: the OAuth flow.
pub fn sign_in_with_api_key(key: &str) -> Result<()> {
    if !key.trim().starts_with("sk-") {
        return Err(anyhow!("Das sieht nicht nach einem gültigen OpenAI-API-Key aus (sollte mit \"sk-\" beginnen)."));
    }
    let creds = StoredCredentials {
        kind: CredentialKind::ApiKey,
        access_token: key.trim().to_string(),
        refresh_token: None,
        expires_at: None,
    };
    store_credentials(&creds).map_err(|e| {
        anyhow!(
            "Konnte den Key nicht sicher speichern: {e}. Läuft ein Schlüsselbund-Dienst \
             (z. B. KWallet oder gnome-keyring)? Ohne einen kann dieses Betriebssystem \
             keine Zugangsdaten dauerhaft sichern."
        )
    })?;

    // Verify the write actually landed and will survive a restart — some
    // Linux setups without a properly unlocked default keyring collection
    // accept the write but only into a session-scoped store that vanishes
    // on the next login, which otherwise fails completely silently.
    match load_credentials() {
        Ok(reloaded) if reloaded.access_token == creds.access_token => Ok(()),
        _ => Err(anyhow!(
            "Der Key wurde scheinbar gespeichert, ließ sich aber nicht wieder auslesen. \
             Das deutet auf ein Problem mit dem System-Schlüsselbund hin (z. B. KWallet \
             nicht entsperrt) — der Key würde einen App-Neustart vermutlich nicht überleben."
        )),
    }
}

/// Returns a currently-valid access token, transparently refreshing an
/// OAuth token if it's close to expiry. API keys are returned as-is.
pub async fn get_token() -> Result<String> {
    let creds = load_credentials()?;

    if creds.kind == CredentialKind::ApiKey {
        return Ok(creds.access_token);
    }

    let still_valid = creds
        .expires_at
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
        .map(|exp| exp > chrono::Utc::now() + chrono::Duration::seconds(60))
        .unwrap_or(false);

    if still_valid {
        return Ok(creds.access_token);
    }

    let refresh_token = creds.refresh_token.context("no refresh token available")?;
    let refreshed = exchange_token(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", &refresh_token),
        ("client_id", CLIENT_ID),
    ])
    .await?;

    store_credentials(&refreshed)?;
    Ok(refreshed.access_token)
}

/// Runs the full "Sign in with ChatGPT" PKCE flow: opens the system
/// browser to auth.openai.com, spins up a one-shot localhost server to
/// catch the redirect, then exchanges the code for tokens.
pub async fn sign_in_oauth(app: &AppHandle) -> Result<()> {
    if CLIENT_ID.starts_with("REPLACE_WITH") {
        return Err(anyhow!(
            "OAuth client_id not configured yet -- register this app with OpenAI first, or use sign_in_with_api_key"
        ));
    }

    let verifier = generate_code_verifier();
    let challenge = code_challenge(&verifier);
    let state = generate_state();
    let redirect_uri = format!("http://localhost:{REDIRECT_PORT}/auth/callback");

    let authorize_url = format!(
        "{AUTH_BASE}/oauth/authorize?response_type=code&client_id={CLIENT_ID}&redirect_uri={redirect}&code_challenge={challenge}&code_challenge_method=S256&state={state}&scope=openid%20profile%20email%20offline_access",
        redirect = urlencoding_light(&redirect_uri),
    );

    app.opener()
        .open_url(&authorize_url, None::<&str>)
        .map_err(|e| anyhow!("could not open browser: {e}"))?;

    // Blocking accept() on a std listener -- run it off the async runtime's
    // worker threads so we don't stall other Tauri commands while we wait
    // for the user to finish the browser flow.
    let (code, returned_state) =
        tauri::async_runtime::spawn_blocking(move || wait_for_redirect(REDIRECT_PORT)).await??;

    if returned_state != state {
        return Err(anyhow!("OAuth state mismatch -- possible CSRF, aborting"));
    }

    let creds = exchange_token(&[
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", &redirect_uri),
        ("client_id", CLIENT_ID),
        ("code_verifier", &verifier),
    ])
    .await?;

    store_credentials(&creds)
}

fn wait_for_redirect(port: u16) -> Result<(String, String)> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .context("could not bind local redirect listener -- is the port already in use?")?;

    let (mut stream, _) = listener.accept()?;
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let request_line = request.lines().next().unwrap_or_default();
    let path = request_line.split_whitespace().nth(1).unwrap_or_default();
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or_default();

    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "code" => code = Some(v.to_string()),
                "state" => state = Some(v.to_string()),
                _ => {}
            }
        }
    }

    let body = "<html><body style=\"font-family:sans-serif;text-align:center;margin-top:20vh\">Anmeldung erfolgreich - du kannst dieses Fenster jetzt schliessen.</body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());

    Ok((
        code.ok_or_else(|| anyhow!("no code in redirect"))?,
        state.ok_or_else(|| anyhow!("no state in redirect"))?,
    ))
}

async fn exchange_token(params: &[(&str, &str)]) -> Result<StoredCredentials> {
    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    }

    let client = reqwest::Client::new();
    let resp: TokenResponse = client
        .post(format!("{AUTH_BASE}/oauth/token"))
        .form(params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let expires_at = resp
        .expires_in
        .map(|secs| (chrono::Utc::now() + chrono::Duration::seconds(secs)).to_rfc3339());

    Ok(StoredCredentials {
        kind: CredentialKind::OAuth,
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_at,
    })
}

fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

// Minimal query-param encoding for the one dynamic value we interpolate
// (the redirect URI) -- avoids pulling in a full url-crate percent-encode
// helper for a single call site.
fn urlencoding_light(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F")
}
