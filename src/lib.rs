//! Another simple Neocities API wrapper.
//! 
//! # Usage:
//! 
//! Create a [`NeocitiesClient`] either with or without authentication
//! (no-auth clients have very limited functionality).
//! 
//! ```no_run
//! let client1 = NeocitiesClient::new("randomuser", "notmypassword");
//! let client2 = NeocitiesClient::new_with_key(&key);
//! let client3 = NeocitiesClient::new_no_auth();
//! ```
//! 
//! From there, you can talk to the Neocities API at your leisure.
//! 
//! ```no_run
//! let info = client1.info();
//! client2.upload("site/file.txt", "file.txt");
//! let someone_elses_info = client3.info_no_auth("ambyshframber");
//! // this is the only method that no-auth clients can call
//! ```
//! 
//! The crate also includes an optional utility module for dealing with site file lists. Enable the `site_info` feature to use it.

use std::path::Path;
use std::fs::read;

use reqwest::{blocking::{Client, RequestBuilder, multipart::{Part, Form}}};
use thiserror::Error;

#[cfg(any(feature = "site_info", doc))]
pub mod site_info;

/// The API client.
/// 
/// Can be created with authentication (username/password or API key) or without authentication.
/// Clients without authentication can only use `info_no_auth()`,
/// and all other methods will return an error.
/// 
/// All methods should return valid JSON.
/// Check beforehand though, because something might go wrong at the remote end.
#[derive(Debug)]
pub struct NeocitiesClient {
    client: Client,
    has_auth: bool,
    username: String,
    password: String,
    api_key: Option<String>
}

impl NeocitiesClient {
    /// Creates a client with a username and password.
    /// API methods called on the client will relate to the website belonging to the auth user.
    /// 
    /// Using a username and password is not recommended for automated tasks,
    /// as that involves leaving plaintext passwords in source code or configuration files.
    pub fn new(username: &str, password: &str) -> NeocitiesClient {
        NeocitiesClient {
            client: Client::new(),
            has_auth: true,
            username: String::from(username),
            password: String::from(password),
            api_key: None
        }
    }
    /// Creates a client with an API key.
    /// API methods called on the client will relate to the website belonging to the auth user.
    /// 
    /// ```no_run
    /// # use rs_neocities::client::NeocitiesClient;
    /// # use std::fs;
    /// let key = fs::read_to_string("auth_key.txt")?;
    /// let c = NeocitiesClient::new_with_key(&key);
    /// assert!(c.info().is_ok());
    /// ```
    /// 
    /// A key can be obtained by creating a client with a username and password,
    /// and calling `get_key()`. Keep it somewhere secure!
    pub fn new_with_key(key: &str) -> NeocitiesClient {
        NeocitiesClient {
            client: Client::new(),
            has_auth: true,
            username: String::new(),
            password: String::new(),
            api_key: Some(String::from(key))
        }
    }
    /// Creates a client with no authentication.
    /// 
    /// Calls to methods other than `info_no_auth()` will return an error.
    /// 
    /// ```no_run
    /// # use rs_neocities::client::NeocitiesClient;
    /// let c = NeocitiesClient::new_no_auth();
    /// assert!(c.info_no_auth("ambyshframber").is_ok());
    /// assert!(c.list_all().is_err())
    /// ```
    pub fn new_no_auth() -> NeocitiesClient {
        NeocitiesClient {
            client: Client::new(),
            has_auth: false,
            username: String::new(),
            password: String::new(),
            api_key: None
        }
    }

    fn get_auth(&self, req: RequestBuilder) -> Result<RequestBuilder, NeocitiesError> {
        if !self.has_auth {
            return Err(NeocitiesError::AuthError)
        }
        Ok(match &self.api_key {
            Some(k) => req.bearer_auth(k),
            None => req.basic_auth(&self.username, Some(&self.password))
        })
    }
    fn get(&self, endpoint: &str) -> Result<String, NeocitiesError> {
        let url = format!("https://neocities.org/api/{}", endpoint);
        Ok(self.get_auth(self.client.get(url))?.send()?.text()?)
    }

    /// Gets info about the auth user's site.
    pub fn info(&self) -> Result<String, NeocitiesError> {
        self.get("info")
    }
    /// Gets info about the given site.
    /// 
    /// Does not error if the site doesn't exist, but the returned value will be an error message from Neocities.
    pub fn info_no_auth(&self, site_name: &str) -> Result<String, NeocitiesError> {
        let url = format!("https://neocities.org/api/info?sitename={}", site_name); // doesn't need auth, just send it raw
        Ok(self.client.get(&url).send()?.text()?)
    }

    /// Lists all files and directories on the auth user's site.
    pub fn list_all(&self) -> Result<String, NeocitiesError> {
        self.get("list")
    }
    /// Lists files and directories starting from the specified path.
    pub fn list(&self, path: &str) -> Result<String, NeocitiesError> {
        self.get(&format!("list?path={}", path))
    }

