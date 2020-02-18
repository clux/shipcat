mod common;
use crate::common::setup;
use shipcat::graph::{generate, nodeidx_from_name};
use shipcat_definitions::{Config, ConfigState};

#[tokio::test]
async fn graph_generate() {
    setup();
    let (conf, reg) = Config::new(ConfigState::Base, "dev-uk").await.unwrap();
    let graph = generate("fake-ask", &conf, &reg, true).await.unwrap();
    assert!(graph.edge_count() > 0);
    print!("got struct: \n{:?}\n", serde_yaml::to_string(&graph));
    let askidx = nodeidx_from_name("fake-ask", &graph).unwrap();
    println!("ask idx {:?}", askidx);
    let strgidx = nodeidx_from_name("fake-storage", &graph).unwrap();
    println!("strg idx {:?}", strgidx);
    let edgeidx = graph.find_edge(askidx, strgidx).unwrap();
    println!("edge idx {:?}", edgeidx);
    let edge = graph.edge_weight(edgeidx).unwrap();
    println!("edge: {:?}", edge);
    assert_eq!(edge.intent, Some("testing graph module".into()));
}
