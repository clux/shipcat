use super::{Result};

pub mod version {
    use reqwest::Url;
    use std::collections::HashMap;
    use std::env;
    use super::Result;

    // version fetching stuff
    #[derive(Deserialize)]
    struct Entry {
        //container: String,
        name: String,
        version: String,
    }

    /// Map of service -> versions
    pub type VersionMap = HashMap<String, String>;

    // The actual HTTP GET logic
    pub fn get_all() -> Result<VersionMap> {
        let client = reqwest::Client::new();
        let vurl = Url::parse(&env::var("VERSION_URL")?)?;
        let mut res = client.get(vurl).send()?;
        if !res.status().is_success() {
            bail!("Failed to fetch version");
        }
        let text = res.text()?;
        debug!("Got version data: {}", text);
        let data : Vec<Entry> = serde_json::from_str(&text)?;
        let res = data.into_iter()
            .fold(HashMap::new(), |mut acc, e| {
                acc.insert(e.name, e.version);
                acc
            });
        Ok(res)
    }
}

pub mod sentryapi {
    use super::Result;
    use std::env;

    // Sentry project struct
    #[derive(Deserialize)]
    struct Project {
        slug: String,
        name: String,
    }

    // Get Sentry info!
    pub fn get_slug(sentry_url: &str, env: &str, svc: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let token = env::var("SENTRY_TOKEN")?;

        let projects_url = format!("{sentry_url}/api/0/teams/sentry/{env}/projects/",
                                   sentry_url = &sentry_url,
                                   env = &env);

        let mut res = client
            .get(reqwest::Url::parse(&projects_url).unwrap())
            .header("Authorization", format!("Bearer {token}", token = token))
            .send()?;

        if !res.status().is_success() {
            bail!("Failed to fetch projects in team {}", env);
        }
        let text = res.text()?;
        debug!("Got data: {}", text);
        let data : Vec<Project> = serde_json::from_str(&text)?;
        if let Some(entry) = data.into_iter().find(|r| r.name == svc) {
            Ok(entry.slug)
        } else {
            bail!("Project {} not found in team {}, {}", svc, env, sentry_url)
        }
    }
}


pub mod newrelic {
    use super::Result;
    use std::env;

    // NewRelic Applications info
    #[derive(Deserialize)]
    struct Application {
        id: u32,
        name: String,
    }
    #[derive(Deserialize)]
    struct Applications {
        applications: Vec<Application>,
    }

    // Get NewRelic link
    pub fn get_link(region: &str, svc: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let api_key = env::var("NEWRELIC_API_KEY")?;
        let account_id = env::var("NEWRELIC_ACCOUNT_ID")?;

        let search = format!("{svc} ({region})", svc = svc, region = region);
        let mut res = client
            .get("https://api.newrelic.com/v2/applications.json")
            .query(&[("filter[name]", search.clone())])
            .header("X-Api-Key", api_key)
            .send()?;

        if !res.status().is_success() {
            bail!("Failed to fetch applications");
        }
        let text = res.text()?;
        debug!("Got NewRelic data: {}", text);
        let data : Applications = serde_json::from_str(&text)?;
        if let Some(entry) = data.applications.into_iter().find(|r| r.name == search) {
            debug!("Application found!");
            Ok(format!(
                    "https://rpm.newrelic.com/accounts/{account_id}/applications/{application_id}",
                    account_id = account_id,
                    application_id = entry.id))
        } else {
            bail!("Application {} not found in NewRelic", &svc)
        }
    }
}
