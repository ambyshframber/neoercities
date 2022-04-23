//! Quality of life struct and methods for dealing with site file lists.
//! 
//! So let's say you only want to upload files when the local versions differ from the remote.
//! You could theoretically download the remote file, check it against your local file,
//! and then if they differ, upload the local. But that's a lot of work,
//! and ends up transferring more data, which is precisely what we were trying to avoid.
//! 
//! Neocities very thoughtfully provide file hashes in their site list responses,
//! meaning you can just compare hashes and transfer far less data overall. For example:
//! 
//! ```no_run
//! # fn main() -> Result<(), NeocitiesError> {
//! # let key = String::new();
//! let client = NeocitiesClient::new_with_key(&key);
//! let info = SiteInfo::new(&client);
//! 
//! if info.file_changed("site/index.html", "/index.html") {
//!     client.upload("site/index.html", "/index.html")
//! }
//! # }
//! ```
//! 
//! The [`SiteInfo`] struct also provides more general methods for getting files and directories on the site.

use std::path::Path;
use std::io;
use std::fs::read;

use serde_json::{Value, from_str};
use chrono::Utc;
use chrono;
use sha1::{Sha1, Digest};

use crate::{NeocitiesClient, NeocitiesError};

/// A struct containing the file list of the site, parsed into native (non-JSON) datatypes.
/// Items are not stored recursively, but instead as one big list.
/// 
/// To make usage easier, `SiteInfo` takes ownership of the client. The field is public, so you can still call methods on it.
/// 
/// Under the hood, this just calls `client.list_all()` and parses the returned JSON data into easier-to-use solid types.
/// All paths begin with `/`, so make sure you take account of that when checking for files.
pub struct SiteInfo {
    pub client: NeocitiesClient,
    /// All the files and directories on the site
    pub items: Vec<SiteItem>
}
impl SiteInfo<'_> {
    /// Create a new `SiteInfo` using an existing client. It will contain info about the auth user's site.
    /// 
    /// Returns an error if the HTTP request fails or if the API call somehow returns malformed or invalid JSON.
    pub fn new(client: &NeocitiesClient) -> Result<SiteInfo, NeocitiesError> {
        let mut i = SiteInfo {
            client,
            items:Vec::new()
        };
        i.refresh()?;
        Ok(i)
    }
    /// Refreshes the information by querying the API again.
    /// 
    /// Returns an error if the HTTP request fails or if the API call somehow returns malformed or invalid JSON.
    pub fn refresh(&mut self) -> Result<(), NeocitiesError> {
        let list = from_str::<Value>(&self.client.list_all()?).unwrap();

        let mut items = Vec::new(); // clear local cache
        for entry in list.get("files").unwrap().as_array().unwrap() { // go through list and parse
            items.push(SiteItem::from_json(entry)?) // don't bother with error checking cuz unless neocities have a problem we should be fine
        }
        self.items = items;

        Ok(())
    }

    /// Get a reference to item on the site, if it exists.
    pub fn get_item(&self, path: &str) -> Option<&SiteItem> {
        for i in &self.items {
            if i.get_path() == path {
                return Some(i)
            }
        }
        None
    }
    /// Get a reference to file on the site, if it exists.
    pub fn get_file(&self, path: &str) -> Option<&File> {
        match self.get_item(path) {
            Some(SiteItem::File(f)) => Some(f),
            _ => None
        }
    }
    /// Get a reference to file on the site, if it exists.
    pub fn get_dir(&self, path: &str) -> Option<&Dir> {
        match self.get_item(path) {
            Some(SiteItem::Dir(d)) => Some(d),
            _ => None
        }
    }

    pub fn file_exists_on_site(&self, path: &str) -> bool {
        self.get_file(path).is_some()
    }
    pub fn dir_exists_on_site(&self, path: &str) -> bool {
        self.get_dir(path).is_some()
    }

    /// Compare hashes of files to find out if the local and remote versions are different.
    /// Returns `true` if the remote file doesn't exist.
    /// 
    /// Returns an error if the local file can't be opened.
    pub fn file_changed(&self, local_path: impl AsRef<Path>, remote_path: &str) -> Result<bool, NeocitiesError> {
        match self.get_file(remote_path) {
            Some(f) => {
                Ok(hash_of_local(local_path)? == f.sha1_hash)
            }
            None => Ok(true)
        }
    }
}

/// Get the sha1 hash of a local file. Returns an error if the file fails to open.
pub fn hash_of_local(path: impl AsRef<Path>) -> Result<String, io::Error> {
    Ok(hash_of_bytes(path.read()?))
}
pub fn hash_of_string(s: impl AsRef<str>) -> String {
    hash_of_bytes(s.as_ref().as_bytes())
}
/// Get the sha1 hash of a set of bytes.
pub fn hash_of_bytes(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let arr = hasher.finalize();
    let mut ret = String::new();
    for b in arr {
        ret.push_str(&format!("{:01x}", b))
    }
    Ok(ret)
}

/// Represents a file on the site
pub struct File {
    /// Path of the file, from root (eg /index.html)
    pub path: String,
    /// Time the file was last modified, in UTC
    pub modified: chrono::DateTime<Utc>,
    /// The sha1 hash of the file
    pub sha1_hash: String,
    /// The size of the file, in bytes
    pub size: u64
}
impl File {
    fn from_json(j: &Value) -> Result<File, NeocitiesError> {
        Ok(File {
            path: format!("/{}", j.get("path").ok_or(NeocitiesError::ListParseError)?.as_str().ok_or(NeocitiesError::ListParseError)?), // extra / for sanity
            modified: chrono::DateTime::parse_from_rfc2822(j.get("updated_at").ok_or(NeocitiesError::ListParseError)?.as_str().ok_or(NeocitiesError::ListParseError)?).unwrap().with_timezone(&Utc),
            sha1_hash: String::from(j.get("sha1_hash").ok_or(NeocitiesError::ListParseError)?.as_str().ok_or(NeocitiesError::ListParseError)?),
            size: j.get("size").ok_or(NeocitiesError::ListParseError)?.as_u64().ok_or(NeocitiesError::ListParseError)? // if any of this panics don't blame me
        })
    }
}
/// Represents a directory on the site.
pub struct Dir {
    /// Path of the directory, from root (eg /blog)
    pub path: String,
    /// Time the directory was last modified, in UTC
    pub modified: chrono::DateTime<Utc>
}
impl Dir {
    fn from_json(j: &Value) -> Result<Dir, NeocitiesError> {
        Ok(Dir {
            path: format!("/{}", j.get("path").ok_or(NeocitiesError::ListParseError)?.as_str().ok_or(NeocitiesError::ListParseError)?),
            modified: chrono::DateTime::parse_from_rfc2822(j.get("updated_at").ok_or(NeocitiesError::ListParseError)?.as_str().ok_or(NeocitiesError::ListParseError)?).unwrap().with_timezone(&Utc),
        })
    }
}

/// Represents an item on the site.
pub enum SiteItem {
    File(File),
    Dir(Dir)
}
impl SiteItem {
    pub fn get_path(&self) -> &str {
        match self {
            SiteItem::Dir(d) => &d.path,
            SiteItem::File(f) => &f.path
        }
    }
    fn from_json(j: &Value) -> Result<SiteItem, NeocitiesError> {
        Ok(if j.get("is_directory").unwrap().as_bool().unwrap() {
            SiteItem::Dir(Dir::from_json(j)?)
        }
        else {
            SiteItem::File(File::from_json(j)?)
        })
    }
}
