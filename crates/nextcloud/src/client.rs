use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::config_schema::Profile;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadedBytes {
    pub bytes_written: u64,
    pub content_length: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ClientAuth {
    pub username: String,
    pub app_password: String,
}

/// Per-client TLS behavior. Custom CA certificates are added to the normal
/// system trust roots; they do not replace them and do not disable hostname
/// verification. `insecure` is deliberately an invocation-only escape hatch
/// and is enforced by the CLI's profile policy before a client is built.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TlsOptions {
    pub ca_bundle: Option<PathBuf>,
    pub insecure: bool,
}

impl TlsOptions {
    pub fn new(ca_bundle: Option<PathBuf>, insecure: bool) -> Self {
        Self {
            ca_bundle,
            insecure,
        }
    }

    pub fn ca_bundle(path: impl Into<PathBuf>) -> Self {
        Self {
            ca_bundle: Some(path.into()),
            insecure: false,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(path) = &self.ca_bundle {
            load_ca_bundle(path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NextcloudClient {
    server: Url,
    auth: Option<ClientAuth>,
    http: reqwest::Client,
}

impl NextcloudClient {
    pub fn new(server: Url, auth: Option<ClientAuth>) -> Result<Self> {
        Self::new_with_tls_options(server, auth, &TlsOptions::default())
    }

    pub fn new_with_tls_options(
        server: Url,
        auth: Option<ClientAuth>,
        tls_options: &TlsOptions,
    ) -> Result<Self> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_static(concat!("nextcloud-cli/", env!("CARGO_PKG_VERSION"))),
        );
        default_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(60))
            .default_headers(default_headers);

        if tls_options.insecure {
            builder = builder.danger_accept_invalid_certs(true);
            builder = builder.danger_accept_invalid_hostnames(true);
        }

        if let Some(path) = &tls_options.ca_bundle {
            for certificate in load_ca_bundle(path)? {
                builder = builder.add_root_certificate(certificate);
            }
        }

        let http = builder.build().map_err(|error| {
            if let Some(path) = &tls_options.ca_bundle {
                Error::TlsCaBundleInvalid {
                    path: path.clone(),
                    message: "certificate bundle could not be loaded".to_owned(),
                }
            } else {
                Error::Http(error)
            }
        })?;

        Ok(Self { server, auth, http })
    }

    pub fn from_profile(profile: &Profile, app_password: Option<String>) -> Result<Self> {
        Self::from_profile_with_tls_options(profile, app_password, &TlsOptions::default())
    }

    pub fn from_profile_with_tls_options(
        profile: &Profile,
        app_password: Option<String>,
        tls_options: &TlsOptions,
    ) -> Result<Self> {
        let auth = app_password.map(|password| ClientAuth {
            username: profile.username.clone(),
            app_password: password,
        });
        Self::new_with_tls_options(profile.server.clone(), auth, tls_options)
    }

    pub fn server(&self) -> &Url {
        &self.server
    }

    pub async fn get_json<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.join(path)?;
        let mut request = self.http.get(url);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(Error::HttpStatus { status, body });
        }

        Ok(response.json::<T>().await?)
    }

