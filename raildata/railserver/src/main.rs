#![feature(proc_macro_hygiene, decl_macro)]
/* Copyright James Lomax 2020 */

#[macro_use] extern crate rocket;

use rocket::State;
use rocket::response::status;
use rocket_contrib::json::Json;
use serde::{Serialize, Deserialize};

use raildata::{
    load_services, RailServices,
    Station, StationId, StationList,
    FixedLinkKind,
    RailTime, Service, ServiceId,
    Journey, Link
};

fn print_journey(stations: &StationList, journey: &Journey) {
    let startname = &stations.get(journey.origin).unwrap().crs_code;
    print!("{}@{}", startname, journey.depart.to_24h());

    for link in &journey.links {
        match link {
            Link::Rail(rl) => {
                let dstname = &stations.get(rl.dst).unwrap().crs_code;
                print!(" -[{}@{}]-> {}", rl.service, rl.depart.to_24h(), dstname);
            }
            Link::Fixed(fl) => {
                let dstname = &stations.get(fl.dst).unwrap().crs_code;
                print!(" -[{:?}]-> {}", fl.kind, dstname);
            }
            _ => {
                print!(" -?-> ?");
            }
        }
    }

    println!(" (total={})", journey.time/60);
}

#[derive(Serialize)]
struct StationInfo {
    crs: String,
    tiplocs: Vec<String>,
    names: Vec<String>,
    min_change_time: u32,
    gref_east: i32,
    gref_north: i32
}

impl StationInfo {
    fn new(s: &Station) -> Self {
        Self {
            crs: s.crs_code.clone(),
            tiplocs: s.tiplocs.clone(),
            names: s.names.clone(),
            min_change_time: s.min_change_time,
            gref_east: s.gref_east,
            gref_north: s.gref_north
        }
    }
}

#[get("/station/<crs>")]
fn station_info(rail: State<RailServices>, crs: String) -> Option<Json<StationInfo>> {
    if let Some(station) = rail.stations.get_by_crs(&crs) {
        Some(Json(StationInfo::new(station)))
    } else {
        None
    }
}

#[get("/lookup/<name>")]
fn station_lookup(rail: State<RailServices>, name: String) -> Json<Vec<StationInfo>> {
    let name = name.to_uppercase();
    let mut searchrs = rail.stations.name_search(&name);
    let mut infs = Vec::new();
    if let Some(station) = rail.stations.get_by_crs(&name) {
        infs.push(StationInfo::new(station));
        searchrs.remove(&station.id); // Don't repeat the results...
    }

    for rs in searchrs {
        infs.push(StationInfo::new(rail.stations.get(rs).unwrap()));
    }

    Json(infs)
}

#[derive(Serialize, Clone)]
struct ServiceStopInfo {
    station: String,
    arrival: String,
    departure: String
}

#[derive(Serialize, Clone)]
struct ServiceInfo {
    id: ServiceId,
    train_uid: String,
    stops: Vec<ServiceStopInfo>
}

impl ServiceInfo {
    fn new(stations: &StationList, service: &Service) -> Self {
        Self {
            id: service.id,
            train_uid: service.train_uid.clone(),
            stops: service.stops.iter().map(|stop| {
                ServiceStopInfo {
                    station: stations.get(stop.station).unwrap().crs_code.clone(),
                    arrival: stop.arrival.to_24h(),
                    departure: stop.departure.to_24h()
                }
            }).collect()
        }
    }
}

#[get("/service/<id>")]
fn service_info(rail: State<RailServices>, id: ServiceId) -> Option<Json<ServiceInfo>> {
    if let Some(service) = rail.timetable.services.get(id as usize) {
        Some(Json(ServiceInfo::new(&rail.stations, service)))
    } else {
        None
    }
}

#[derive(Deserialize)]
struct ComputeJourneysRequest {
    start: String,
    origin: String,
    dests: Vec<String>,
    contingency: u32,
    flexi_depart: u32
}

#[derive(Serialize, Clone)]
struct RailLinkInfo {
    dst: String,
    time: u32,
    depart: String,
    service: ServiceId
}

