use crate::region::Region;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Kafka {
    #[serde(default)]
    pub mountPodIP: bool,

    /// Brokers for the region
    ///
    /// ```yaml
    /// brokers: kafka.babylontech.co.uk:92101,kafka.babylontech.co.uk:92102
    #[serde(default)]
    pub brokers: Vec<String>,

    /// Zookeeper ensemble for the region
    ///
    /// ```yaml
    /// zk: zk.babylontech.co.uk:21811,zk.babylontech.co.uk:21812
    #[serde(default)]
    pub zk: Vec<String>,

    /// A mapping of kafka properties to environment variables.
    ///
    /// ```yaml
    /// property_env_mapping:
    ///   sasl.enabled.mechanisms: KAKFA_SASL_ENABLED_MECHANISMS
    ///   sasl.jaas.config:        KAFKA_SASL_JAAS_CONFIG
    ///   ssl.keystore.password:   KAFKA_SSL_KEYSTORE_PASSWORD
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propertyEnvMapping: Option<BTreeMap<String, String>>,
}

impl Kafka {
    pub fn implicits(&mut self, _svc: &str, reg: Region) {
        for v in reg.kafka.brokers {
            self.brokers.push(v);
        }
        for v in reg.kafka.zk {
            self.zk.push(v);
        }
    }
}