    /// Uploads a local file to the site, placing it at `remote_path` relative to the site root.
    /// 
    /// Returns an error if the file can't be opened.
    /// 
    /// ## Example
    /// 
    /// ```no_run
    /// # use rs_neocities::client::NeocitiesClient;
    /// # use std::fs;
    /// # let key = String::new();
    /// let c = NeocitiesClient::new_with_key(&key);
    /// c.upload("site/index.html", "index.html");
    /// ```
    pub fn upload(&self, local_path: impl AsRef<Path>, remote_path: &str) -> Result<String, NeocitiesError> {
        let v = vec![(local_path, remote_path)];
        self.upload_multiple(&v) // reduce code reuse
    }
    /// Uploads multiple local files to the site. Path tuples should take the form `(local, remote)`,
    /// where `local` is the local path, and `remote` is the desired remote path relative to the root.
    /// 
    /// Returns an error if any of the files can't be opened.
    /// 
    /// ## Example
    /// 
    /// ```no_run
    /// # use rs_neocities::client::NeocitiesClient;
    /// # use std::fs;
    /// # let key = String::new();
    /// let c = NeocitiesClient::new_with_key(&key);
    /// 
    /// let mut files = Vec::new();
    /// files.push(("site/index.html", "index.html"));
    /// files.push(("images/favicon.ico", "favicon.ico"));
    /// 
    /// c.upload_multiple(files);
    /// ```
    pub fn upload_multiple(&self, paths: &[(impl AsRef<Path>, &str)]) -> Result<String, NeocitiesError> {
        let mut files = Vec::new();
        for (local, remote) in paths {
            files.push((read(local)?, remote))
        }

        self.upload_bytes_multiple(files)
    }
    /// Uploads a vector of bytes to the site as a file, placing it at `remote_path` relative to the site root.
    /// This is useful if you're generating data directly from an application,
    /// and want to upload it without having to save it to a file first.
    /// 
    /// ## Example
    /// 
    /// ```no_run
    /// # use rs_neocities::client::NeocitiesClient;
    /// # use std::fs;
    /// # let key = String::new();
    /// let c = NeocitiesClient::new_with_key(&key);
    /// let bytes = String::from("hello world!").bytes().collect();
    /// c.upload_bytes(bytes, "hello.txt");
    /// ```
    pub fn upload_bytes(&self, bytes: Vec<u8>, remote_path: &str) -> Result<String, NeocitiesError> {
        let v = vec![(bytes, remote_path)];
        self.upload_bytes_multiple(v)
    }
    /// Uploads multiple vectors  of bytes to the site as files.
    /// Tuples should take the form `(data, remote)`, where `data` is the data,
    /// and `remote` is the desired remote path relative to the root.
    /// 
    /// ## Example
    /// 
    /// ```no_run
    /// # use rs_neocities::client::NeocitiesClient;
    /// # use std::fs;
    /// # let key = String::new();
    /// let c = NeocitiesClient::new_with_key(&key);
    /// 
    /// let data = Vec::new();
    /// data.push((String::from("hello world!").bytes().collect(), "hello.txt"));
    /// let generated_data = get_data();
    /// data.push((generated_data, "data.bin"))
    /// 
    /// c.upload_bytes_multiple(data);
    /// ```
    pub fn upload_bytes_multiple(&self, bytes: Vec<(Vec<u8>, impl AsRef<str>)>) -> Result<String, NeocitiesError> {
        let mut form = Form::new();

        for (data, path) in bytes {
            let part = Part::bytes(data).file_name(String::from(path.as_ref()));
            form = form.part("", part)
        }

        Ok(self.get_auth(self.client.post("https://neocities.org/api/upload").multipart(form))?.send()?.text()?)
    }

    /// Delete a file on the site.
    /// `path` is from the site root.
    pub fn delete(&self, path: &str) -> Result<String, NeocitiesError> {
        let v = vec![path];
        self.delete_multiple(v)
    }

    /// Delete multiple files.
    pub fn delete_multiple(&self, files: Vec<&str>) -> Result<String, NeocitiesError> {
        let mut req = self.get_auth(self.client.post("https://neocities.org/api/delete"))?;

        for f in files {
            req = req.query(&[("filenames[]", f)]);
        }

        Ok(req.send()?.text()?)
    }
    
    /// Gets the API key for the auth user. You generally only need to get this once,
    /// so I would recommend just doing it with curl:
    /// 
    /// ```sh
    /// curl "https://USER:PASS@neocities.org/api/key"
    /// ```
    pub fn get_key(&self) -> Result<String, NeocitiesError> {
        self.get("key")
    }
}

#[derive(Error, Debug)]
pub enum NeocitiesError {
    #[error("http request error")]
    RequestError(#[from] reqwest::Error),
    #[error("local file read error")]
    FileError(#[from] std::io::Error),
    #[error("authentication error")]
    AuthError,
    #[error("site item list parse error")]
    ListParseError
}
