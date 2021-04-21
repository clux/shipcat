### Shipping shipcat

1. As soon as you're confident with your code changes:
    * update from `master`
    * take note of current semver in any `Cargo.toml` files ([this one](shipcat_definitions/Cargo.toml#L14) for example)
    * run `scripts/bump_version.sh current_sem_ver bumped_sem_ver`

2. As soon as your PR is merged and [all CircleCI jobs](https://app.circleci.com/pipelines/github/babylonhealth/shipcat) complete successfully
    * [mint a new release](https://github.com/babylonhealth/shipcat/releases/new) with id (git tag) and title equal to bumped semver
    * wait till some [extra CircleCI jobs](https://app.circleci.com/pipelines/github/babylonhealth/shipcat) complete
    * ensure that the below extra assets are attached to github release: 
        ````
        shipcat.sha256
        shipcat.x86_64-apple-darwin.tar.gz
        shipcat.x86_64-unknown-linux-musl.tar.gz
        ````

3. Update private babylonhealth brew tap
    * Check out / update [homebrew-babylon](https://github.com/babylonhealth/homebrew-babylon) locally
    * init (if `.venv` missing locally) and activate python3 virtualenv (the below issued at `homebrew-babylon` checkout):
        ```
        python3 -m venv .venv
        source .venv/bin/activate
        pip3 install -r requirements.txt
        ```
    * checkout a new branch
    * run `./update.py`
    * push/review/merge the usual way

4. Update version pins in [manifests/shipcat.conf](https://github.com/babylonhealth/manifests/blob/6a9d2a2fde8cd81e67cea31512e45d757d5caf5e/shipcat.conf#L4090)
    * if your changes are hitting all environments at once (i.e. you add extra fields to `shipcat.conf` itself), version *has* to be promoted globally in one go
    * otherwise, one might aim at gradual version promotion, testing out actual manifest schema changes in dev/staging regions first