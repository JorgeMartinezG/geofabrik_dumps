use geo::{coord, Contains, Coord, LineString, Rect};

use shapefile::Shape;
use shapefile::ShapeWriter;
use std::cell::RefCell;
use std::{collections::HashMap, fs::File, io::BufWriter};

struct Area {
    writer: RefCell<ShapeWriter<BufWriter<File>>>,
    rect: Rect,
    ref_nodes: HashMap<i64, Coord>,
}

fn count<F: Fn(&osmpbfreader::Tags) -> bool>(filter: F, filename: &std::ffi::OsStr) {
    let r = std::fs::File::open(&std::path::Path::new(filename)).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);

    let barranquilla = Area {
        rect: Rect::new(
            coord! {x: -74.919218,y: 10.913888},
            coord! {x:-74.753632,y: 11.106644},
        ),
        ref_nodes: HashMap::new(),
        writer: RefCell::new(
            ShapeWriter::from_path("barranquilla.shp").expect("Cannot create shapefile"),
        ),
    };

    let medellin = Area {
        rect: Rect::new(
            coord! {x: -75.719423, y: 6.162617},
            coord! {x:-75.473408,y: 6.376421},
        ),
        ref_nodes: HashMap::new(),
        writer: RefCell::new(
            ShapeWriter::from_path("medellin.shp").expect("Cannot create shapefile"),
        ),
    };

    let mut areas = vec![barranquilla, medellin];

    for obj in pbf.par_iter().map(Result::unwrap) {
        if !filter(obj.tags()) {
            continue;
        }
        match obj {
            osmpbfreader::OsmObj::Node(node) => {
                let coord = coord! {x: node.lon(), y: node.lat()};
                areas.iter_mut().for_each(|area| {
                    if area.rect.contains(&coord) == true {
                        area.ref_nodes.insert(node.id.0.clone(), coord);
                    }
                });
            }
            osmpbfreader::OsmObj::Way(way) => {
                if way.tags.into_inner().contains_key("highway") == false {
                    continue;
                }

                // Find which area belongs the way.
                let area_with_line = areas
                    .iter()
                    .map(|area| {
                        let way_coords = way
                            .nodes
                            .iter()
                            .map(|node_id| area.ref_nodes.get(&node_id.0))
                            .collect::<Vec<Option<&Coord>>>();

                        if way.nodes.len() != way_coords.iter().filter(|opt| opt.is_some()).count()
                        {
                            return None;
                        }

                        let filtered_coords = way_coords
                            .into_iter()
                            .filter_map(|opt| opt.cloned())
                            .collect::<Vec<Coord>>();
                        let line_string =
                            geo::Geometry::LineString(LineString::new(filtered_coords));

                        let shape_ls = match shapefile::Shape::try_from(line_string).unwrap() {
                            Shape::Polyline(line) => line,
                            _ => panic!("AAAAA"),
                        };

                        return Some((area, shape_ls));
                    })
                    .filter_map(|e| e)
                    .nth(0);

                area_with_line
                    .map(|(area, line)| area.writer.borrow_mut().write_shape(&line).unwrap());
            }
            _ => {}
        }
    }
    //println!("{:?}", vec_nodes);
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    match args.len() {
        2 => {
            println!("counting objects...");
            count(|_| true, &args[1]);
        }
        3 => {
            let key = args[2].to_str().unwrap();
            println!("counting objects with \"{}\" in tags...", key);
            count(|tags| tags.contains_key(key), &args[1]);
        }
        4 => {
            let key = args[2].to_str().unwrap();
            let val = args[3].to_str().unwrap();
            println!("counting objects with tags[\"{}\"] = \"{}\"...", key, val);
            count(|tags| tags.contains(key, val), &args[1]);
        }
        _ => println!("usage: count filename [key [value]]",),
    };
}
