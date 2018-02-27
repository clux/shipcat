path "sys/*" {
  policy = "deny"
}

# Default deny
path "secret/*" {
  policy = "deny"
}

# Only allow read access to the test variables
path "secret/dev-uk/test-shipcat/*" {
  policy = "read"
}
