use std::fmt;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::PathBuf;

const HEADER_SIZE: usize = 20;
const NODE_SIZE: usize = 28;
const NAVIGATION_NODE_SIZE: usize = 14;
const LINK_SIZE: usize = 4;
const FILLER_SIZE: usize = 768;
const NAVIGATION_LINK_SIZE: usize = 2;
const LINK_LENGTH_SIZE: usize = 1;
const PATH_INTERSECTION_FLAGS_SIZE: usize = 1;
const UNKNOWN_DATA_SIZE: usize = 192 * 2;

pub struct NodeFile {
    pub header: PathHeader,
    pub nodes: Vec<Node>,
    pub navigation_nodes: Vec<NavigationNode>,
    pub links: Vec<Link>,
    pub navigation_links: Vec<NavigationLink>,
    pub link_lengths: Vec<LinkLength>,
    pub path_intersection_flags: Vec<PathIntersectionFlags>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    Car,
    Boat,
    Ped,
}
impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct PathHeader {
    pub number_of_nodes: u32,
    pub number_of_vehicle_nodes: u32,
    pub number_of_ped_nodes: u32,
    pub number_of_navi_nodes: u32,
    pub number_of_links: u32,
}

pub struct Node {
    pub node_type: NodeType,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub link_id: u16,
    pub area_id: u16,
    pub node_id: u16,
    pub path_width: u8,
    pub flood_fill: u8,
    pub link_count: u8,
    pub traffic_level: u8,
    pub emergency_vehicle_only: bool,
    pub is_not_highway: bool,
    pub is_highway: bool,
    pub parking: bool,
}

pub struct NavigationNode {
    pub x: f32,
    pub y: f32,
    pub area_id: u16,
    pub node_id: u16,
    pub direction_x: f32,
    pub direction_y: f32,
    pub path_node_width: u8,
    pub number_of_left_lanes: u8,
    pub number_of_right_lanes: u8,
    pub traffic_light_direction_behavior: u8,
    pub traffic_light_behavior: u8,
    pub train_corssing: u8,
}

pub struct Link {
    pub area_id: u16,
    pub node_id: u16,
}

pub struct NavigationLink {
    pub navigation_node_id: u16,
    pub area_id: u16,
}

pub struct LinkLength {
    pub length: u8,
}

pub struct PathIntersectionFlags {
    pub road_crossing: bool,
    pub pedestrian_traffic_light: bool,
}

pub fn parse_path_file(path: &PathBuf) -> Option<NodeFile> {
    match File::open(&path) {
        Err(_) => None,
        Ok(mut file) => {
            let header = parse_path_header(&mut file);
            let nodes = parse_path_nodes(&mut file, &header);
            let navigation_nodes = parse_navigation_nodes(&mut file, &header);
            let links = parse_path_links(&mut file, &header);
            file.seek_relative(FILLER_SIZE as i64).unwrap();
            let navigation_links = parse_navigation_links(&mut file, &header);
            let link_lengths = parse_link_lengths(&mut file, &header);
            let path_intersection_flags = parse_path_intersection_flags(&mut file, &header);
            file.seek_relative(UNKNOWN_DATA_SIZE as i64).unwrap();

            let current_position = file.seek(std::io::SeekFrom::Current(0)).unwrap();
            let length = file.metadata().unwrap().len();

            if current_position as u64 != length {
                panic!(
                    "Unexpected file size expected {} found {}",
                    length, current_position
                );
            }

            Some(NodeFile {
                header,
                nodes,
                navigation_nodes,
                links,
                navigation_links,
                link_lengths,
                path_intersection_flags,
            })
        }
    }
}

fn parse_path_header(file: &mut File) -> PathHeader {
    let mut buf: [u8; HEADER_SIZE] = [0; HEADER_SIZE];
    let bytes = file.read(&mut buf).unwrap();

    if bytes != HEADER_SIZE {
        panic!("Invalid header size");
    }

    PathHeader {
        number_of_nodes: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
        number_of_vehicle_nodes: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        number_of_ped_nodes: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        number_of_navi_nodes: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
        number_of_links: u32::from_le_bytes(buf[16..20].try_into().unwrap()),
    }
}

fn parse_path_nodes(file: &mut File, header: &PathHeader) -> Vec<Node> {
    let mut nodes = Vec::with_capacity(header.number_of_nodes as usize);

    for index in 0..header.number_of_nodes {
        let mut node_buf: [u8; NODE_SIZE] = [0; NODE_SIZE];
        let bytes = file.read(&mut node_buf).unwrap();

        if bytes != NODE_SIZE {
            panic!("Invalid node size");
        }

        let link_count = node_buf[24] & 0b0000_1111;
        let traffic_level = (node_buf[24] & 0b0011_0000 >> 4) as u8;

        let boats = node_buf[24] & 0b1000_0000 > 0;
        let emergency_vehicle_only = node_buf[25] & 0b0000_0001 > 0;
        let is_not_highway = node_buf[25] & 0b0001_0000 > 0;
        let is_highway = node_buf[25] & 0b0010_0000 > 0;
        let parking = node_buf[26] & 0b0010_0000 > 0;

        let node = Node {
            node_type: if index < header.number_of_vehicle_nodes {
                if boats { NodeType::Boat } else { NodeType::Car }
            } else {
                NodeType::Ped
            },
            x: i16::from_le_bytes(node_buf[8..10].try_into().unwrap()) as f32 / 8f32,
            y: i16::from_le_bytes(node_buf[10..12].try_into().unwrap()) as f32 / 8f32,
            z: i16::from_le_bytes(node_buf[12..14].try_into().unwrap()) as f32 / 8f32,
            link_id: u16::from_le_bytes(node_buf[16..18].try_into().unwrap()),
            area_id: u16::from_le_bytes(node_buf[18..20].try_into().unwrap()),
            node_id: u16::from_le_bytes(node_buf[20..22].try_into().unwrap()),
            path_width: node_buf[22],
            flood_fill: node_buf[23],
            link_count,
            traffic_level,
            emergency_vehicle_only,
            is_not_highway,
            is_highway,
            parking,
        };

        nodes.push(node);
    }

    nodes
}