#[derive(Serialize, Clone)]
struct FixedLinkInfo {
    dst: String,
    time: u32
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum LinkInfo {
    Rail(RailLinkInfo),
    Walk(FixedLinkInfo),
    Tube(FixedLinkInfo),
    Metro(FixedLinkInfo),
    Bus(FixedLinkInfo),
    Ferry(FixedLinkInfo),
    Transfer(FixedLinkInfo),
    Dummy
}

impl LinkInfo {
    fn new(stations: &StationList, link: &Link) -> Self {
        match link {
            Link::Rail(rl) => {
                LinkInfo::Rail(RailLinkInfo {
                    dst: stations.get(rl.dst).unwrap().crs_code.clone(),
                    time: rl.time,
                    depart: rl.depart.to_24h(),
                    service: rl.service
                })
            }
            Link::Fixed(fl) => {
                let l = FixedLinkInfo {
                    dst: stations.get(fl.dst).unwrap().crs_code.clone(),
                    time: fl.time
                };

                match fl.kind {
                    FixedLinkKind::Walk => LinkInfo::Walk(l),
                    FixedLinkKind::Tube => LinkInfo::Tube(l),
                    FixedLinkKind::Metro => LinkInfo::Metro(l),
                    FixedLinkKind::Bus => LinkInfo::Bus(l),
                    FixedLinkKind::Ferry => LinkInfo::Ferry(l),
                    FixedLinkKind::Transfer => LinkInfo::Transfer(l)
                }
            }
            Link::Dummy => LinkInfo::Dummy
        }
    }
}

#[derive(Serialize, Clone)]
struct JourneyInfo {
    origin: String,
    depart: String,
    time: u32,
    links: Vec<LinkInfo>
}

#[post("/computejourneys", data = "<req>")]
fn compute_journeys(rail: State<RailServices>, req: Json<ComputeJourneysRequest>) 
        -> Result<Json<Vec<JourneyInfo>>, status::BadRequest<String>>
{
    let mut start_time = RailTime::new(0, 0);
    if let Some(st) = RailTime::from_24h(&req.start) {
        start_time = st;
    } else {
        let msg = format!("Could not parse time {}", req.start);
        return Err(status::BadRequest(Some(msg)));
    }

    let mut origin_id = 0;
    if let Some(origin) = rail.stations.get_by_crs(&req.origin) {
        origin_id = origin.id;
    } else {
        let msg = format!("Could not find CRS {}", req.origin);
        return Err(status::BadRequest(Some(msg)));
    }

    let mut dst_ids = Vec::new();
    for dst in &req.dests {
        if let Some(s) = rail.stations.get_by_crs(&dst) {
            dst_ids.push(s.id);
        } else {
            let msg = format!("Could not find CRS {}", dst);
            return Err(status::BadRequest(Some(msg)));
        }
    }

    let journeys = rail.graph.compute_journeys(start_time, origin_id, dst_ids, req.contingency, req.flexi_depart);
    let journeys = journeys.iter().map(|journey| {
        JourneyInfo {
            origin: rail.stations.get(journey.origin).unwrap().crs_code.clone(),
            depart: journey.depart.to_24h(),
            time: journey.time,
            links: journey.links.iter()
                    .map(|link| LinkInfo::new(&rail.stations, link))
                    .collect()
        }
    }).collect();

    Ok(Json(journeys))
}

fn main() {
    println!("Loading rail database... (this can take a while)");
    let rail = load_services("../../Starter/out/RJTTF748").unwrap();
    println!("Loaded {} stations, {} fixed legs and {} services!", rail.stations.count(), rail.fixedlinks.len(), rail.timetable.services.len());
    let (total, min, max) = rail.graph.stat_edges();
    println!("Loaded travel graph with ed.g.es total={} min/max = {}/{}", total, min, max);
    
    // let yat_id = rail.stations.get_by_crs("YAT").unwrap().id;
    // let dest_ids = vec!["BRI", "MAN", "PAD", "TAU", "CBG"].drain(..)
    //     .map(|crs| rail.stations.get_by_crs(crs).unwrap().id)
    //     .collect::<Vec<StationId>>();
    // println!("YAT={} and others={:?}", yat_id, dest_ids);

    // println!("Computing journeys...");
    // let journeys = rail.graph.compute_journeys(RailTime::new(9,30), yat_id, dest_ids, 15*60, 60*60);
    // for j in journeys {
    //     println!("Journey 1 taking {} mins", j.time / 60);
    //     print_journey(&rail.stations, &j);
    // }

    let default = rocket_cors::CorsOptions::default();
    let cors = default.to_cors().expect("error while building CORS object");

    rocket::ignite()
        .manage(rail)
        .mount("/", routes![
            station_info, 
            station_lookup, 
            service_info,
            compute_journeys
        ])
        .attach(cors)
        .launch();
}
