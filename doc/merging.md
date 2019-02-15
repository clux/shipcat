# Manifest Merging

The final manifest for a service is formed by merging together various sources.

## Sources

If there are multiple manifest sources for a service, they are reduced by merging each source into the previous. The sources are as follows (from highest precedence to lowest):

1. Service's region-specific configuration (`services/$service/$region.yml`)
1. Service's environment-specific configuration (`services/$service/$environment.yml`)
1. Service's configuration (`services/$service/shipcat.yml`)
1. Region configuration (from the current region in `shipcat.conf`)
1. Global configuration (from the global configuration in `shipcat.conf`)

## Rules

_See [`Manifest#merge`](../shipcat_definitions/src/merge.rs) for the full logic of two manifest sources are merged.

Some properties are global, so must only be in the service's root manifest (`shipcat.yml`):
- `name`
- `regions`
- `metadata`

For other properties, merging logic depends on type:
* For optional properties (e.g., `version`), the value is overridden if set in the override manifest.
* For list properties (e.g., `sidecars`, `dependencies`), the list is replaced if the override manifest has a non-empty list.

Certain properties have special merging logic:
* `env` maps are merged by adding override entries to the manifest, replacing existing values if they exist in the override.
* `kong` can not be overridden (i.e., it can not be declared in multiple sources for a manifest at the same time). However, it can occur in any source
  * E.g., if it's declared in `staging.yml`, it can't be declared in `staging-uk.yml`, but it can be in `dev-uk.yml`.

### Example
Given the following configuration

```yaml
# shipcat.conf
regions:
- name: staging-uk
  env:
    LOG_LEVEL: info

# service/my-service/shipcat.yml
dependencies:
- name: foo-service
env:
  FEATURE_A: disabled

# service/my-service/dev-uk.yml
dependencies: []
env:
  FEATURE_A: enabled
kong:
  uris: /my-service

# service/my-service/staging.yml
version: 1.0.0
dependencies:
- name: bar-service
env:
  FEATURE_B: enabled
kong:
  uris: /my-service/v1

# service/my-service/staging-uk.yml
version: 1.0.5
env:
  LOG_LEVEL: warn
  FEATURE_B: disabled
```

results in the following:

```yaml
# region: dev-uk
# version is unset
dependencies:
- name: foo-service # from shipcat.yml, because dev-uk.yml value is empty
env:
  # LOG_LEVEL is unset
  FEATURE_A: enabled # from dev-uk.yml, overridding shipcat.yml
  # FEATURE_B is unset
kong: # from dev-uk.yml
  uris: /my-service

# region: staging-uk
version: 1.0.5 # from staging-uk.yml
dependencies:
- name: bar-service # from staging.yml
env:
  LOG_LEVEL: warn # from staging-uk.yml, overriding region in shipcat.conf
  FEATURE_A: disabled # from shipcat.yml
  FEATURE_B: disabled # from staging-uk.yml, overridding staging.yml
kong: # from staging.yml
  uris: /my-service/v1

# region: staging-us
version: 1.0.0 # from staging.yml
dependencies:
- name: bar-service # from staging.yml
env:
  LOG_LEVEL: info # from region in shipcat.conf
  FEATURE_A: disabled # from shipcat.yml
  FEATURE_B: disabled # from staging.yml
kong: # from staging.yml
  uris: /my-service/v1
```
