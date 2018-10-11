## Vault secrets
Secrets for services are stored in a regional [vault](https://www.vaultproject.io/), which is configured in `shipcat.conf`

### Environment variables
The typical use case is classifying secret containing evars with a `IN_VAULT` specifier:

```yaml
env:
  MY_SECRET: IN_VAULT
  NON_SECRET_EVAR: plaintext-foo
```

This will be placed in the output of `shipcat values -s`, by doing a vault lookup against `{vaultroot}/myservice/MY_SECRET`.

## Secret Files
For larger secrets, you can use `secretFiles`:

```yaml
secretFiles:
-  myservice-ssl-keystore: IN_VAULT
```

This will do a vault lookup against `{vaultroot}/myservice-myservice-ssl-keystore` and decode a base64 encoded secret.


## Vault Root
Vault root can be specified in `shipcat.conf` for a region:

```yaml
regions:
  platform-us:
    vault:
      url: https://vault.myhost.com:8200
      folder: apps
```

which will cause vault lookups with `https://vault.myhost.com:8200/v1/secret/apps` as `{vaultroot}` in the examples above.
