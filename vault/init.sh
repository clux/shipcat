#!/usr/bin/env sh

set -ex

#vault policy write shipcat shipcat-policy.hcl
policy_body=$(jq -R -s '{ rules: . }' shipcat-policy.hcl)
curl --fail -Ss \
  -X PUT \
  -d "$policy_body" \
  -H "Content-Type: application/json" \
  -H "X-Vault-Token: $VAULT_TOKEN" \
  "$VAULT_ADDR/v1/sys/policy/shipcat"

#vault token create --policy "shipcat" --id="identity-secret"
token_body='{
  "id": "identity-secret",
  "policies": ["shipcat"]
}'
curl --fail -Ss \
  -X POST \
  -d "$token_body" \
  -H "Content-Type: application/json" \
  -H "X-Vault-Token: $VAULT_TOKEN" \
  "$VAULT_ADDR/v1/auth/token/create"

# vault write secret/dev-uk/test-shipcat/FAKE_NUMBER -2
curl --fail -Ss \
  -X POST \
  -d '{"value":-2}' \
  -H "Content-Type: application/json" \
  -H "X-Vault-Token: $VAULT_TOKEN" \
  "$VAULT_ADDR/v1/secret/dev-uk/test-shipcat/FAKE_NUMBER"

# vault write secret/dev-uk/test-shipcat/FAKE_SECRET "hello"
curl --fail -Ss \
  -X POST \
  -d '{"value":"hello"}' \
  -H "Content-Type: application/json" \
  -H "X-Vault-Token: $VAULT_TOKEN" \
  "$VAULT_ADDR/v1/secret/dev-uk/test-shipcat/FAKE_SECRET"

# vault write secret/dev-uk/test-shipcat/fake-file "aGVsbG8gd29ybGQgYmFzZTY0Cg=="
curl --fail -Ss \
  -X POST \
  -d '{"value":"aGVsbG8gd29ybGQgYmFzZTY0Cg=="}' \
  -H "Content-Type: application/json" \
  -H "X-Vault-Token: $VAULT_TOKEN" \
  "$VAULT_ADDR/v1/secret/dev-uk/test-shipcat/fake-file"
