# Default deny
path "secret/*" {
  policy = "deny"
}

# Allow full access to dev-uk
path "secret/*" {
  capabilities = ["create", "update"]
}
