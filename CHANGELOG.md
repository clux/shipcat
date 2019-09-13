0.127.0 / 2019-09-13
==================
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
==================
  * `reconciliationMode` default is now `CrdStatus` rather than `CrdVersioned`
  * Kubernetes default server requirement now 1.12
