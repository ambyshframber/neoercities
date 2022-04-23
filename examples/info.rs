use neoercities::NeocitiesClient;

fn main() {
    let c = NeocitiesClient::new_no_auth();
    let info = c.info_no_auth("ambyshframber").unwrap(); // if this panics, check your internet connection
    println!("{}", info.to_string());
}