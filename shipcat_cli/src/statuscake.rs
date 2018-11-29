use shipcat_definitions::structs::Kong;
use super::{Manifest, Result, Region, Config};

/// One Statuscake object
#[derive(Serialize)]
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
    fn new(region: &Region, name: String, external_svc: String, kong: Kong) -> Self {
        let website_name = format!("{} {} healthcheck", region.name, name);
        // TODO HANDLE HOSTS
        let website_url = format!("{}/status/{}/health",
                                 external_svc,
                                 name);

        // Generate tags, both regional and environment
        let mut test_tags = [
            region.name.clone(),
            region.environment.clone()
        ].join(",");

        let mut contact_group = None;
        // Process extra region-specific config
        // Set the Contact group if available
        if let Some(ref conf) = region.statuscake {
            contact_group = conf.contact_group.clone();
            if let Some(ref region_tags) = conf.extra_tags {
                test_tags = [
                    test_tags,
                    region_tags.to_string()
                ].join(",");
            }
        };

        StatuscakeTest {
            name: name.into(),
            WebsiteName: website_name,
            WebsiteURL: website_url,
            ContactGroup: contact_group,
            TestTags: Some(test_tags),
        }
    }
}

fn generate_statuscake_output(conf: &Config, region: &Region) -> Result<Vec<StatuscakeTest>> {
    let mut tests = Vec::new();

    // Ensure the region has a base_url
    if let Some(external_svc) = region.base_urls.get("external_services") {
        debug!("Using base_url.external_services {:?}", external_svc);
        // Generate list of APIs to feed to Statuscake
        for svc in Manifest::available(&region.name)? {
            debug!("Scanning service {:?}", svc);
            let mf = Manifest::simple(&svc, &conf, region)?; // does not need secrets
            debug!("Found service {} in region {}", mf.name, region.name);
            if let Some(k) = mf.kong.clone() {
                tests.push(StatuscakeTest::new(
                        region,
                        svc,
                        external_svc.to_string(),
                        k));
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
