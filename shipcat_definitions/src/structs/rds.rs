use super::Result;
use super::Metadata;

/// Supported RDS engines
///
/// Subset of the official [AWS RDS database engines](https://aws.amazon.com/rds/).
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum RdsEngine {
    Postgres,
    Mysql
}

/// RDS Instance types
///
/// Subset of the official [AWS RDS instance type list](https://aws.amazon.com/rds/instance-types/).
/// Current gen (m5 + t3) along with older m4 + t2.
#[derive(Deserialize, Serialize, Clone)]
pub enum InstanceClass {
    // Burstable T2 instances for compat
    #[serde(rename = "db.t2.micro")]
    DbT2micro,
    #[serde(rename = "db.t2.small")]
    DbT2small,
    #[serde(rename = "db.t2.medium")]
    DbT2medium,

    // Burstable T3 instance
    #[serde(rename = "db.t3.micro")]
    DbT3micro,
    #[serde(rename = "db.t3.small")]
    DbT3small,
    #[serde(rename = "db.t3.medium")]
    DbT3medium,

    // General purpose M4 instances for compat
    #[serde(rename = "db.m4.large")]
    DbM4Large,
    #[serde(rename = "db.m4.xlarge")]
    DbM4Xlarge,
    #[serde(rename = "db.m4.2xlarge")]
    DbM42xlarge,
    #[serde(rename = "db.m4.4xlarge")]
    DbM44xlarge,

    // General purpose M5 instances
    #[serde(rename = "db.m5.large")]
    DbM5Large,
    #[serde(rename = "db.m5.xlarge")]
    DbM5Xlarge,
    #[serde(rename = "db.m5.2xlarge")]
    DbM52xlarge,
    #[serde(rename = "db.m5.4xlarge")]
    DbM54xlarge,
}
// TODO: pr to serde for extra case rename type?
// https://github.com/serde-rs/serde/blob/7950f3cdc52d4898aa4195b853cbec12d65bb091/serde_derive/src/internals/case.rs

/// AWS RDS parameters for infrastructure provisioning
///
/// Simplified input for configuring a database for your service.
/// Based loosely on the inputs from
/// [terraform aws_db_instance](https://www.terraform.io/docs/providers/aws/r/db_instance.html).
#[derive(Serialize, Deserialize, Clone)]
pub struct Rds {
    /// Name of service (filled from manifest name)
    #[serde(skip_deserializing)]
    pub name: Option<String>,

    /// Name of team owning the service (filled from manifest)
    #[serde(skip_deserializing)]
    pub team: Option<String>,

    /// The allocated storage in gibibytes
    pub size: u32,

    /// The database engine to use
    ///
    /// E.g. `Postgres`, `
    pub engine: RdsEngine,

    /// The database engine version
    ///
    /// E.g. 9.6 for postgres, or 8.
    pub version: String,

    /// The instance type of the RDS instance
    ///
    /// E.g. db.m4.large
    pub instanceClass: Option<InstanceClass>,

    // TODO: allow customizing backup setup?
}

impl Rds {
    pub fn verify(&self) -> Result<()> {
        if self.size > 20_000 { // gp2 limits for rds
            bail!("Cannot allocate RDS databases larger than than 20 TB")
        }
        if self.size < 20 {
            bail!("Minimum allocatable RDS database is 20 GB") // rds limit
        }
        let _ev = &self.version;
        match self.engine {
            // TODO: do a super-low-level sanity on versions?
            RdsEngine::Postgres => {} // maybe check if ev starts with 9, then at least 9.6
            RdsEngine::Mysql => {} // maybe check that ev is 5.7* or 8.* something
        }

        Ok(())
    }

    pub fn implicits(&mut self, svc: &str, md: &Metadata) {
        // databases named after services
        self.name = Some(svc.into());
        self.team = Some(md.team.clone());
    }
}
