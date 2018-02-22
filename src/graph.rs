use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::dot::{Dot, Config};
use serde_yaml;

use super::{Manifest, Dependency, Result};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub name: String,
}
impl Node {
    fn new(mf: &Manifest) -> Self {
        Node {
            name: mf.name.clone()
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


fn contains_node(name: &str, graph: &CatGraph) -> Option<Node> {
    for rn in graph.raw_nodes() {
        if rn.weight.name == name {
            return Some(rn.weight.clone());
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
        if let Some(n) = contains_node(&dep.name, &graph) {
            // TODO: avoid duplicate add - need to get the index of the node
            let depidx = graph.add_node(n);
            graph.update_edge(idx, depidx, Edge::new(&dep));

            debug!("Ignoring covered node {}", dep.name);
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

pub fn generate(service: &str, dot: bool) -> Result<()> {
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

    Ok(())
}
