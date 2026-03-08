use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use ::kml::types::{
    AltitudeMode, Coord, KmlDocument, LineStyle, LinearRing, Placemark, PolyStyle, Polygon, Style,
};
use ::kml::{Kml, KmlReader, KmlWriter};
use lc_kml_utils::model::{EChoice, InputData};

fn circle_coords(center: &Coord, radius_m: f64, num_points: usize, on_top: bool) -> Vec<Coord> {
    let earth_radius = 6_371_000.0;
    let lat = center.y.to_radians();
    let lon = center.x.to_radians();

    let mut coords = Vec::with_capacity(num_points + 1);
    for i in 0..=num_points {
        let bearing = 2.0 * std::f64::consts::PI * (i as f64) / (num_points as f64);
        let d = radius_m / earth_radius;

        let lat2 = (lat.sin() * d.cos() + lat.cos() * d.sin() * bearing.cos()).asin();
        let lon2 =
            lon + (bearing.sin() * d.sin() * lat.cos()).atan2(d.cos() - lat.sin() * lat2.sin());

        coords.push(Coord {
            x: lon2.to_degrees(),
            y: lat2.to_degrees(),
            z: Some(if on_top { 50.0 } else { 0.0 }),
        });
    }
    coords
}

fn find_placemark_point(kml: &Kml, name: &str) -> Option<Coord> {
    match kml {
        Kml::KmlDocument(doc) => doc.elements.iter().find_map(|e| find_placemark_point(e, name)),
        Kml::Document { elements, .. } => {
            elements.iter().find_map(|e| find_placemark_point(e, name))
        }
        Kml::Folder(folder) => {
            folder.elements.iter().find_map(|e| find_placemark_point(e, name))
        }
        Kml::Placemark(p) => {
            if p.name.as_deref() == Some(name) {
                if let Some(::kml::types::Geometry::Point(pt)) = &p.geometry {
                    return Some(pt.coord.clone());
                }
            }
            None
        }
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.yml> <output.kml>", args[0]);
        std::process::exit(1);
    }
    let input_yml = &args[1];
    let output_kml = &args[2];

    let yml_file = File::open(input_yml).expect("Failed to open input YAML file");
    let input_data: InputData = serde_yaml::from_reader(yml_file).expect("Failed to parse YAML");

    // Cache parsed KML files to avoid re-reading
    let mut kml_cache: HashMap<String, Kml> = HashMap::new();

    // Collect all elements for the output document
    let mut output_elements: Vec<Kml> = Vec::new();

    for choice in &input_data.choices {
        match choice {
            EChoice::ConcentricCircles(cc) => {
                let kml = kml_cache.entry(cc.center.kml.clone()).or_insert_with(|| {
                    let kml_path = Path::new(&cc.center.kml);
                    KmlReader::from_path(kml_path)
                        .expect(&format!("Failed to open {}", cc.center.kml))
                        .read()
                        .expect("Failed to parse KML")
                });

                let center = find_placemark_point(kml, &cc.center.name)
                    .expect(&format!("Placemark '{}' not found", cc.center.name));

                for (i, radius) in cc.v_radius.iter().enumerate() {
                    let style_id = format!("circle_style_{}_{}", cc.name, i);

                    output_elements.push(Kml::Style(Style {
                        id: Some(style_id.clone()),
                        line: Some(LineStyle {
                            color: "ff0000ff".to_string(),
                            width: 2.0,
                            ..Default::default()
                        }),
                        poly: Some(PolyStyle {
                            fill: false,
                            outline: true,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));

                    let coords = circle_coords(&center, *radius, 72, cc.circle_on_top);
                    let altitude_mode = if cc.circle_on_top {
                        AltitudeMode::RelativeToGround
                    } else {
                        AltitudeMode::ClampToGround
                    };
                    output_elements.push(Kml::Placemark(Placemark {
                        name: Some(format!("{} - {}m", cc.name, radius)),
                        style_url: Some(format!("#{}", style_id)),
                        geometry: Some(::kml::types::Geometry::Polygon(Polygon {
                            outer: LinearRing {
                                coords,
                                altitude_mode,
                                ..Default::default()
                            },
                            altitude_mode,
                            ..Default::default()
                        })),
                        ..Default::default()
                    }));
                }
            }
        }
    }

    let output_kml_doc = Kml::KmlDocument(KmlDocument {
        elements: vec![Kml::Document {
            attrs: HashMap::new(),
            elements: output_elements,
        }],
        ..Default::default()
    });

    let file = File::create(output_kml).expect("Failed to create output KML file");
    let mut writer = KmlWriter::from_writer(BufWriter::new(file));
    writer.write(&output_kml_doc).expect("Failed to write KML");
    println!("Written to {}", output_kml);
}
