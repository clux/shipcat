# Kong Configurator

## Installation

Set up virtual environment.

```
virtualenv -p python3 venv
```

Install dependencies.

```
pip install -r requirements.txt
```

## Usage

Configure Kong with a config file:

```
python kong.py -c KONG_CONFIG_FILE -u KONG_URL
```

Get script version:

```
python kong.py --version
```

Get help:

```
python kong.py -h
```

## Current support

This script currently supports the following configuration:

- Add/Update API
- Add/Update Consumers
- Add/Update Anonymous Consumers
- Add/Update the following plugins:
    - babylon-auth-header
    - oauth2
    - oauth2-extension
    - cors
    - ip-restriction
    - tcp-log
    - correlation-id

## Config file

The configuration file per environment is a `json` file, which structure must be the following:

```
{
  "apis": {
    "myapi": {
      "name": "myapi",
      "uris": "/myapi",
      "upstream_url": "http://myapi.dev"
    },
    ...
  },
  "kong": {
    "internal_ips_whitelist": [
      "X.X.X.X",
      "Y.Y.Y.Y",
      "Z.Z.Z.Z"
    ],
    "kong_token_expiration": 1800,
    "oauth_provision_key": "THISISTHEPROVISIONKEY",
    "consumers": {
      "myconsumer": {
        "username": "myconsumer",
        "oauth_client_id": "MYCONSUMERID",
        "oauth_client_secret": "MYCONSUMERSECRET"
      },
      ...
    },
    "anonymous_consumers": {
      "anonymous": {
        "username": "anonymous",
        "id": "IDIDIDIDIDIDIDID"
      }
    }
  }
}

```

It is generated using `shipcat kong`.
