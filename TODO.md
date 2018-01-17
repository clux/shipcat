# TODO

## Plan
### immediate
- finish basic templating of files
- service-configs repo (we need to look there, track/update it)
- linux build (ez)
- used on travis (moderate (access, fetch maybe in docker)
- kubectl apply with output of shipcat generate tested (yaml format)
- commit it to ai-deploy and test a few runs
- create manifests for everything (and fix up issues)
- make jenkins job work without k8s plugin (for next step)
- generalize jenkins job to work with both dev AND global envs

### post dev environment
- generalize bdd stuff in manifests
- untangle weird conditional ansible in [ai-deploy/better](https://github.com/Babylonpartners/ai-deploy/blob/a5f98480c37181e12be9566e314433db733d3d25/deployment/better/inventories/jenkins-dev.yml#L11)
- osx build
- helm integration

## Investigation
### kube deploy currently via ai-deploy
- `kube ctl set image deployment/{name} {name}={repo}/{srvc}:{version} -n dev`
- await `kubectl rollout status deployment/{name} -n dev` (retries 20 delay 5)
- slack announce update
- run bdds (docker image chatbot-tests:{bdd-tag} with a few evars and a junit mount)
- check bdd res (junit mount inspection)
- slack notify bdd results
if failures:
 - `kubectl rollout undo deployment/{name} -n dev`
