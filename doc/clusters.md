# Cluster configuration
The `shipcat.conf` file at the root of your manifests repo defines your kube clusters, context aliases, shipcat regions and how they interplay.

Sample config:

```yaml
clusters:
  platformus-green:
    api: https://api.platformus-green.kube.domain.invalid
    teleport: https://FUFUFUFU.bl2.eu-west-2.eks.amazonaws.com
    regions:
    - platform-us
  kops-uk:
    api: https://api.kube-uk.dev.domain.invalid
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
  cluster: platformus-green
  vault: ...
  kong: ...
- name: dev-uk:
  namespace: dev
  environment: dev
  cluster: kops-uk
  versioningScheme: GitShaOrSemver
  vault: ...
  kong: ...
- name: staging-uk:
  namespace: staging
  environment: staging
  cluster: kops-uk
  versioningScheme: Semver
  vault: ...
  kong: ...
```

## cluster <-> region relations
- one region can be covered by multiple clusters (`platform-us` -> `platformus-green` + `platformus-blue`)
- one cluster can serve multiple regions (`kops-uk` covers to `dev-uk` and `staging-uk`)

The `cluster` key on the region disambiguates the cluster choice when reconciling a region.

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