    pub async fn get_ocs_json<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.join(path)?;
        let mut request = self.http.get(url).header("OCS-APIRequest", "true");
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        parse_ocs_json_response(response).await
    }

    pub async fn post_empty_json<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.join(path)?;
        let response = self.http.post(url).send().await?;
        parse_json_response(response).await
    }

    pub async fn post_form_json<F, T>(&self, url: url::Url, form: &F) -> Result<T>
    where
        F: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let response = self.http.post(url).form(form).send().await?;
        parse_json_response(response).await
    }

    pub async fn post_ocs_form_json<F, T>(&self, path: &str, form: &F) -> Result<T>
    where
        F: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = self.join(path)?;
        let mut request = self
            .http
            .post(url)
            .header("OCS-APIRequest", "true")
            .form(form);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        parse_ocs_json_response(response).await
    }

    pub async fn request_json<B, T>(&self, method: Method, path: &str, body: &B) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = self.join(path)?;
        let mut request = self.http.request(method, url).json(body);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        parse_json_response(response).await
    }

    pub async fn request_json_with_ocs_header<B, T>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = self.join(path)?;
        let mut request = self
            .http
            .request(method, url)
            .header("OCS-APIRequest", "true")
            .json(body);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        parse_json_response(response).await
    }

    pub async fn request_json_with_ocs_header_no_response<B>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<()>
    where
        B: Serialize + ?Sized,
    {
        let url = self.join(path)?;
        let mut request = self
            .http
            .request(method, url)
            .header("OCS-APIRequest", "true")
            .json(body);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(Error::HttpStatus { status, body });
        }
        Ok(())
    }

    pub async fn request_empty_with_ocs_header(&self, method: Method, path: &str) -> Result<()> {
        let url = self.join(path)?;
        let mut request = self
            .http
            .request(method, url)
            .header("OCS-APIRequest", "true");
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(Error::HttpStatus { status, body });
        }
        Ok(())
    }

    pub async fn delete_ocs_json<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.join(path)?;
        let mut request = self.http.delete(url).header("OCS-APIRequest", "true");
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        parse_ocs_json_response(response).await
    }

    pub async fn request_text(
        &self,
        method: Method,
        path: &str,
        dav_body: Option<(&str, &str)>,
    ) -> Result<String> {
        let url = self.join(path)?;
        let mut request = self.http.request(method, url);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }
        if let Some((depth, body)) = dav_body {
            request = request
                .header("Depth", depth)
                .header(CONTENT_TYPE, "application/xml; charset=utf-8")
                .body(body.to_owned());
        }

        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| String::new());
        if !status.is_success() {
            return Err(Error::HttpStatus { status, body });
        }

        Ok(body)
    }

    pub async fn request_xml_text(
        &self,
        method: Method,
        path: &str,
        body: String,
    ) -> Result<String> {
        let url = self.join(path)?;
        let mut request = self
            .http
            .request(method, url)
            .header(CONTENT_TYPE, "application/xml; charset=utf-8")
            .body(body);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| String::new());
        if !status.is_success() {
            return Err(Error::HttpStatus { status, body });
        }

        Ok(body)
    }

    pub async fn request_webdav_transfer(
        &self,
        method: Method,
        path: &str,
        destination: &Url,
        overwrite: bool,
    ) -> Result<()> {
        let url = self.join(path)?;
        let mut request = self
            .http
            .request(method, url)
            .header("Destination", destination.as_str())
            .header("Overwrite", if overwrite { "T" } else { "F" });
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(Error::HttpStatus { status, body });
        }

        Ok(())
    }

    pub async fn request_bytes(&self, method: Method, path: &str) -> Result<Vec<u8>> {
        let url = self.join(path)?;
        self.request_bytes_url(method, url).await
    }

    pub async fn preview(&self, file_id: &str, width: u32, height: u32) -> Result<Vec<u8>> {
        let mut url = self.join("core/preview")?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("fileId", file_id);
            query.append_pair("x", &width.max(1).to_string());
            query.append_pair("y", &height.max(1).to_string());
            query.append_pair("a", "1");
        }
        self.request_bytes_url(Method::GET, url).await
    }

    async fn request_bytes_url(&self, method: Method, url: Url) -> Result<Vec<u8>> {
        let mut request = self.http.request(method, url);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(Error::HttpStatus { status, body });
        }

        Ok(response.bytes().await?.to_vec())
    }

    pub async fn request_to_writer<W>(
        &self,
        method: Method,
        path: &str,
        writer: &mut W,
    ) -> Result<DownloadedBytes>
    where
        W: Write,
    {
        let url = self.join(path)?;
        let mut request = self.http.request(method, url);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }

        let mut response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(Error::HttpStatus { status, body });
        }

        let content_length = response.content_length();
        let mut bytes_written = 0_u64;
        while let Some(chunk) = response.chunk().await? {
            writer
                .write_all(&chunk)
                .map_err(|source| Error::WriteResponse { source })?;
            bytes_written += chunk.len() as u64;
        }

        if let Some(expected) = content_length
            && expected != bytes_written
        {
            return Err(Error::DownloadSizeMismatch {
                expected,
                actual: bytes_written,
            });
        }

        Ok(DownloadedBytes {
            bytes_written,
            content_length,
        })
    }

    pub async fn put_bytes(
        &self,
        path: &str,
        bytes: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<Option<String>> {
        let url = self.join(path)?;
        let mut request = self.http.put(url).body(bytes);
        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.app_password));
        }
        if let Some(content_type) = content_type {
            request = request.header(CONTENT_TYPE, content_type);
        }

        let response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await.unwrap_or_else(|_| String::new());
        if !status.is_success() {
            return Err(Error::HttpStatus { status, body });
        }

        Ok(headers
            .get("etag")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim_matches('"').to_owned()))
    }

    pub fn join(&self, path: &str) -> Result<Url> {
        let normalized = path.trim_start_matches('/');
        self.server
            .join(normalized)
            .map_err(|source| Error::InvalidServerUrl {
                value: format!("{} + {}", self.server, path),
                source,
            })
    }
}

