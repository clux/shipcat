use config::{Region, HostPort};

#[derive(Serialize, Deserialize, Clone)]
pub struct Kafka {
    /// Brokers for the region
    #[serde(default)]
    pub brokers: Vec<HostPort>,
}

impl Kafka {
    pub fn implicits(&mut self, _svc: &str, reg: Region) {
        for v in reg.kafka.brokers {
            self.brokers.push(v);
        }
    }
}
