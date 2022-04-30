# neoercities

Another simple Neocities API wrapper.

## Usage:

Create a [`NeocitiesClient`] either with or without authentication
(no-auth clients have very limited functionality).

```rust
let client1 = NeocitiesClient::new("randomuser", "notmypassword");
let client2 = NeocitiesClient::new_with_key(&key);
let client3 = NeocitiesClient::new_no_auth();
```

From there, you can talk to the Neocities API at your leisure.

```rust
let info = client1.info();
client2.upload("site/file.txt", "file.txt");
let someone_elses_info = client3.info_no_auth("ambyshframber");
// this is the only method that no-auth clients can call
```

The crate also includes an optional utility module for dealing with site file lists. Enable the `site_info` feature to use it.
