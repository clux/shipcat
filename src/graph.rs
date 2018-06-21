use std::io::{self, Write};

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::dot;
use serde_yaml;

use super::structs::{Dependency, DependencyProtocol};
use std::fmt::{self, Debug};
use super::{Manifest, Result, Config};

/// The node type in `CatGraph` representing a `Manifest`
#[derive(Serialize, Deserialize, Clone)]
pub struct ManifestNode {
    pub name: String,
    //pub image: String,
}
impl ManifestNode {
    fn new(mf: &Manifest) -> Self {
        ManifestNode {
            name: mf.name.clone(),
            // image would be nice, but requires env override atm - should be global
            //image: format!("{}", mf.image.clone().unwrap()),
        }
    }
}
// Debug is used for the `dot` interface - nice to have a minimal output for that
impl Debug for ManifestNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// The edge type in `CatGraph` representing a `Dependency`
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DepEdge {
    pub api: String,
    pub contract: Option<String>,
    pub protocol: DependencyProtocol,
    pub intent: Option<String>,
}
impl DepEdge {
    fn new(dep: &Dependency) -> Self {
        DepEdge {
            api: dep.api.clone().unwrap(),
            contract: dep.contract.clone(),
            protocol: dep.protocol.clone(),
            intent: dep.intent.clone(),
        }
    }
}


/// Graph of simplified manifests with dependencies as edges
///
/// This is fully serializable because it is created with `petgraph` using the serde
/// featurset. We use that to serialize the graph as yaml.
/// We can also convert this to `graphviz` format via some of the `petgraph` helpers.
pub type CatGraph = DiGraph<ManifestNode, DepEdge>;

fn nodeidx_from_name(name: &str, graph: &CatGraph) -> Option<NodeIndex> {
    for id in graph.node_indices() {
        if let Some(n) = graph.node_weight(id) {
            if n.name == name {
                return Some(id);
            }
        }
    }
    None
}

fn recurse_manifest(idx: NodeIndex, mf: &Manifest, conf: &Config, graph: &mut CatGraph, reg: &str) -> Result<()> {
    for dep in &mf.dependencies {
        debug!("Recursing into {}", dep.name);
        // skip if node exists to avoid infinite loop
        if let Some(depidx) = nodeidx_from_name(&dep.name, &graph) {
            trace!("Linking root node {} to existing node {}", mf.name, dep.name);
            graph.update_edge(idx, depidx, DepEdge::new(&dep));
            debug!("Stopping recursing - node {} covered", dep.name);
            continue;
        }

        let depmf = Manifest::stubbed(&dep.name, conf, reg)?;

        let depnode = ManifestNode::new(&depmf);
        let depidx = graph.add_node(depnode);

        graph.update_edge(idx, depidx, DepEdge::new(&dep));
        recurse_manifest(depidx, &depmf, conf, graph, reg)?;
    }

    Ok(())
}

/// Generate dependency graph from an entry point via recursion
pub fn generate(service: &str, conf: &Config, dot: bool, reg: &str) -> Result<CatGraph> {
    let base = Manifest::stubbed(service, conf, &reg)?;


    let mut graph : CatGraph = DiGraph::<_, _>::new();
    let node = ManifestNode::new(&base);
    let baseidx = graph.add_node(node);

    recurse_manifest(baseidx, &base, conf, &mut graph, reg)?;

    let out = if dot {
        format!("{:?}", dot::Dot::with_config(&graph, &[dot::Config::EdgeNoLabel]))
    }
    else {
        format!("{}", serde_yaml::to_string(&graph)?)
    };
    let _ = io::stdout().write(&out.as_bytes());
    Ok(graph)
}

/// Generate dependency graph from services directory
///
/// This is a better solution even if we wanted the result centered around
/// one or more services as we could also show grahps reaching into the ecosystem.
///
/// But it would require: TODO: optionally filter edges around node(s)
pub fn full(dot: bool, conf: &Config, reg: &str) -> Result<CatGraph> {
    let mut graph : CatGraph = DiGraph::<_, _>::new();
    let services = Manifest::available()?;
    for svc in services {
        debug!("Scanning service {:?}", svc);

        let mf = Manifest::stubbed(&svc, conf, reg)?;
        let node = ManifestNode::new(&mf);
        let idx = graph.add_node(node);

        for dep in &mf.dependencies {
            let subidx = if let Some(id) = nodeidx_from_name(&dep.name, &graph) {
                trace!("Found dependency with existing node: {}", dep.name);
                id
            } else {
                trace!("Found dependency new in graph: {}", dep.name);
                let depmf = Manifest::stubbed(&dep.name, conf, &reg)?;
                let depnode = ManifestNode::new(&depmf);
                let depidx = graph.add_node(depnode);
                depidx
            };
            graph.update_edge(idx, subidx, DepEdge::new(&dep));
        }
    }

    let out = if dot {
        format!("{:?}", dot::Dot::with_config(&graph, &[dot::Config::EdgeNoLabel]))
    }
    else {
        format!("{}", serde_yaml::to_string(&graph)?)
    };
    let _ = io::stdout().write(&out.as_bytes());
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use serde_yaml;
    use super::{generate, nodeidx_from_name};
    use tests::setup;
    use super::Config;

    #[test]
    fn graph_generate() {
        setup();
        let conf = Config::read().unwrap();
        let graph = generate("fake-ask", &conf, true, "dev-uk").unwrap();
        assert!(graph.edge_count() > 0);
        print!("got struct: \n{:?}\n", serde_yaml::to_string(&graph));
        let askidx = nodeidx_from_name("fake-ask", &graph).unwrap();
        debug!("ask idx {:?}", askidx);
        let strgidx = nodeidx_from_name("fake-storage", &graph).unwrap();
        debug!("strg idx {:?}", strgidx);
        let edgeidx = graph.find_edge(askidx, strgidx).unwrap();
        debug!("edge idx {:?}", edgeidx);
        let edge = graph.edge_weight(edgeidx).unwrap();
        debug!("edge: {:?}", edge);
        assert_eq!(edge.intent, Some("testing graph module".into()));
    }
}
