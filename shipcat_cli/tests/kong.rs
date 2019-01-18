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
    let output = KongfigOutput::new(kongrs);

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


    assert_eq!(output.apis.len(), 1);
    let api = &output.apis[0];
    assert_eq!(api.name, "fake-ask");
    assert_eq!(api.attributes.uris, Some(vec!["/ai-auth".to_string()]));
    assert_eq!(api.attributes.strip_uri, false);
    assert_eq!(api.attributes.upstream_url, "http://fake-ask.dev.svc.cluster.local");
    assert_eq!(api.plugins.len(), 4);

    // api plugins
    let attr = plugin_attributes!("CorrelationId", &api.plugins[0], ApiPlugin::CorrelationId);
    assert_eq!(attr.enabled, true);
    assert_eq!(attr.config.header_name, "babylon-request-id");

    let attr = plugin_attributes!("TcpLog", &api.plugins[1], ApiPlugin::TcpLog);
    assert_eq!(attr.enabled, true);

    let attr = plugin_attributes!("Oauth2", &api.plugins[2], ApiPlugin::Oauth2);
    assert_eq!(attr.enabled, true);
    assert_eq!(attr.config.global_credentials, true);
    assert_eq!(attr.config.provision_key, "key");
    assert_eq!(attr.config.anonymous, Some("".to_string()));
    assert_eq!(attr.config.token_expiration, 1800);

    assert_plugin_removed!("Jwt", &api.plugins[3], ApiPlugin::Jwt);
}
