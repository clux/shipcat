0.147.0 / 2020-02-XX
====================
  * Convert shipcat to use async/await internally
    - tokio "fs" feature in shipcat_filebacked
    - tokio "process" feature in shipcat_cli
    - kube 0.25.0 for k8s api calls
    * Remove rayon + threadpool dependencies
    - cluster + validate/verify commands now just use tokio "stream" feature
  * Upgrade reqwest to 0.10 everywhere in shipcat cli (-15 deps)
  * Upgrade url to 2.X and remove url_serde dependency
  * Added optional `metadata.context` field

0.146.0 / 2020-02-10
====================
  * `Manifest.eventStreams` syntax added for Kafka topic and Zookeepr ACL creation - #381

0.145.0 / 2020-02-07
====================
  * service deletion now gets a slack warning
  * `shipcat restart` now also restarts worker deployments
  * `shipcatmanifest` crd status object now contains a `lastSuccessfulRolloutVersion`

0.144.1 / 2020-02-05
====================
  * shipcat diff `-k` or `--current` removed (now default)
  * new `--mock` introduced in `shipcat diff` to get old behaviour
  * `workload` new property on `Manifest` (to support Deployment/Statefulset switches)

0.143.0 / 2020-01-31
====================
  * Tiller support removed
  * `shipcat template -c MYSVC` properly validates template labels
  * `shipcat cluster check` is a region-wide variant of `shipcat template -c`

0.142.0 / 2020-01-07
====================
  * ReconciliationMode::CrdOwned new default
  * Tiller support fully deprecated and will be removed soon

0.134.1 / 2019-10-23
====================
  * Speculative `shipcat self-upgrade`
  * Can be invoked automatically with `SHIPCAT_AUTOUPGRADE=1` evar.
  * Can avoid rate limiting with `SHIPCAT_AUTOUPGRADE_TOKEN` to github token with `repo:read` scope.

0.133.0 / 2019-10-18
====================
  * introductory syntax for `newRelic` and `sentry`
  * kong: supporting multiple kong apis per service
  * `jobs` syntax removed
  * version warning now always present, but ignored unless --strict-version-check set
  * `shipcat restart {service}` subcommand added
  * `shipcat diff` improvements
  * `shipcat port-forward` improvements
  * better error messages for missing executable dependencies

0.132.0 / 2019-10-08
====================
  * `.rbac` key in manifests now support arbitrary verbs and nouns
  * better error messages for `teams.yml` errors
  * readiness / liveness probes support timeout properly

0.131.0 / 2019-10-07
====================
  * kong: json cookie header deprecated parameters removed

0.130.0 / 2019-10-01
====================
  * `shipcat` accepts team owners in `teams.yml`
  * `shipcat.conf` will phase out `teams` vector from this release

0.129.0 / 2019-09-26
====================
  * `diff` can can compare regions
  * `apply` now resilient against manifest schema changes (bug)

0.127.1 / 2019-09-16
====================
  * Diff now correctly detects kubectl diff output and filters out generation

0.127.0 / 2019-09-13
====================
  * First release to support upgrades withut tiller.
  * `ReconciliationMode::CrdOwned` and can be set per-region in `shipcat.conf`.
  * Tested properly in examples/ directory
  * Charts must provide the following properties in metadata now:

```yaml
metadata:
  labels:
    app.kubernetes.io/name: {{ .Values.name }}
    app.kubernetes.io/version: {{ .Values.version }}
    app.kubernetes.io/managed-by: shipcat
  ownerReferences:
  - apiVersion: babylontech.co.uk/v1
    kind: ShipcatManifest
    controller: false
    name: {{ .Values.name }}
    uid: {{ .Values.uid }}
```

 * `shipcat diff` rewritten, uses kubectl by default
   - `--minify` or `-m` added (to minify the diff)
   - `--obfuscate` added (to hide secrets)
   - `--current` or `-k` added (to use actual uids/versions from kube)
 * `shipcat template` improved
   - `--check` or `-c` added (verifies chart assumptions)
   - `--current` or `-k` added (to use actual uids/versions from kube)

0.126.0 / 2019-09-03
====================
  * `reconciliationMode` default is now `CrdStatus` rather than `CrdVersioned`
  * Kubernetes default server requirement now 1.12
