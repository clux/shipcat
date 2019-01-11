use super::Result;

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
#[derive(Deserialize, Serialize, Clone)]
pub enum InstanceClass {
    #[serde(rename = "db.t2.medium")]
    DbT2Medium,

    #[serde(rename = "db.m4.large")]
    DbM4Large,

    #[serde(rename = "db.m4.xlarge")]
    DbM4Xlarge,

    #[serde(rename = "db.m4.4xlarge")]
    DbM44xlarge,
}

// TODO: maybe force this explicit?
// this was a standard.
impl Default for InstanceClass {
    fn default() -> Self { InstanceClass::DbM4Large }
}

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
    /// Defaults to db.m4.large
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

    pub fn implicits(&mut self, svc: &str) {
        // databases named after services
        self.name = Some(svc.into());
        self.instanceClass = Some(InstanceClass::default());
    }
}
