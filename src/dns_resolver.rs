use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use trust_dns_resolver::config::*;
use trust_dns_resolver::TokioAsyncResolver;

pub struct DNSRecord {
    ip: Vec<IpAddr>,
    valid_until: Instant,
}

#[derive(Clone)]
pub struct DNSResolver {
    records: Arc<RwLock<HashMap<String, DNSRecord>>>,
    ttl: Duration,
    tokio_resolver: TokioAsyncResolver,
}

impl DNSResolver {
    pub fn new(ttl: u64) -> Self {
        // TokioAsyncResolver::tokio_from_system_conf().await.unwrap() // uses system DNS resolver instead of default
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl),
            tokio_resolver: TokioAsyncResolver::tokio(
                ResolverConfig::default(),
                ResolverOpts::default(),
            )
            .unwrap(),
        }
    }

    pub fn check_cache(&self, domain: &str) -> Option<Vec<IpAddr>> {
        let records = self.records.read().unwrap();
        if let Some(record) = records.get(domain) {
            if record.valid_until > Instant::now() {
                return Some(record.ip.clone());
            }
        }
        None
    }

    pub fn update_cache(&self, domain: &str, ip: Vec<IpAddr>) {
        let mut records = self.records.write().unwrap();
        records.insert(
            domain.to_string(),
            DNSRecord {
                ip: ip,
                valid_until: Instant::now() + self.ttl,
            },
        );
    }

    pub async fn cleanup_expired_records(&self) {
        let mut records = self.records.write().unwrap();
        records.retain(|_, record| record.valid_until > Instant::now());
    }

    pub async fn resolve_domain(&self, domain: &str) -> Result<Vec<IpAddr>> {
        // cleanup stale data
        self.cleanup_expired_records().await;

        if let Some(ip) = self.check_cache(domain) {
            return Ok(ip);
        }

        match self.tokio_resolver.lookup_ip(domain).await {
            Ok(result) => {
                let resolved_ips: Vec<IpAddr> = result.iter().map(|ip| ip.into()).collect();
                if !resolved_ips.is_empty() {
                    self.update_cache(domain, resolved_ips.clone());
                }
                Ok(resolved_ips)
            }
            Err(err) => Err(Error::new(ErrorKind::Other, err)),
        }
    }
}

#[cfg(test)]
mod test {
    use tokio::sync::futures;

    use super::*;

    #[tokio::test]
    async fn test_successful_resolution() {
        let dns_resolver = DNSResolver::new(60);
        let domain = "www.google.com";

        let ips = dns_resolver.resolve_domain(domain).await.unwrap();
        assert!(!ips.is_empty(), "Should resolve to at least one IP");
        assert_eq!(dns_resolver.check_cache(domain).unwrap(), ips);
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let dns_resolver = DNSResolver::new(1);
        let domain = "www.google.com";

        // Perform the first resolution, which should populate the cache.
        let first_resolution = dns_resolver.resolve_domain(domain).await.unwrap();
        assert!(!first_resolution.is_empty());

        // Perform the second resolution, which should hit the cache.
        let second_resolution = dns_resolver.resolve_domain(domain).await.unwrap();
        assert_eq!(
            first_resolution, second_resolution,
            "Subsequent resolutions should hit the cache"
        );
        assert_eq!(dns_resolver.check_cache(domain).unwrap(), second_resolution);
        assert_eq!(dns_resolver.records.read().unwrap().len(), 1);

        // Wait for the cache to expire.
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Perform the third resolution, which should not hit the cache.
        dns_resolver.cleanup_expired_records().await;
        assert_eq!(dns_resolver.records.read().unwrap().len(), 0);
        let _third_resolution = dns_resolver.resolve_domain(domain).await.unwrap();
        assert_eq!(dns_resolver.records.read().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_failed_resolution() {
        let dns_resolver = DNSResolver::new(60);
        let domain = "www.google.com.invalid";

        let ips = dns_resolver.resolve_domain(domain).await;
        assert!(ips.is_err(), "Should fail to resolve");
        assert!(dns_resolver.check_cache(domain).is_none());
    }

    // test for small servers with one IP
    #[tokio::test]
    async fn test_concurrent_resolutions() {
        let dns_resolver = DNSResolver::new(60);
        let domain = "127.0.0.1";

        let mut handles = Vec::new();
        for _ in 0..10 {
            let dns_resolver = dns_resolver.clone();
            let domain = domain.to_string();
            handles.push(tokio::spawn(async move {
                dns_resolver.resolve_domain(&domain).await.unwrap()
            }));
        }

        let mut ips = Vec::new();
        for handle in handles {
            ips.push(handle.await.unwrap());
        }

        assert_eq!(ips.len(), 10, "Should resolve to 10 IPs");
        println!("{:?}", ips);
        println!("len of ips: {}", ips.len());
        assert_eq!(
            dns_resolver.check_cache(domain).unwrap(),
            ips[0],
            "All resolutions should hit the cache"
        );
        assert_eq!(
            dns_resolver.records.read().unwrap().len(),
            1,
            "Should only have one entry in the cache"
        );
    }

    // test for big servers with multiple IPs
    #[tokio::test]
    async fn test_concurrent_resolutions_2() {
        let dns_resolver = DNSResolver::new(60);
        let domain = "www.google.com";

        let mut handles = Vec::new();
        for _ in 0..10 {
            let dns_resolver = dns_resolver.clone();
            let domain = domain.to_string();
            handles.push(tokio::spawn(async move {
                dns_resolver.resolve_domain(&domain).await.unwrap()
            }));
        }

        for handle in handles {
            assert!(
                !handle.await.unwrap().is_empty(),
                "Should resolve to at least one IP"
            );
        }
    }

    #[tokio::test]
    async fn test_concurrent_resolutions_diff_domains() {
        let dns_resolver = DNSResolver::new(60);
        let domains = vec!["www.google.com", "www.facebook.com", "www.twitter.com"];

        let mut handles = Vec::new();
        for domain in domains.clone() {
            let dns_resolver = dns_resolver.clone();
            let domain = domain.to_string();
            handles.push(tokio::spawn(async move {
                dns_resolver.resolve_domain(&domain).await.unwrap()
            }));
        }

        let mut ips = Vec::new();
        for handle in handles {
            ips.push(handle.await.unwrap());
        }

        assert_eq!(ips.len(), 3);
        assert_eq!(dns_resolver.check_cache(domains[0]).unwrap(), ips[0]);
        assert_eq!(dns_resolver.check_cache(domains[1]).unwrap(), ips[1]);
        assert_eq!(dns_resolver.check_cache(domains[2]).unwrap(), ips[2]);
        assert_eq!(dns_resolver.records.read().unwrap().len(), 3);
    }
}
