## Templates

Because of helm limitations it's hard to attach arbitrary files as config mapped files into a service. We solve this by pre-templating files with `shipcat` then passing them to helm as large strings.

## Usage

If you have the following in your manifest:

```yaml
configs:
  mount: /config/
  files:
  - name: logging.conf.j2
    dest: logging.conf
  - name: newrelic-python.ini.j2
    dest: newrelic.ini
```

then this will be interpolated by `shipcat values` by looking for files called `logging.conf.j2` and `newrelic-python.ini.j2` in two search folders:

- `services/myservice` (service folder)
- `templates` (global templates folder)

Files from the global `templates` folder are only used if there's no file with the same name in your service folder.

When applied to the helm chart, `shipcat` will template these files and return a struct like:

```yaml
configs:
  name: myservice-config
  mount: /config/
  files:
    - name: logging.conf.j2
      dest: logging.conf
      value: "# Basic logging config for Python microservices\n[loggers]\nkeys=root,babylon\n\n[handlers]\nkeys=consoleHandler\n\n[formatters]\nkeys=simpleFormatter,logStashFormatter\n\n[logger_root]\nlevel=DEBUG\nhandlers=consoleHandler\n\n[logger_babylon]\nlevel=DEBUG\nhandlers=consoleHandler\nqualname=babylon\n\n[handler_consoleHandler]\nclass=babylon.logging.StreamHandler\nlevel=DEBUG\nformatter=logStashFormatter\nargs=(sys.stdout,)\n\n[formatter_simpleFormatter]\nformat=%(asctime)s [%(levelname)s] %(message)s\ndatefmt=%Y-%m-%dT%H:%M:%S%z\n\n[formatter_logStashFormatter]\nclass=babylon.logging.LogstashFormatter"
    - name: newrelic-python.ini.j2
      dest: newrelic.ini
      value: "[newrelic]\nlicense_key = ZZZZZZZZZZZZZZZZ\napp_name = myservice\n\n[newrelic:development]\nmonitor_mode = true\n\n"
```

basically, a huge inlined string that gets put into a kube `ConfigMap` and eventually mounted under `/config/` inside the container.

## Format
Templates are rendered using [tera](https://tera.netlify.com/) which uses basic Jinja2 syntax.

Here's an example of a configuration file a service using S3, which gets region names, service names, and environment variables:

```yaml
jobmanager.rpc.address: localhost
jobmanager.rpc.port: 6123
jobmanager.heap.size: 1024m
taskmanager.heap.size: 1024m
taskmanager.numberOfTaskSlots: 1
parallelism.default: 1
state.backend: filesystem
state.checkpoints.dir: s3://{{ region }}-{{service}}/{{ service }}/checkpoints
state.savepoints.dir: s3://{{ region }}-{{service}}/{{ service }}/savepoints
rest.port: 8081
s3.access-key: {{ env.S3_ACCESS_KEY }}
s3.pin-client-to-current-region: true
```

## Available Context
The current data available in temmplates are:

- `service` - service `name` property
- `region` - region we are deploying to
- `env` - completed environment variables
- `base_urls` - base urls for kong and others
- `kong` - kong configuration and consumer info
- `kafka` - kafka cluster setup

## Templating environment variables
Due to popular demands of removing duplication between services, we can use light templating of environment variables.

These are templated before `configs` to avoid uncompleted template strings ending up in configmaps.

Examples:

```yaml
env:
  INTERNAL_AUTH_URL: "{{ base_urls.services }}/auth/internal"
  CI_PLATFORM_URL: "{{ base_urls.services }}/ci-platform//"
  INTERNAL_AUTH_CLIENTID: "{{ kong.consumers['my-service'].oauth_client_id }}"
  INTERNAL_AUTH_CLIENTSECRET: "{{ kong.consumers['my-service'].oauth_client_secret | as_secret }}"
```

The first two are filled in from the `base_urls` map in `shipcat.conf` for the region, while the other ones are fetched from `kong.consumers` for the service after first fetching the values from vault.

The `| as_secret` template function lets shipcat know that this evar should be mounted as a kubernetes secret and not visible in plaintext on a kube dashboard.
