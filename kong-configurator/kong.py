#!/usr/bin/env python3
import requests
import json
import argparse
import sys
from typing import Dict, List, Optional


##################################
# Colors
##################################

class bcolors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


##################################
# Argument Parsing
##################################

parser = argparse.ArgumentParser()
parser.add_argument('-c', action='store',
                    dest='kong_config_file',
                    required=True,
                    help='Path to Kong config file')
parser.add_argument('-u', action='store',
                    dest='kong_url',
                    required=True,
                    help='Kong URL')
parser.add_argument('--api', action='store',
                    dest='kong_api',
                    default=None,
                    help='Filter a specific Kong API')
parser.add_argument('--version', action='version',
                    version='%(prog)s 1.0')
results = parser.parse_args()
kong_config_file = results.kong_config_file
kong_endpoint = results.kong_url
kong_api = results.kong_api


##################################
# Config Loading
##################################

with open(kong_config_file) as data_file:
    config = json.load(data_file)
apis = config["apis"]
if kong_api:
    apis = dict((k, v) for k, v in apis.items() if k == kong_api)
kong_config = config["kong"]


def get_current_apis(kong_endpoint):
    r = requests.get(kong_endpoint + '/apis?size=10000')
    return r.json()


def get_current_plugins(kong_endpoint):
    r = requests.get(kong_endpoint + '/plugins?size=10000')
    return r.json()


def get_current_consumers(kong_endpoint):
    r = requests.get(kong_endpoint + '/consumers')
    return r.json()


current_apis = get_current_apis(kong_endpoint)
current_plugins = get_current_plugins(kong_endpoint)
current_consumers = get_current_consumers(kong_endpoint)


##################################
# Functions
##################################

def configure_oauth2(api, kong_endpoint, kong_config, current_plugins):

    # Get OAuth2 API Plugin Metadata if already exists in Kong
    current_plugin = find_plugin(
        plugins=current_plugins["data"],
        api_id=api["id"],
        plugin_name="oauth2"
    )

    if "auth" not in api or api["auth"] != "none":
        oauth2_api_config = {
            "name": "oauth2",
            "enabled": True,
            "config": {
                "mandatory_scope": False,
                "token_expiration": kong_config["kong_token_expiration"],
                "enable_implicit_grant": False,
                "hide_credentials": False,
                "enable_password_grant": True,
                "global_credentials": True,
                "accept_http_if_already_terminated": False,
                "provision_key": kong_config["oauth_provision_key"],
                "enable_client_credentials": False,
                "enable_authorization_code": True,
                "anonymous": api.get("oauth2_anonymous", "")
            }
        }
        # Get Oauth2 API Plugin Metadata if already exists in Kong
        if current_plugin:
            oauth2_api_config["id"] = current_plugin["id"]
            oauth2_api_config["created_at"] = current_plugin["created_at"]
        # Add/Update Oauth2 API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            requests.put(
                kong_plugin_config_endpoint,
                json=oauth2_api_config
            )
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        print("    OAuth2: \t\t\t[{}]".format(
            bcolors.OKGREEN + "OK" + bcolors.ENDC)
        )
    elif current_plugin:
        try:
            response = requests.delete(
                f"{kong_endpoint}/apis/{api['id']}/plugins/{current_plugin['id']}"
            )
            response.raise_for_status()
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])

        print("    OAuth2: \t\t[{}]".format(bcolors.WARNING + "Removed" + bcolors.ENDC))
    else:
        print("    OAuth2: \t\t\t[{}]".format(
            bcolors.WARNING + "None" + bcolors.ENDC)
        )


