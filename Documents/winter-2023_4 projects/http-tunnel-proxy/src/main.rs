
use tokio;

mod dns_resolver;
use crate::dns_resolver::DNSResolver;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let dns_resolver = DNSResolver::new(60);
}
