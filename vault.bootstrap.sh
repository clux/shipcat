#!/bin/bash
set -ex

vault policy-write ci-shipcat .vault-policy
# create a 3 months valid token
vault token-create -policy="ci-shipcat" --ttl="129600m" -display-name="shipcat-ci"