fn parse_navigation_nodes(file: &mut File, header: &PathHeader) -> Vec<NavigationNode> {
    let mut nodes = Vec::new();

    for _ in 0..header.number_of_navi_nodes {
        let mut node_buf: [u8; NAVIGATION_NODE_SIZE] = [0; NAVIGATION_NODE_SIZE];
        let bytes = file.read(&mut node_buf).unwrap();

        if bytes != NAVIGATION_NODE_SIZE {
            panic!("Invalid node size");
        }

        let node = NavigationNode {
            x: i16::from_le_bytes(node_buf[0..2].try_into().unwrap()) as f32 / 8f32,
            y: i16::from_le_bytes(node_buf[2..4].try_into().unwrap()) as f32 / 8f32,
            area_id: u16::from_le_bytes(node_buf[4..6].try_into().unwrap()),
            node_id: u16::from_le_bytes(node_buf[6..8].try_into().unwrap()),
            direction_x: (node_buf[8] as i8) as f32 / 100f32,
            direction_y: (node_buf[9] as i8) as f32 / 100f32,
            path_node_width: node_buf[10],
            number_of_left_lanes: node_buf[11] & 0b1110_0000 >> 5,
            number_of_right_lanes: node_buf[11] & 0b0001_1100 >> 2,
            traffic_light_direction_behavior: node_buf[11] & 0b0000_0010 >> 1,
            traffic_light_behavior: node_buf[12] & 0b1100_0000 >> 6,
            train_corssing: node_buf[12] & 0b0010_0000 >> 5,
        };

        nodes.push(node);
    }

    nodes
}

fn parse_path_links(file: &mut File, header: &PathHeader) -> Vec<Link> {
    let mut nodes = Vec::with_capacity(header.number_of_links as usize);

    for _ in 0..header.number_of_links {
        let mut node_buf: [u8; LINK_SIZE] = [0; LINK_SIZE];
        let bytes = file.read(&mut node_buf).unwrap();

        if bytes != LINK_SIZE {
            panic!("Invalid node size");
        }

        nodes.push(Link {
            area_id: u16::from_le_bytes(node_buf[0..2].try_into().unwrap()),
            node_id: u16::from_le_bytes(node_buf[2..4].try_into().unwrap()),
        });
    }

    nodes
}

fn parse_navigation_links(file: &mut File, header: &PathHeader) -> Vec<NavigationLink> {
    let mut nodes = Vec::with_capacity(header.number_of_links as usize);

    for _ in 0..header.number_of_links {
        let mut node_buf: [u8; NAVIGATION_LINK_SIZE] = [0; NAVIGATION_LINK_SIZE];
        let bytes = file.read(&mut node_buf).unwrap();

        if bytes != NAVIGATION_LINK_SIZE {
            panic!("Invalid node size");
        }

        nodes.push(NavigationLink {
            navigation_node_id: u16::from_le_bytes(node_buf[0..2].try_into().unwrap())
                & 0b1111_1111_1100_0000 >> 6,
            area_id: (node_buf[1] & 0b0011_1111) as u16,
        });
    }

    nodes
}

fn parse_link_lengths(file: &mut File, header: &PathHeader) -> Vec<LinkLength> {
    let mut nodes = Vec::with_capacity(header.number_of_links as usize);

    for _ in 0..header.number_of_links {
        let mut node_buf: [u8; LINK_LENGTH_SIZE] = [0; LINK_LENGTH_SIZE];
        let bytes = file.read(&mut node_buf).unwrap();

        if bytes != LINK_LENGTH_SIZE {
            panic!("Invalid node size");
        }

        nodes.push(LinkLength {
            length: node_buf[0],
        });
    }

    nodes
}

fn parse_path_intersection_flags(
    file: &mut File,
    header: &PathHeader,
) -> Vec<PathIntersectionFlags> {
    let mut nodes = Vec::with_capacity(header.number_of_links as usize);

    for _ in 0..header.number_of_links {
        let mut node_buf: [u8; PATH_INTERSECTION_FLAGS_SIZE] = [0; PATH_INTERSECTION_FLAGS_SIZE];
        let bytes = file.read(&mut node_buf).unwrap();

        if bytes != PATH_INTERSECTION_FLAGS_SIZE {
            panic!("Invalid node size");
        }

        nodes.push(PathIntersectionFlags {
            road_crossing: node_buf[0] & 0b1000_0000 >> 7 > 0,
            pedestrian_traffic_light: node_buf[0] & 0b0100_0000 >> 6 > 0,
        });
    }

    nodes
}
