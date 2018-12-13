#! /usr/bin/env bash

#
# Bumps the version number from <current> to <next> on all libraries.
# Use from shipcat root directory (git root)
#

if [ -z "${1}" ] || [ -z "${2}" ]; then
  echo "Usage: $0 <current> <next>"
  echo "Example: $0 0.77.0 0.78.0"
  exit 1
fi

if ! git grep -c "${1}" > /dev/null; then
  echo "The version '${1}' doesn't appear to be correct."
  echo "Exiting."
  exit 1
fi

function do_replace() {
  find . -maxdepth 2 -mindepth 2 -name "*.toml" -print0 | xargs -0 sed -i "s/${1}/${2}/g"
}

do_replace "${1}" "${2}"

echo "Versions replaced. Please ensure the following diff is sane:"
git diff
