use neoercities::NeocitiesClient;
use chrono::Utc;

fn main() {
    let time = Utc::now().to_rfc2822().bytes().collect(); // get the time and turn it into a vec of bytes
    let c = NeocitiesClient::new("username", "password"); // create a client
    let info = c.upload_bytes(time, "time.txt").unwrap(); // upload the bytes to a file at /time.html
    println!("{}", info.to_string());
}
