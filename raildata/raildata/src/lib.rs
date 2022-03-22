#![allow(dead_code)]
#![feature(map_first_last)]
/** Copyright James Lomax 2020 */

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod record_parsing;
mod utils;
pub mod stations;
pub mod timetable;
pub mod fixed_links;
pub mod travel_graph;

use std::fs::File;
use std::io::BufReader;
pub use stations::{Station, StationList, StationId};
pub use fixed_links::FixedLinkKind;
pub use timetable::{Timetable, RailTime, Service, ServiceId};
pub use travel_graph::{Journey, TravelGraph, Link};

pub struct RailServices {
    pub stations: StationList,
    pub fixedlinks: Vec<fixed_links::FixedLink>,
    pub timetable: Timetable,
    pub graph: TravelGraph
}

pub fn load_services(file_prefix: &str) -> std::io::Result<RailServices> {
    // Load Master Station Names (MSN) file
    let msnname = format!("{}.MSN", file_prefix);
    let msnfile = File::open(&msnname)?;
    let mut msnreader = BufReader::new(msnfile);
    let stations = StationList::read_msn_file(&mut msnreader)?;

    // Load Fixed Leg File (FLF)
    let flfname = format!("{}.FLF", file_prefix);
    let flffile = File::open(&flfname)?;
    let mut flfreader = BufReader::new(flffile);
    let fixedlinks = fixed_links::parse_fixed_links(&stations, &mut flfreader)?;

    // Load services file (MCA) file
    let mcaname = format!("{}.MCA", file_prefix);
    let mcafile = File::open(&mcaname)?;
    let mut mcareader = BufReader::with_capacity(1024*1024, mcafile);
    let timetable = Timetable::read_mca_file(&stations, &mut mcareader)?;

    // Compute graph
    let graph = TravelGraph::new(&stations, &fixedlinks, &timetable);

    return Ok(RailServices {
        stations: stations,
        fixedlinks: fixedlinks,
        timetable: timetable,
        graph: graph
    });
}
