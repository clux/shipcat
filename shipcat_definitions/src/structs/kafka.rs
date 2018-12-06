use crate::region::{Region};


#[derive(Serialize, Deserialize, Clone)]
pub struct Kafka {
    #[serde(default)]
    pub mountPodIP: bool,
    /// Brokers for the region
    #[serde(default)]
    pub brokers: Vec<String>,
}

impl Kafka {
    pub fn implicits(&mut self, _svc: &str, reg: Region) {
        for v in reg.kafka.brokers {
            self.brokers.push(v);
        }
    }
}
