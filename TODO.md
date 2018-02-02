# TODO

## Plan
### immediate
- kubectl apply with output of shipcat generate tested (yaml format)
- create manifests for everything (and fix up issues)
- make new jenkins job from cathulk repo without k8s plugin
- generalize jenkins job to work with both dev AND global envs

### post dev environment
- generalize bdd stuff in manifests
- helm integration?
- shipcat to work without hardcoding it to manifests repo?

## Investigation
### kube deploy currently via ai-deploy
- `kube ctl set image deployment/{name} {name}={repo}/{srvc}:{version} -n dev`
- await `kubectl rollout status deployment/{name} -n dev` (retries 20 delay 5)
- slack announce update
- run bdds (docker image chatbot-tests:{bdd-tag} with a few evars and a junit mount)
- check bdd res (junit mount inspection)
- slack notify bdd results
if failures:
 - `kubectl rollout undo deployment/{name} -n dev` (lolcal transactions?)

## Deployment strategies
- explicit images released in different versions of service repos
- configs updated in service-configs repo

Figure out if we CAN deploy a new version
- `shipcat diff -n dev {service}`
should return differences from deployed service

- `shipcat update -n dev {service}`
should update manifest

- `shipcat ship -n dev {service}`

## Pipeline ideas
### 1. Current flow
- develop branch merged
- circle builds docker image
- circle triggers ai-deploy-dev
- ai-deploy-dev runs ansible pipeline (bdd tests) and updates static kube files

#### Problems
- no config solutions (static kube files in ops-kubernetes)
- kube files not used as part of pipeline
- ansible bdd stuff is hacky

#### Good
- queues deploys on dev cluster via jenkins resources
- bdd guarantee on dev-environment (MOSTLY)

### 2. Shipcat aggresesive flow (bad idea)
- develop branch merged
- circle builds docker image
- circle calls `shipcat ship -n dev` deploys to cube
- `shipcat test -n dev` runs bdds
- `shipcat rollback` on failure

#### Problems
- no locks on kube environment (everyone can deploy at once)
- hard to edit pipeline (in circle config everywhere)
- need to put a lot of junit and baby specific testing logic in shipcat
- need to fetch service-configs repo as part of shipcat deploy
- no separation between config changes and image changes for dev

#### Good
- no jenkins
- service configs accounted for

### 3. Shipcat island flow
- develop branch merge
- circle builds docker image
- circle triggers `deploy-dev`
- deploy-dev forked from `ai-deploy-dev` - runs in `service-configs`
- new (mirror) deploy procedure forked from ai-deploy (better)

#### Good
- maintain jenkins lock on dev env for now
- avoids kube plugin in jenkins with fork
- accounts for kubefiles
- baby specific deploy/testing logic in service-configs repo
- same BDD guarantee on dev environment

#### Bad
- tied to a jenkins deploy job
- tied to the `dev` resource on jenkins
- complex recreation

### 4. Shipcat future flow
- shipcat can deploy directly
- shipcat is configurable and can run anywhere
- config file + musl build

#### Good
- dev self-service solution
- generalized tool

#### Bad
- needs propre redesign of test pipeline (no BDD lock)
- FFA deployment style without a BDD lock
- tool would expect a repo of manifests
- need to track state of config repo
