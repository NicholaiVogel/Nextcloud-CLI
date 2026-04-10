use std::time::Duration;

use reqwest::Method;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::config_schema::Profile;
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ClientAuth {
    pub username: String,
    pub app_password: String,
}

#[derive(Debug, Clone)]
pub struct NextcloudClient {
    server: Url,
    auth: Option<ClientAuth>,
    http: reqwest::Client,
}

impl NextcloudClient {
    pub fn new(server: Url, auth: Option<ClientAuth>) -> Result<Self> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_static(concat!("nextcloud-cli/", env!("CARGO_PKG_VERSION"))),
        );
        default_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .default_headers(default_headers)
            .build()?;

        Ok(Self { server, auth, http })
    }

    pub fn from_profile(profile: &Profile, app_password: Option<String>) -> Result<Self> {
        let auth = app_password.map(|password| ClientAuth {
            username: profile.username.clone(),
            app_password: password,
        });
        Self::new(profile.server.clone(), auth)
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
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(Error::HttpStatus { status, body });
        }

        Ok(response.json::<T>().await?)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
