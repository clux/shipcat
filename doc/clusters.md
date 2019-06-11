# Cluster configuration
The `shipcat.conf` file at the root of your manifests repo defines your kube clusters, context aliases, shipcat regions and how they interplay.

Sample config:

```yaml
clusters:
  platformus-green:
    api: https://api.platformus-green.kube.babylontech.co.uk
    regions:
    - platform-us
  kops-uk:
    api: https://api.kube-uk.dev.babylontech.co.uk
    regions:
    - dev-uk
    - staging-uk
contextAliases:
  platformus-green: platform-us
regions:
- name: platform-us:
  environment: platform
  namespace: apps
  versioningScheme: Semver
  vault: ...
  kong: ...
- name: dev-uk:
  namespace: dev
  environment: dev
  versioningScheme: GitShaOrSemver
  vault: ...
  kong: ...
- name: staging-uk:
  namespace: staging
  environment: staging
  versioningScheme: Semver
  vault: ...
  kong: ...
```

## cluster <-> region relations
- one region can have multiple clusters (`platform-us` -> `platformus-green` + `platformus-blue`)
- one cluster can have multiple regions (`kops-uk` covers to `dev-uk` and `staging-uk`)

## cluster aliases
This is a raw map of kube context (`kubectl config current-context`) into the shipcat `region` as specified by a key name in `regions`.

This setup allows us to quickly create a failover cluster without changing any of the manifests. We simply change add the new cluster and make a job to reconcile in this new `platformus-blue` cluster.

## shipcat region
Region is a lightweight abstraction on top of a kube context.

A kube context is a tuple: `(ContextName, ClusterApiUrl, Namespace, AuthInfo)`

A shipcat region is an abstract kube region with the possibility of getting the cluster data given a `ContextName`. This definition should also work without an updated `~/.kube/config` for most cases.

## cluster <-> context relations
- one cluster can have multiple contexts
- one context is bound to a single cluster

This is because a kube context is a triple: , and a shipcat region is a light abstraction on top of that.

## Reconciliation caveats
If a job is running reconciliation on a region backed by multiple clusters; you need to specify which of the clusters you are specifying.
