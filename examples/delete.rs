use neoercities::NeocitiesClient;

fn main() {
    let c = NeocitiesClient::new("username", "password"); // create a client
    let info = c.delete("time.txt").unwrap(); // upload the bytes to a file at /time.html
    println!("{}", info.to_string());
}