fn load_ca_bundle(path: &Path) -> Result<Vec<reqwest::Certificate>> {
    let bytes = fs::read(path).map_err(|source| Error::TlsCaBundleRead {
        path: path.to_path_buf(),
        source,
    })?;

    let certificates = reqwest::Certificate::from_pem_bundle(&bytes).map_err(|_error| {
        Error::TlsCaBundleInvalid {
            path: path.to_path_buf(),
            message: "expected a PEM-encoded certificate bundle".to_owned(),
        }
    })?;
    if certificates.is_empty() {
        return Err(Error::TlsCaBundleInvalid {
            path: path.to_path_buf(),
            message: "expected a PEM-encoded certificate bundle".to_owned(),
        });
    }
    Ok(certificates)
}

async fn parse_json_response<T>(response: reqwest::Response) -> Result<T>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| String::new());
        return Err(Error::HttpStatus { status, body });
    }

    Ok(response.json::<T>().await?)
}

async fn parse_ocs_json_response<T>(response: reqwest::Response) -> Result<T>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if status.is_success() {
        return Ok(response.json::<T>().await?);
    }

    let body = response.text().await.unwrap_or_else(|_| String::new());
    if let Ok(value) = serde_json::from_str::<T>(&body) {
        return Ok(value);
    }

    Err(Error::HttpStatus { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;

    #[test]
    fn joins_relative_paths_to_server() -> Result<()> {
        let server = Url::parse("https://cloud.example.com/").expect("valid URL");
        let client = NextcloudClient::new(server, None)?;
        assert_eq!(
            client.join("/status.php")?.as_str(),
            "https://cloud.example.com/status.php"
        );
        Ok(())
    }

    #[test]
    fn invalid_ca_bundle_fails_during_client_construction() {
        let missing = PathBuf::from("/definitely/missing/nextcloud-ca.pem");
        let error = NextcloudClient::new_with_tls_options(
            Url::parse("https://cloud.example.com/").expect("valid URL"),
            None,
            &TlsOptions::ca_bundle(missing.clone()),
        )
        .expect_err("missing CA bundle should fail before any request");

        assert_eq!(error.code(), "tls_ca_bundle_read_failed");
        assert!(error.to_string().contains("nextcloud-ca.pem"));
    }

    #[test]
    fn rejects_invalid_pem_certificate_bundle_before_any_request() -> Result<()> {
        let temp = TempDir::new().expect("temporary directory");
        let path = temp.path().join("ca.pem");
        fs::write(
            &path,
            "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n",
        )
        .expect("write test fixture");

        let error = NextcloudClient::new_with_tls_options(
            Url::parse("https://cloud.example.com/").expect("valid URL"),
            None,
            &TlsOptions::ca_bundle(path),
        )
        .expect_err("invalid certificate fixture should be rejected");
        assert_eq!(error.code(), "tls_ca_bundle_invalid");
        Ok(())
    }

    #[tokio::test]
    async fn custom_ca_bundle_trusts_a_local_tls_server() -> Result<()> {
        let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()])
            .expect("generate local test certificate");
        let certificate_der = certified.cert.der().clone();
        let private_key = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der()),
        );
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate_der], private_key)
            .expect("build local TLS server config");
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local TLS server");
        let address = listener.local_addr().expect("read local TLS address");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept local TLS request");
            let mut stream = acceptor
                .accept(stream)
                .await
                .expect("complete local TLS handshake");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await.expect("read HTTP request");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 12\r\nconnection: close\r\n\r\n{\"ok\":true}\n",
                )
                .await
                .expect("write HTTP response");
        });

        let temp = TempDir::new().expect("temporary directory");
        let ca_path = temp.path().join("local-ca.pem");
        fs::write(&ca_path, certified.cert.pem()).expect("write local CA bundle");
        let client = NextcloudClient::new_with_tls_options(
            Url::parse(&format!("https://{address}/")).expect("valid local TLS URL"),
            None,
            &TlsOptions::ca_bundle(ca_path),
        )?;
        let response: serde_json::Value = client.get_json("health").await?;
        assert_eq!(response["ok"], true);
        server.await.expect("local TLS server task");
        Ok(())
    }
}
