use std::io::{self, Write};

use super::{Manifest, Result, Region, Config};

/// One Statuscake object
#[derive(Serialize, Deserialize)]
struct StatuscakeTest {
    pub name: String,
    pub WebsiteName: String,
    pub WebsiteURL: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ContactGroup: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub TestTags: Option<String>,
}

impl StatuscakeTest {
    fn new(region: &Region, name: String, kong: Kong) -> Self {
        let websiteName = format!("{} {} healthcheck", region.name, name);
        // TODO HANDLE HOSTS
        let websiteUrl = format!("{}/status/{}/health",
                                 region.base_urls.get("external_services").unwrap(),
                                 name);

        // Generate tags, both regional and environment
        let testTags = [
            region.clone().name,
            region.clone().environment
        ].join(",");

        // Set the Contact group to production if a prod env
        let contactGroup = if region.environment == "prod" {
            Some("34145".into())
        } else {
            None
        };

        StatuscakeTest {
            name: name.into(),
            WebsiteName: websiteName,
            WebsiteURL: websiteUrl,
            ContactGroup: contactGroup,
            TestTags: Some(testTags),
        }
    }
}

fn generate_statuscake_output(conf: &Config, region: &Region) -> Result<Vec<StatuscakeTest>> {
    let mut tests = Vec::new();

    // Generate list of APIs to feed to Statuscake
    for svc in Manifest::available(&region.name)? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::simple(&svc, &conf, region)?; // does not need secrets
        debug!("Found service {} in region {}", mf.name, region.name);
        if let Some(k) = mf.kong.clone() {
           tests.push(StatuscakeTest::new(
                   region,
                   svc,
                   k));
        }
    }

    // Extra APIs - let's not monitor them for now (too complex)

    //for (name, api) in region.kong.extra_apis.clone() {
    //    apis.insert(name, api);
    //}

    Ok(tests)
}

/// Generate Statuscake config from a filled in global config
pub fn output(conf: &Config, region: &Region) -> Result<()> {
    let res = generate_statuscake_output(&conf, &region)?;
    let output = serde_yaml::to_string(&res)?;
    let _ = io::stdout().write(format!("{}\n", output).as_bytes());

    Ok(())
}
