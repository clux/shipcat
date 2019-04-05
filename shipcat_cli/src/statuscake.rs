use shipcat_definitions::structs::Kong;
use super::{Result, Region, Config};

/// One Statuscake object
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct StatuscakeTest {
    #[serde(rename = "name")]
    pub name: String,
    pub website_name: String,
    #[serde(rename = "WebsiteURL")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_group: Option<String>,
    pub test_tags: String,
}

impl StatuscakeTest {
    fn new(region: &Region, name: String, external_svc: String, kong: Kong) -> Self {
        let website_name = format!("{} {} healthcheck", region.name, name);

        // Generate the URL to test
        let website_url = if let Some(host) = kong.hosts.first() {
            Some(format!("https://{}/health", host))
        } else if let Some(uris) = kong.uris {
            Some(format!("{}/status/{}/health",
                    external_svc,
                    uris.trim_start_matches("/")))
        } else {
            // No host, no uri, what's going on?
            None
        };

        // Generate tags, both regional and environment
        let mut test_tags = format!("{},{}",
            region.name,
            region.environment.to_string());

        // Process extra region-specific config
        // Set the Contact group if available
        let contact_group = if let Some(ref conf) = region.statuscake {
            if let Some(ref region_tags) = conf.extra_tags {
                test_tags = format!("{},{}", test_tags, region_tags);
            }
            conf.contact_group.clone()
        } else {
            None
        };

        StatuscakeTest {
            name,
            website_name,
            website_url,
            contact_group,
            test_tags,
        }
    }
}

fn generate_statuscake_output(conf: &Config, region: &Region) -> Result<Vec<StatuscakeTest>> {
    let mut tests = Vec::new();

    // Ensure the region has a base_url
    if let Some(external_svc) = region.base_urls.get("external_services") {
        debug!("Using base_url.external_services {:?}", external_svc);
        // Generate list of APIs to feed to Statuscake
        for mf in shipcat_filebacked::available(conf, region)? {
            debug!("Found service {:?}", mf);
            if let Some(k) = mf.kong.clone() {
                debug!("{:?} has a kong configuration, adding", mf);
                tests.push(StatuscakeTest::new(
                        region,
                        mf.base.name.to_string(),
                        external_svc.to_string(),
                        k));
            } else {
                debug!("{:?} has no kong configuration, skipping", mf);
            }
        }
        // Extra APIs - let's not monitor them for now (too complex)

        //for (name, api) in region.kong.extra_apis.clone() {
        //    apis.insert(name, api);
        //}

    } else {
        bail!("base_url.external_services is not defined for region {}", region.name);
    }

    Ok(tests)
}

/// Generate Statuscake config from a filled in global config
pub fn output(conf: &Config, region: &Region) -> Result<()> {
    let res = generate_statuscake_output(&conf, &region)?;
    let output = serde_yaml::to_string(&res)?;
    println!("{}", output);

    Ok(())
}
