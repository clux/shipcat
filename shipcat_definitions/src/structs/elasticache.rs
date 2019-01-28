//! Minimal abstraction around aws elasticache
//! Supports redis with cluster mode disabled (single shard - up to 5 read replicas)
//! https://docs.aws.amazon.com/AmazonElastiCache/latest/red-ug/Replication.Redis-RedisCluster.html

use super::Result;
use super::Metadata;

/// ElastiCache Node Types
///
/// Subset of the official [AWS ElastiCache node type list](https://aws.amazon.com/elasticache/pricing/).
/// Only current generation (m5 + r5) + along with cheap t2 nodes
#[derive(Deserialize, Serialize, Clone)]
pub enum NodeType {
    // Cheap t2 nodes
    #[serde(rename = "cache.t2.micro")]
    CacheT2micro,
    #[serde(rename = "cache.t2.small")]
    CacheT2small,
    #[serde(rename = "cache.t2.medium")]
    CacheT2medium,

    // General purpose m5
    #[serde(rename = "cache.m5.large")]
    CacheM5large,
    #[serde(rename = "cache.m5.xlarge")]
    CacheM5xlarge,
    #[serde(rename = "cache.m5.2xlarge")]
    CacheM52xlarge,
    #[serde(rename = "cache.m5.4xlarge")]
    CacheM54xlarge,

    // Memory optimized
    #[serde(rename = "cache.r5.large")]
    CacheR5large,
    #[serde(rename = "cache.r5.xlarge")]
    CacheR5xlarge,
    #[serde(rename = "cache.r5.2xlarge")]
    CacheR52xlarge,
    #[serde(rename = "cache.r5.4xlarge")]
    CacheR54xlarge,
}

/// AWS ElastiCache parameters for infrastructure provisioning
///
/// Simplified input for configuring a single instance Redis for your service.
/// Based loosely on the inputs from:
/// - [aws elasticache cluster replication](https://docs.aws.amazon.com/AmazonElastiCache/latest/red-ug/Replication.Redis-RedisCluster.html)
/// - [aws elasticache cluster replication groups](https://docs.aws.amazon.com/AmazonElastiCache/latest/red-ug/Replication.CreatingReplGroup.ExistingCluster.html)
/// - [terraform aws_elasticache_replication_group](https://www.terraform.io/docs/providers/aws/r/elasticache_replication_group.html)
#[derive(Serialize, Deserialize, Clone)]
pub struct ElastiCache {
    /// Name of service (filled from manifest name)
    #[serde(skip_deserializing)]
    pub name: Option<String>,

    /// Name of team owning the service (filled from manifest)
    #[serde(skip_deserializing)]
    pub team: Option<String>,

    /// Number of nodes (master + read replicas)
    ///
    /// Sometimes referred to as num cache clusters (in cluster mode disabled).
    /// This number includes the master/shard.
    ///
    /// Must be between an integer `1` and `6`.
    pub nodes: Option<u8>,

    /// The node type of the ElastiCache instance
    ///
    /// E.g. cache.m4.large
    pub nodeType: Option<NodeType>,
}

impl ElastiCache {
    pub fn verify(&self) -> Result<()> {
        let num = self.nodes.unwrap(); // must exist by implicits
        if num < 1 {
            bail!("Need at least 1 node (cluster includes the master)")
        }
        if num > 6 {
            bail!("Need less than 6 nodes (non-cluster mode has max 5 read replicas)")
        }
        Ok(())
    }

    pub fn implicits(&mut self, svc: &str, md: &Metadata) {
        // caches named after services
        self.name = Some(svc.into());
        self.team = Some(md.team.clone());
    }
}
