mod common;
use crate::common::setup;

use shipcat::kong::{KongfigOutput, generate_kong_output};
use shipcat_definitions::structs::kongfig::{ConsumerCredentials, PluginBase, ApiPlugin};
use shipcat_definitions::Config;
use shipcat_definitions::ConfigType;

macro_rules! plugin_attributes {
    ( $name:expr, $plugin:expr, $type:path ) => {
        match $plugin {
            $type(PluginBase::Present(attributes)) => attributes,
            $type(PluginBase::Removed) => panic!("{} plugin is removed", $name),
            _ => panic!("plugin is not a {} plugin", $name),
        }
    };
}

macro_rules! assert_plugin_removed {
    ( $name:expr, $plugin:expr, $type:path ) => {
        match $plugin {
            $type(PluginBase::Removed) => {},
            $type(_) => panic!("{} plugin is not removed", $name),
            _ => panic!("plugin is not a {} plugin", $name),
        }
    };
}

#[test]
fn kong_test() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let kongrs = generate_kong_output(&conf, &reg).unwrap();
    let mut output = KongfigOutput::new(kongrs, &reg);

    assert_eq!(output.host, "admin.dev.something.domain.com");

    assert_eq!(output.consumers.len(), 3);
    assert_eq!(output.consumers[0].username, "fake-ask");
    assert_eq!(output.consumers[0].credentials.len(), 1);
    if let ConsumerCredentials::OAuth2(attrs) = &output.consumers[0].credentials[0] {
        assert_eq!(attrs.client_id, "FAKEASKID");
        assert_eq!(attrs.client_secret, "FAKEASKSECRET");
    } else {
        panic!("Not an OAuth2 credential")
    }

    assert_eq!(output.consumers[1].username, "my-idp");
    assert_eq!(output.consumers[1].credentials.len(), 1);
    if let ConsumerCredentials::Jwt(attrs) = &output.consumers[1].credentials[0] {
        assert_eq!(attrs.key, "https://my-issuer/");
        assert_eq!(attrs.algorithm, "RS256");
        assert_eq!(attrs.rsa_public_key, "-----BEGIN PUBLIC KEY-----\nmy-key\n-----END PUBLIC KEY-----");
    } else {
        panic!("Not a JWT credential")
    }

    assert_eq!(output.consumers[2].username, "anonymous");
    assert!(output.consumers[2].credentials.is_empty());


    assert_eq!(output.apis.len(), 2);

    // fake-ask API
    let mut api = output.apis.remove(0);
    assert_eq!(api.name, "fake-ask");
    assert_eq!(api.attributes.uris, Some(vec!["/ai-auth".to_string()]));
    assert_eq!(api.attributes.strip_uri, false);
    assert_eq!(api.attributes.upstream_url, "http://fake-ask.dev.svc.cluster.local");

    let attr = plugin_attributes!("CorrelationId", api.plugins.remove(0), ApiPlugin::CorrelationId);
    assert_eq!(attr.enabled, true);
    assert_eq!(attr.config.header_name, "babylon-request-id");

    let attr = plugin_attributes!("TcpLog", api.plugins.remove(0), ApiPlugin::TcpLog);
    assert_eq!(attr.enabled, true);

    let attr = plugin_attributes!("Oauth2", api.plugins.remove(0), ApiPlugin::Oauth2);
    assert_eq!(attr.enabled, true);
    assert_eq!(attr.config.global_credentials, true);
    assert_eq!(attr.config.provision_key, "key");
    assert_eq!(attr.config.anonymous, Some("".to_string()));
    assert_eq!(attr.config.token_expiration, 1800);

    assert_plugin_removed!("Jwt", api.plugins.remove(0), ApiPlugin::Jwt);
    assert_plugin_removed!("JwtValidator", api.plugins.remove(0), ApiPlugin::JwtValidator);

    assert!(api.plugins.is_empty());

    // fake-storage API
    let mut api = output.apis.remove(0);
    assert_eq!(api.name, "fake-storage");
    assert_eq!(api.attributes.uris, Some(vec!["/fake-storage".to_string()]));
    assert_eq!(api.attributes.strip_uri, false);
    assert_eq!(api.attributes.upstream_url, "http://fake-storage.dev.svc.cluster.local");
    assert_eq!(api.plugins.len(), 8);

    let attr = plugin_attributes!("CorrelationId", api.plugins.remove(0), ApiPlugin::CorrelationId);
    assert_eq!(attr.enabled, true);
    assert_eq!(attr.config.header_name, "babylon-request-id");

    let attr = plugin_attributes!("TcpLog", api.plugins.remove(0), ApiPlugin::TcpLog);
    assert_eq!(attr.enabled, true);

    assert_plugin_removed!("Oauth2", api.plugins.remove(0), ApiPlugin::Oauth2);
    assert_plugin_removed!("Oauth2Extension", api.plugins.remove(0), ApiPlugin::Oauth2Extension);

    let attr = plugin_attributes!("Jwt", api.plugins.remove(0), ApiPlugin::Jwt);
    assert_eq!(attr.enabled, true);
    assert_eq!(attr.config.uri_param_names, Vec::<String>::new());
    assert_eq!(attr.config.claims_to_verify, vec!["exp"]);
    assert_eq!(attr.config.key_claim_name, "kid");
    assert_eq!(attr.config.anonymous, Some("".to_string()));

    let attr = plugin_attributes!("JwtValidator", api.plugins.remove(0), ApiPlugin::JwtValidator);
    assert_eq!(attr.enabled, true);
    assert_eq!(attr.config.allowed_audiences, vec!["https://babylonhealth.com"]);
    assert_eq!(attr.config.expected_region, "dev-uk");
    assert_eq!(attr.config.expected_scope, "internal");
    assert_eq!(attr.config.remove_invalid_tokens, true);

    assert_plugin_removed!("JsonCookiesToHeaders", api.plugins.remove(0), ApiPlugin::JsonCookiesToHeaders);
    assert_plugin_removed!("JsonCookiesCsrf", api.plugins.remove(0), ApiPlugin::JsonCookiesCsrf);

    assert!(api.plugins.is_empty());
}