def configure_correlation_id(api, kong_endpoint, current_plugins):
    correlation_id_api_config = {
        "name": "correlation-id",
        "config": {
            "header_name": "babylon-request-id",
            "generator": "uuid",
            "echo_downstream": True
        }
    }
    # Get Correlation ID API Plugin Metadata if already exists in Kong
    for plugin in current_plugins["data"]:
        if plugin["name"] == 'correlation-id' and plugin["api_id"] == api["id"]:
            correlation_id_api_config["id"] = plugin["id"]
            correlation_id_api_config["created_at"] = plugin["created_at"]
    # Add/Update Correlation ID API Plugin
    kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
    try:
        requests.put(kong_plugin_config_endpoint, json=correlation_id_api_config)
    except Exception:
        print("Unexpected error:", sys.exc_info()[0])
    print("    Correlation ID: \t\t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))


def configure_ip_restriction(api, kong_endpoint, kong_config, current_plugins):
    internal = api.get("internal", False)

    # Get IP Restriction API Plugin Metadata if already exists in Kong
    current_plugin = find_plugin(
        plugins=current_plugins["data"],
        api_id=api["id"],
        plugin_name="ip-restriction"
    )

    if internal:
        additional_internal_ips = api.get("additional_internal_ips", [])
        whitelist = ",".join(kong_config["internal_ips_whitelist"] + additional_internal_ips)
        ip_restriction_api_config = {
            "name": "ip-restriction",
            "enabled": True,
            "config": {
                "whitelist": whitelist,
            }
        }

        if current_plugin:
            ip_restriction_api_config["id"] = current_plugin["id"]
            ip_restriction_api_config["created_at"] = current_plugin["created_at"]

        # Add/Update IP Restriction API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'

        try:
            response = requests.put(kong_plugin_config_endpoint, json=ip_restriction_api_config)
            response.raise_for_status()
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])

        print("    IP Restriction: \t\t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))
    elif current_plugin:
        try:
            response = requests.delete(
                f"{kong_endpoint}/apis/{api['id']}/plugins/{current_plugin['id']}"
            )
            response.raise_for_status()
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])

        print("    IP Restriction: \t\t[{}]".format(bcolors.WARNING + "Removed" + bcolors.ENDC))
    else:
        print("    IP Restriction: \t\t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))


def configure_babylon_auth_header(api, kong_endpoint, current_plugins):
    babylon_auth_header_config = api.get("babylon_auth_header", {})
    babylon_auth_header_enabled = babylon_auth_header_config.get("enabled", False)
    if babylon_auth_header_enabled:
        babylon_auth_header_api_config = {
            "name": "babylon-auth-header",
            "enabled": True,
            "config": {
                "auth_service": babylon_auth_header_config["auth_service"],
                "http_timeout_msec": babylon_auth_header_config["http_timeout_msec"],
                "cache_timeout_sec": babylon_auth_header_config["cache_timeout_sec"]
            }
        }
        # Get Babylon Auth Header API Plugin Metadata if already exists in Kong
        for plugin in current_plugins["data"]:
            if plugin["name"] == 'babylon-auth-header' and plugin["api_id"] == api["id"]:
                babylon_auth_header_api_config["id"] = plugin["id"]
                babylon_auth_header_api_config["created_at"] = plugin["created_at"]
        # Add/Update Babylon Auth Header API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            requests.put(kong_plugin_config_endpoint, json=babylon_auth_header_api_config)
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        print("    Babylon Auth Header: \t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))

    else:
        print("    Babylon Auth Header: \t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))


def configure_json_cookies_to_headers(api, kong_endpoint, current_plugins):
    cookie_to_header_config = {
        "cookie_name": "autologin_token",
        "field_name": "kong_token"
    }
    cookie_to_header_enabled = api.get("cookie_auth", False)
    if cookie_to_header_enabled:
        cookie_to_header_api_config = {
            "name": "json-cookies-to-headers",
            "enabled": True,
            "config": cookie_to_header_config
        }
        # Get Babylon Auth Header API Plugin Metadata if already exists in Kong
        for plugin in current_plugins["data"]:
            if plugin["name"] == 'json-cookies-to-headers' and plugin["api_id"] == api["id"]:
                cookie_to_header_api_config["id"] = plugin["id"]
                cookie_to_header_api_config["created_at"] = plugin["created_at"]
        # Add/Update Babylon Auth Header API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            requests.put(kong_plugin_config_endpoint, json=cookie_to_header_api_config)
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        print("    Cookie to Header: \t\t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))

    else:
        print("    Cookie to Header: \t\t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))

def configure_json_cookies_csrf(api, kong_endpoint, current_plugins):
    cookies_json_csrf_config = {
        "cookie_name": "autologin_info",
        "csrf_field_name": "csrf_token",
        "csrf_header_name": "x-security-token"
    }
    cookies_json_csrf_enabled = api.get("cookie_auth_csrf", False)
    if cookies_json_csrf_enabled:
        json_cookies_csrf_api_config = {
            "name": "json-cookies-csrf",
            "enabled": True,
            "config": cookies_json_csrf_config
        }
        # Get Babylon Auth Header API Plugin Metadata if already exists in Kong
        for plugin in current_plugins["data"]:
            if plugin["name"] == 'json-cookies-csrf' and plugin["api_id"] == api["id"]:
                json_cookies_csrf_api_config["id"] = plugin["id"]
                json_cookies_csrf_api_config["created_at"] = plugin["created_at"]
        # Add/Update Babylon Auth Header API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            requests.put(kong_plugin_config_endpoint, json=json_cookies_csrf_api_config)
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        print("    JSON Cookies Anti-CSRF: \t\t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))

    else:
        print("    JSON Cookies Anti-CSRF: \t\t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))


def configure_oauth2_extension(api, kong_endpoint, current_plugins):
    oauth2_extension_plugin = api.get("oauth2_extension_plugin", False)
    if oauth2_extension_plugin:
        oauth2_extension_api_config = {
            "name": "oauth2-extension",
            "enabled": True,
            "config": {}
        }
        # Get Oauth2 Extension API Plugin Metadata if already exists in Kong
        for plugin in current_plugins["data"]:
            if plugin["name"] == 'ip-restriction' and plugin["api_id"] == api["id"]:
                oauth2_extension_api_config["id"] = plugin["id"]
                oauth2_extension_api_config["created_at"] = plugin["created_at"]
        # Add/Update Oauth2 Extension API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            requests.put(kong_plugin_config_endpoint, json=oauth2_extension_api_config)
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        print("    Oauth2 Extension: \t\t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))

    else:
        print("    Oauth2 Extension: \t\t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))


def configure_cors(api, kong_endpoint, kong_config, current_plugins):
    cors_config = api.get("cors", {})
    cors_enabled = cors_config.get("enabled", False)
    if cors_enabled:
        cors_api_config = {
            "name": "cors",
            "enabled": True,
            "config": {
                "origins": cors_config["origin"],
                "methods": cors_config["methods"],
                "headers": cors_config["headers"],
                "exposed_headers": cors_config["exposed_headers"],
                "credentials": cors_config["credentials"],
                "max_age": cors_config["max_age"],
                "preflight_continue": cors_config["preflight_continue"]
            }
        }
        # Get CORS API Plugin Metadata if already exists in Kong
        for plugin in current_plugins["data"]:
            if plugin["name"] == 'cors' and plugin["api_id"] == api["id"]:
                cors_api_config["id"] = plugin["id"]
                cors_api_config["created_at"] = plugin["created_at"]
        # Add/Update CORS API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            response = requests.put(kong_plugin_config_endpoint, json=cors_api_config)
            if response.status_code > 399:
                print("Unexpected status code %s, error: " % (response.status_code, response.text))
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        print("    CORS: \t\t\t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))
    else:
        print("    CORS: \t\t\t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))


def configure_tcp_log(api, kong_endpoint, kong_config, current_plugins):
    tcp_log_config = kong_config.get("tcp_log", {})
    tcp_log_enabled = tcp_log_config.get("enabled", False)
    if tcp_log_enabled:
        tcp_log_api_config = {
            "name": "tcp-log",
            "enabled": True,
            "config": {
                "host": tcp_log_config.get("host", "127.0.0.1"),
                "port": tcp_log_config.get("port", "9901"),
                "timeout": tcp_log_config.get("timeout", "10000"),
                "keepalive": tcp_log_config.get("keepalive", "60000")
            }
        }
        # Get TCP Log API Plugin Metadata if already exists in Kong
        for plugin in current_plugins["data"]:
            if plugin["name"] == 'tcp-log' and plugin["api_id"] == api["id"]:
                tcp_log_api_config["id"] = plugin["id"]
                tcp_log_api_config["created_at"] = plugin["created_at"]
        # Add/Update TCP Log API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            requests.put(kong_plugin_config_endpoint, json=tcp_log_api_config)
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        print("    TCP Log: \t\t\t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))
    else:
        print("    TCP Log: \t\t\t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))

def configure_response_transformer(api, kong_endpoint, kong_config, current_plugins):
    # Get response-transformer API Plugin Metadata if already exists in Kong
    current_plugin = find_plugin(
        plugins=current_plugins["data"],
        api_id=api["id"],
        plugin_name="response-transformer"
    )

    add_headers = api.get("add_headers", None)
    if add_headers:
        # Normalise map of headers into an array
        headers_array = []
        for (k, v) in add_headers.items():
            headers_array.append(f"{k}: {v}")

        response_transformer_api_config = {
            "name": "response-transformer",
            "enabled": True,
            "config": {
                "add": {
                    "headers": headers_array
                }
            }
        }
        # Get Response Transformer API Plugin Metadata if already exists in Kong
        if current_plugin:
            response_transformer_api_config["id"] = current_plugin["id"]
            response_transformer_api_config["created_at"] = current_plugin["created_at"]
        # Add/Update Response Transformer API Plugin
        kong_plugin_config_endpoint = kong_endpoint + '/apis/' + api["id"] + '/plugins/'
        try:
            requests.put(kong_plugin_config_endpoint, json=response_transformer_api_config)
            print("    Response transformer: \t[{}]".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))
        except:
            print("Unexpected error:", sys.exc_info()[0])
    elif current_plugin:
        try:
            response = requests.delete(
                f"{kong_endpoint}/apis/{api['id']}/plugins/{current_plugin['id']}"
            )
            response.raise_for_status()
            print("    Response transformer: \t[{}]".format(bcolors.WARNING + "Removed" + bcolors.ENDC))
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
    else:
        print("    Response transformer: \t[{}]".format(bcolors.WARNING + "None" + bcolors.ENDC))


def configure_consumer(consumer_config, kong_endpoint, current_consumers, anonymous=False):
    existing_consumer = {
        "username": consumer_config["username"]
    }
    # Check if consumer already exists
    for consumer in current_consumers["data"]:
        if consumer["username"] == consumer_config["username"]:
            existing_consumer["id"] = consumer["id"]
            existing_consumer["created_at"] = consumer["created_at"]
    # Add/Update Consumer
    try:
        r = requests.put(kong_endpoint + '/consumers', data=existing_consumer)
    except Exception:
        print("Unexpected error:", sys.exc_info()[0])
    print("{}: ".format(consumer_config["username"]) + bcolors.OKGREEN + "OK" + bcolors.ENDC)
    if not anonymous:
        consumer_oauth = {
            "name": consumer_config["username"],
            "client_id": consumer_config["oauth_client_id"],
            "client_secret": consumer_config["oauth_client_secret"],
            "redirect_uri": 'http://example.com/unused'
        }
        kong_consumer_oauth = "{}/{}/oauth2".format(kong_endpoint + '/consumers', consumer_config["username"])

        # Check if OAuth Creds already exist
        r = requests.get(kong_consumer_oauth)
        current_oauth_creds = r.json()
        for oauth_cred in current_oauth_creds["data"]:
            if oauth_cred["name"] == consumer_config["username"]:
                consumer_oauth["consumer_id"] = oauth_cred["consumer_id"]
                consumer_oauth["id"] = oauth_cred["id"]
                consumer_oauth["created_at"] = oauth_cred["created_at"]

        # Add/Update OAuth Credentials
        try:
            r = requests.put(kong_consumer_oauth, data=consumer_oauth)
        except Exception:
            print("Unexpected error:", sys.exc_info()[0])
        consumer_oauth["id"] = r.json()["id"]
        consumer_oauth["created_at"] = r.json()["created_at"]
        print("Credentials: {}".format(bcolors.OKGREEN + "OK" + bcolors.ENDC))


def configure_api(api, current_apis, kong_endpoint, kong_config, current_plugins):
    # Configure API
    api_config = {
        "name":                     api["name"],
        "hosts":                    api.get("hosts", ""),
        "uris":                     api.get("uris", ""),
        "methods":                  api.get("methods", ""),
        "upstream_url":             api["upstream_url"],
        "strip_uri":                api.get("strip_uri", False),
        "preserve_host":            api.get("preserve_host", True),
        "retries":                  0,
        "upstream_connect_timeout": api.get("upstream_connect_timeout", 30000),
        "upstream_send_timeout":    api.get("upstream_send_timeout", 30000),
        "upstream_read_timeout":    api.get("upstream_read_timeout", 30000),
        "https_only":               api.get("https_only", False),
        "http_if_terminated":       api.get("http_if_terminated", False)
    }
    # Get API Metadata if already exists in Kong
    for existing_api in current_apis["data"]:
        if existing_api["name"] == api["name"]:
            api_config["id"] = existing_api["id"]
            api_config["created_at"] = existing_api["created_at"]
    # Add/Update API
    try:
        r = requests.put(kong_endpoint + '/apis', data=api_config)
    except Exception:
        print("Unexpected error:", sys.exc_info()[0])
    api["id"] = r.json()["id"]
    api["created_at"] = r.json()["created_at"]
    print("API {}: ".format(api_config["name"]) + bcolors.OKGREEN + "OK" + bcolors.ENDC)
    # Configure Oauth2 Plugin
    configure_oauth2(api, kong_endpoint, kong_config, current_plugins)
    # Configure Correlation ID Plugin
    configure_correlation_id(api, kong_endpoint, current_plugins)
    # Configure IP Restriction Plugin
    configure_ip_restriction(api, kong_endpoint, kong_config, current_plugins)
    # Configure Babylon Auth Header Plugin
    configure_babylon_auth_header(api, kong_endpoint, current_plugins)
    # Configure Oauth2 Extension Plugin
    configure_oauth2_extension(api, kong_endpoint, current_plugins)
    # Configure CORS Plugin
    configure_cors(api, kong_endpoint, kong_config, current_plugins)
    # Configure Cookie to Header Plugin
    configure_json_cookies_to_headers(api, kong_endpoint, current_plugins)
    # Configure Cookie CSRF Plugn
    configure_json_cookies_csrf(api, kong_endpoint, current_plugins)
    # Configure TCP Plugin
    configure_tcp_log(api, kong_endpoint, kong_config, current_plugins)
    # Configure Response Transformer Plugin
    configure_response_transformer(api, kong_endpoint, kong_config, current_plugins)


def find_plugin(
    *, plugins: List[Dict], api_id: str, plugin_name: str,
) -> Optional[Dict]:
    for plugin in plugins:
        if plugin["name"] == plugin_name and plugin["api_id"] == api_id:
            return plugin

    return None


##################################
# Actions
##################################

if __name__ == '__main__':
    print("\n\n- APIs -------------------------------")
    for api, api_config in apis.items():
        print("--------------------------------------")
        configure_api(api_config, current_apis, kong_endpoint, kong_config, current_plugins)
    if "consumers" in kong_config:
        print("\n\n- CONSUMERS --------------------------")
        for key, consumer_config in kong_config["consumers"].items():
            print("--------------------------------------")
            configure_consumer(consumer_config, kong_endpoint, current_consumers)

    if "anonymous_consumers" in kong_config:
        print("\n\n- ANONYMOUS CONSUMERS ----------------")
        for key, consumer_config in kong_config["anonymous_consumers"].items():
            print("--------------------------------------")
            configure_consumer(consumer_config, kong_endpoint, current_consumers, anonymous=True)
