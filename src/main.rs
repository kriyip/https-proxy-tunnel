mod dns_resolver;
mod tunnel;
mod utils;
use crate::dns_resolver::DNSResolver;
use tokio;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let dns_resolver = DNSResolver::new(60);
}
