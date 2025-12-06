use gtasa_extractor::parser::parse_path_file;
use neo4rs::{BoltList, BoltMap, BoltType, Graph, query};
use std::fs::read_dir;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./paths")]
    path: String,

    #[arg(short, long)]
    uri: String,

    #[arg(short, long)]
    username: String,

    #[arg(short, long)]
    password: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut node_files = Vec::new();
    let paths = read_dir(args.path).unwrap();
    for path in paths {
        let path = path.unwrap().path();
        node_files.push(parse_path_file(&path).unwrap());
    }

    let nodes_count = node_files
        .iter()
        .map(|node| node.header.number_of_nodes)
        .sum::<u32>();

    let graph = Graph::new(args.uri, args.username, args.password).await.unwrap();

    graph
        .run(query("CREATE CONSTRAINT IF NOT EXISTS FOR (n:PathNode) REQUIRE (n.area_id, n.node_id) IS UNIQUE"))
        .await
        .unwrap();

    graph
        .run(query("MATCH (n:PathNode) DETACH DELETE n"))
        .await
        .unwrap();

    let mut all_nodes = Vec::new();
    let mut all_relationships = Vec::new();

    for node_file in &node_files {
        if node_file.nodes.is_empty() {
            continue;
        }

        for node in &node_file.nodes {
            let mut node_data = BoltMap::new();
            node_data.put("type".into(), BoltType::from(node.node_type.to_string()));
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
            node_data.put("is_not_highway".into(), BoltType::from(node.is_not_highway));
            node_data.put("is_highway".into(), BoltType::from(node.is_highway));
            node_data.put("parking".into(), BoltType::from(node.parking));
            all_nodes.push(node_data);

            let link_start = node.link_id as usize;
            for i in 0..node.link_count {
                let link_index = link_start + i as usize;

                if link_index < node_file.links.len()
                    && link_index < node_file.link_lengths.len()
                    && link_index < node_file.navigation_links.len()
                {
                    let link = &node_file.links[link_index];
                    let link_length = &node_file.link_lengths[link_index];

                    let mut rel_data = BoltMap::new();
                    rel_data.put("from_area_id".into(), BoltType::from(node.area_id as i64));
                    rel_data.put("from_node_id".into(), BoltType::from(node.node_id as i64));
                    rel_data.put("to_area_id".into(), BoltType::from(link.area_id as i64));
                    rel_data.put("to_node_id".into(), BoltType::from(link.node_id as i64));

                    rel_data.put("length".into(), BoltType::from(link_length.length as i64));

                    all_relationships.push(rel_data);
                }
            }
        }
    }

    const BATCH_SIZE: usize = 1000;

    let mut index = 1;
    for chunk in all_nodes.chunks(BATCH_SIZE) {
        println!(
            "Processing relationship chunk {} of {}",
            index,
            (all_relationships.len() + BATCH_SIZE - 1) / BATCH_SIZE
        );
        let mut nodes_list = BoltList::new();
        for node_map in chunk {
            nodes_list.push(BoltType::Map(node_map.clone()));
        }
        graph
            .run(
                query(
                    "UNWIND $nodes AS nodeData \
                    CREATE (n:PathNode {\
                        type: nodeData.type,\
                        x: nodeData.x,\
                        y: nodeData.y,\
                        z: nodeData.z,\
                        link_id: nodeData.link_id,\
                        area_id: nodeData.area_id,\
                        node_id: nodeData.node_id,\
                        path_width: nodeData.path_width,\
                        flood_fill: nodeData.flood_fill,\
                        link_count: nodeData.link_count,\
                        traffic_level: nodeData.traffic_level,\
                        emergency_vehicle_only: nodeData.emergency_vehicle_only,\
                        is_highway: nodeData.is_highway,\
                        parking: nodeData.parking\
                    })",
                )
                .param("nodes", BoltType::List(nodes_list)),
            )
            .await
            .unwrap();
    }

    println!("Nodes count {}", nodes_count);

    index = 1;
    for chunk in all_relationships.chunks(BATCH_SIZE) {
        println!(
            "Processing relationship chunk {} of {}",
            index,
            (all_relationships.len() + BATCH_SIZE - 1) / BATCH_SIZE
        );
        index += 1;
        let mut rels_list = BoltList::new();
        for rel_map in chunk {
            rels_list.push(BoltType::Map(rel_map.clone()));
        }
        graph
            .run(query("UNWIND $rels AS relData \
                MATCH (from:PathNode {area_id: relData.from_area_id, node_id: relData.from_node_id}) \
                MATCH (to:PathNode {area_id: relData.to_area_id, node_id: relData.to_node_id}) \
                CREATE (from)-[:CONNECTS_TO {length: relData.length}]->(to)")
                .param("rels", BoltType::List(rels_list)))
            .await
            .unwrap();
    }

    println!("Relationships count {}", all_relationships.len());
}
