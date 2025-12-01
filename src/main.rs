mod parser;

use neo4rs::{BoltList, BoltMap, BoltType, Graph, query};
use parser::parse_path_file;
use std::fs::read_dir;

#[tokio::main]
async fn main() {
    let mut node_files = Vec::new();
    let paths = read_dir("./paths").unwrap();
    for path in paths {
        let path = path.unwrap().path();
        node_files.push(parse_path_file(&path).unwrap());
    }

    let nodes_count = node_files
        .iter()
        .map(|node| node.header.number_of_nodes)
        .sum::<u32>();

    let uri = "94.130.19.15:7687";
    let user = "neo4j";
    let pass = "Ef92Qt0LWWPUJF9obR7cwOnRXCL1qIbq";

    let graph = Graph::new(uri, user, pass).await.unwrap();

    graph
        .run(query("MATCH (n:PathNode) DELETE n"))
        .await
        .unwrap();

    let mut all_nodes = Vec::new();
    for node_file in node_files {
        for node in node_file.nodes {
            let mut node_data = BoltMap::new();
            node_data.put("x".into(), BoltType::from(node.x as f64));
            node_data.put("y".into(), BoltType::from(node.y as f64));
            node_data.put("z".into(), BoltType::from(node.z as f64));
            node_data.put("link_id".into(), BoltType::from(node.link_id as i64));
            node_data.put("area_id".into(), BoltType::from(node.area_id as i64));
            node_data.put("node_id".into(), BoltType::from(node.node_id as i64));
            node_data.put("path_width".into(), BoltType::from(node.path_width as i64));
            node_data.put("flood_fill".into(), BoltType::from(node.flood_fill as i64));
            node_data.put("link_count".into(), BoltType::from(node.link_count as i64));
            node_data.put(
                "traffic_level".into(),
                BoltType::from(node.traffic_level as i64),
            );
            node_data.put(
                "emergency_vehicle_only".into(),
                BoltType::from(node.emergency_vehicle_only),
            );
            node_data.put("is_highway".into(), BoltType::from(node.is_highway));
            node_data.put("parking".into(), BoltType::from(node.parking));
            all_nodes.push(node_data);
        }
    }

    // Batch create nodes in chunks of 1000
    const BATCH_SIZE: usize = 1000;
    for chunk in all_nodes.chunks(BATCH_SIZE) {
        let mut nodes_list = BoltList::new();
        for node_map in chunk {
            nodes_list.push(BoltType::Map(node_map.clone()));
        }
        graph
            .run(query("UNWIND $nodes AS nodeData CREATE (n:PathNode {x: nodeData.x, y: nodeData.y, z: nodeData.z, link_id: nodeData.link_id, area_id: nodeData.area_id, node_id: nodeData.node_id, path_width: nodeData.path_width, flood_fill: nodeData.flood_fill, link_count: nodeData.link_count, traffic_level: nodeData.traffic_level, emergency_vehicle_only: nodeData.emergency_vehicle_only, is_highway: nodeData.is_highway, parking: nodeData.parking})")
                .param("nodes", BoltType::List(nodes_list)))
            .await
            .unwrap();
    }

    println!("Nodes count {}", nodes_count);
}
