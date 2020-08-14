pub mod sentryapi {
    use crate::Result;
    use std::collections::BTreeMap;

    // Sentry project struct
    #[derive(Deserialize)]
    struct Project {
        slug: String,
        name: String,
    }

    /// Service -> Link
    pub type SentryMap = BTreeMap<String, String>;

    // Get Sentry info
    pub async fn get_slugs(sentry_url: &str, env: &str) -> Result<SentryMap> {
        let client = reqwest::Client::new();
        let token = std::env::var("SENTRY_TOKEN")?;

        let projects_url = format!(
            "{sentry_url}/api/0/teams/sentry/{env}/projects/",
            sentry_url = &sentry_url,
            env = &env
        );

        debug!("Fetching {}", projects_url);
        let res = client
            .get(reqwest::Url::parse(&projects_url).unwrap())
            .header("Authorization", format!("Bearer {token}", token = token))
            .send()
            .await?;

        if !res.status().is_success() {
            bail!("Failed to fetch projects in {}: {}", env, res.status());
        }
        let text = res.text().await?;
        debug!("Got slugs: {}", text);
        let data: Vec<Project> = serde_json::from_str(&text)?;
        let res = data.into_iter().fold(BTreeMap::new(), |mut acc, e| {
            acc.insert(e.name, e.slug);
            acc
        });
        Ok(res)
    }
}

pub mod newrelic {
    use crate::Result;
    use std::collections::BTreeMap;
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

    /// Service -> Link
    pub type RelicMap = BTreeMap<String, String>;

    // Get NewRelic link
    pub async fn get_links(region: &str) -> Result<RelicMap> {
        let client = reqwest::Client::new();
        let api_key = std::env::var("NEWRELIC_API_KEY")?;
        let account_id = std::env::var("NEWRELIC_ACCOUNT_ID")?;

        let search = format!("({region})", region = region);
        let res = client
            .get("https://api.newrelic.com/v2/applications.json")
            .query(&[("filter[name]", search)])
            .header("X-Api-Key", api_key)
            .send()
            .await?;

        if !res.status().is_success() {
            bail!("Failed to fetch applications: {}", res.status());
        }
        let text = res.text().await?;
        debug!("Got NewRelic data: {}", text);
        let data: Applications = serde_json::from_str(&text)?;
        let res = data.applications.into_iter().fold(BTreeMap::new(), |mut acc, e| {
            let link = format!(
                "https://rpm.newrelic.com/accounts/{account_id}/applications/{application_id}",
                account_id = account_id,
                application_id = e.id
            );
            let splits: Vec<_> = e.name.split(' ').collect();
            acc.insert(splits[0].to_string(), link);
            acc
        });
        Ok(res)
    }
}
