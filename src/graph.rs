use std::path::Path;
use std::io::{self, Write};

use walkdir::WalkDir;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::dot::{Dot, Config};
use serde_yaml;

use super::{Manifest, Dependency, Result};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub name: String,
    //pub image: String,
}
impl Node {
    fn new(mf: &Manifest) -> Self {
        Node {
            name: mf.name.clone(),
            // image would be nice, but requires env override atm - should be global
            //image: format!("{}", mf.image.clone().unwrap()),
        }
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Edge {
    pub api: String,
    pub contract: Option<String>
}
impl Edge {
    fn new(dep: &Dependency) -> Self {
        Edge {
            api: dep.api.clone().unwrap(),
            contract: dep.contract.clone(),
        }
    }
}


type CatGraph = DiGraph<Node, Edge>;

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

fn recurse_manifest(idx: NodeIndex, mf: &Manifest, graph: &mut CatGraph) -> Result<()> {
    for dep in &mf.dependencies {
        debug!("Recursing into {}", dep.name);
        if dep.name == "core-ruby" || dep.name == "php-backend-monolith" {
            debug!("Ignoring dependencies for non-shipcat monolith");
            continue;
        }

        // skip if node exists to avoid infinite loop
        if let Some(depidx) = nodeidx_from_name(&dep.name, &graph) {
            trace!("Linking root node {} to existing node {}", mf.name, dep.name);
            graph.update_edge(idx, depidx, Edge::new(&dep));
            debug!("Stopping recursing - node {} covered", dep.name);
            continue;
        }

        let depmf = Manifest::basic(&dep.name)?;

        let depnode = Node::new(&depmf);
        let depidx = graph.add_node(depnode);

        graph.update_edge(idx, depidx, Edge::new(&dep));
        recurse_manifest(depidx, &depmf, graph)?;
    }

    Ok(())
}

/// Generate dependency graph from an entry point
pub fn generate(service: &str, dot: bool) -> Result<CatGraph> {
    let base = Manifest::basic(service)?;


    let mut graph : CatGraph = DiGraph::<_, _>::new();
    let node = Node::new(&base);
    let baseidx = graph.add_node(node);

    recurse_manifest(baseidx, &base, &mut graph)?;

    if dot {
        println!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));
    }
    else {
        println!("{}", serde_yaml::to_string(&graph)?);
    }
    io::stdout().flush()?; // allow piping stdout elsewhere

    Ok(graph)
}

/// Generate dependency graph from services directory
///
/// TODO: optionally filter around a node
pub fn full(dot: bool) -> Result<CatGraph> {
    let svcsdir = Path::new(".").join("services");
    let svcs = WalkDir::new(&svcsdir)
        .min_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir());

    let mut graph : CatGraph = DiGraph::<_, _>::new();
    for e in svcs {
        let mut cmps = e.path().components();
        cmps.next(); // .
        cmps.next(); // services
        let svccomp = cmps.next().unwrap();
        let svcname = svccomp.as_os_str().to_str().unwrap();

        debug!("Scanning service {:?}", svcname);

        let mf = Manifest::basic(svcname)?;
        let node = Node::new(&mf);
        let idx = graph.add_node(node);

        for dep in &mf.dependencies {
            if dep.name == "core-ruby" || dep.name == "php-backend-monolith" {
                debug!("Ignoring dependencies for non-shipcat monolith");
                continue;
            }
            let subidx = if let Some(id) = nodeidx_from_name(&dep.name, &graph) {
                trace!("Found dependency with existing node: {}", dep.name);
                id
            } else {
                trace!("Found dependency new in graph: {}", dep.name);
                let depmf = Manifest::basic(&dep.name)?;
                let depnode = Node::new(&depmf);
                let depidx = graph.add_node(depnode);
                depidx
            };
            graph.update_edge(idx, subidx, Edge::new(&dep));
        }
    }

    if dot {
        println!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));
    }
    else {
        println!("{}", serde_yaml::to_string(&graph)?);
    }
    io::stdout().flush()?; // allow piping stdout elsewhere

    Ok(graph)
}
