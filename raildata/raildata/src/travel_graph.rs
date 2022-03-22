/** Copyright James Lomax 2020 */

use crate::stations::{StationId, StationList};
use crate::timetable::{ServiceId, Timetable, RailTime};
use crate::fixed_links;
use crate::fixed_links::FixedLinkKind;

#[derive(Clone, PartialEq, Debug)]
pub struct RailLink {
    pub dst: StationId,
    pub service: ServiceId,
    pub depart: RailTime,
    pub time: u32
}

#[derive(Clone, PartialEq, Debug)]
pub struct FixedLink {
    pub dst: StationId,
    pub time: u32,
    pub kind: FixedLinkKind
}

#[derive(Clone, PartialEq, Debug)]
pub enum Link {
    Rail(RailLink),
    Fixed(FixedLink),
    Dummy
}

impl Link {
    fn simple_rail(dst: StationId, service: ServiceId, depart: &str, time: u32) -> Self {
        Link::Rail(RailLink {
            dst: dst,
            service: service,
            depart: RailTime::from_24h(depart).unwrap(),
            time: time
        })
    }
    
    fn simple_fixed(dst: StationId, time: u32, kind: FixedLinkKind) -> Self {
        Link::Fixed(FixedLink {
            dst: dst,
            time: time,
            kind: kind
        })
    }

    fn service(&self) -> Option<ServiceId> {
        match self {
            Link::Rail(rl) => Some(rl.service),
            _ => None
        }
    }

    /**
     * Any time we are changing (i.e. not just sitting) from self to other service
     */
    fn ischange(&self, other: &Self) -> bool {
        self.service() != other.service() || self.service() == None
    }
}

pub struct Journey {
    pub origin: StationId,
    pub depart: RailTime,
    pub time: u32,
    pub links: Vec<Link>
}

#[derive(Clone, PartialEq, Debug)]
struct TGNode {
    links: Vec<Link>,
    transfer_time: u32
}

#[derive(Clone, PartialEq, Debug)]
pub struct TravelGraph {
    stations: Vec<TGNode>
}

impl TravelGraph {
    pub fn new(stations: &StationList, fixedlinks: &Vec<fixed_links::FixedLink>, timetable: &Timetable) -> Self {
        // Initialise stations vector based on station list
        let mut graph = TravelGraph {
            stations: Vec::with_capacity(stations.count())
        };

        for station in stations.iter() {
            graph.stations.push(TGNode {
                links: Vec::with_capacity(16),
                transfer_time: station.min_change_time
            })
        }
        
        // Add all the fixed links
        for flink in fixedlinks {
            graph.stations[flink.a].links.push(Link::simple_fixed(flink.b, flink.time, flink.kind));
            graph.stations[flink.b].links.push(Link::simple_fixed(flink.a, flink.time, flink.kind));
        }

        // Iterate over the services in timetable and add connections
        for service in &timetable.services {
            for i in 0..(service.stops.len() - 1) {
                let s1 = &service.stops[i];
                let s2 = &service.stops[i+1];
                graph.stations[s1.station].links.push(
                    Link::Rail(RailLink {
                        dst: s2.station,
                        service: service.id,
                        depart: s1.departure.clone(),
                        time: s1.departure.timetil(&s2.arrival)
                    })
                );
            }
        }

        return graph;
    }

    /**
     * Compute the journey times to each destination
     * 
     * @param depart    Earliest departure time
     * @param origin    Start station
     * @param destinations  List of destinations to extract journeys for
     * @param contingency   Time (seconds) to allow for each change of train services
     * @param flexi_depart  Time (seconds) from the earliest departure to the latest first train we would take. 0 means depart ASAP.
     */
    pub fn compute_journeys(&self, depart: RailTime, origin: StationId, destinations: Vec<StationId>, contingency: u32, flexi_depart: u32) -> Vec<Journey> {
        let mut pathfinder = dijkstras::TimeDijkstras::new(self.stations.len(), contingency);
        pathfinder.perform(self, origin, depart, flexi_depart);

        destinations.iter().map(|dest| {
            pathfinder.best_journey(*dest)
        }).collect()
    }

    pub fn stat_edges(&self) -> (usize, usize, usize) {
        let mut total = 0;
        let mut min = 0;
        let mut max = 0;
        for st in &self.stations {
            let l = st.links.len();
            total += l;
            min = std::cmp::min(min, l);
            max = std::cmp::max(max, l);
        }
        return (total, min, max);
    }
}


mod dijkstras {
    use super::*;
    use std::collections::BTreeSet;

    #[derive(Eq, PartialEq, Clone)]
    struct ToVisit {
        station: StationId,
        time: u32
    }

    // Ordering by time required to pick next station to visit
    impl std::cmp::Ord for ToVisit {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            if self.time == other.time {
                self.station.cmp(&other.station)
            } else {
                self.time.cmp(&other.time)
            }
        }
    }

    impl std::cmp::PartialOrd for ToVisit {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    #[derive(Clone)]
    struct BestJourney {
        time: u32,
        depart: RailTime,
        last_station: StationId,
        last_link: Link
    }

    pub struct TimeDijkstras {
        visitq: BTreeSet<ToVisit>,
        contingency: u32,
        nodes: Vec<BestJourney>,
        origin: StationId,
        flexi_depart: u32
    }

    /** Travel Dijkstras....
     * 
     * Store BestJourney for each station
     * 
     * Store set of ToVisit's (visitq), sorted by time in descending order,
     * which is used to pick the next station to visit.
     * 
     * Start by adding ($originstation, 0), then continually pick off set to visit..
     * 
     * Visiting:
     *  - If the ToVisit.time is > the current best in the station, it's an old ToVisit, discard!
     * Iterate through all the links:
     *  - If a link leads to an improved route to another station, apply the improvement,
     *  and add current station and reached station to visitq.
     *  - If there are no improving links, don't re-add ourselves (we're done at this station)
     * 
     * The algorithm is complete when visitq is empty.
     */
    impl TimeDijkstras {
        pub fn new(station_count: usize, contingency: u32) -> Self {
            let mut s = Self {
                visitq: BTreeSet::new(),
                contingency: contingency,
                nodes: Vec::new(),
                origin: 0,
                flexi_depart: 0
            };
            s.nodes.resize(station_count, BestJourney {
                time: std::u32::MAX,
                depart: RailTime::new(0, 0),
                last_station: 0,
                last_link: Link::Dummy
            });
            return s;
        }

        pub fn perform(&mut self, graph: &TravelGraph, start_station: StationId, start_time: RailTime, flexi_depart: u32) {
            self.visitq.clear();
            self.nodes[start_station] = BestJourney {
                time: 0,
                depart: start_time,
                last_station: start_station,
                last_link: Link::Dummy
            };
            self.visitq.insert(ToVisit {
                station: start_station,
                time: 0
            });

            self.origin = start_station;
            self.flexi_depart = flexi_depart;

            // While visitq is non empty
            while let Some(tovisit) = self.visitq.pop_first() {
                // If tovisit.time > best.time then no point visiting
                if tovisit.time <= self.nodes[tovisit.station].time {
                    // If tovisit.time < best.time then somethings gone wrong
                    assert_eq!(tovisit.time, self.nodes[tovisit.station].time);

                    self.visit_next(&graph, tovisit);
                }
            }
        }

        fn visit_next(&mut self, graph: &TravelGraph, tovisit: ToVisit) {
            let curtime = self.nodes[tovisit.station].depart;
            let lastlink = self.nodes[tovisit.station].last_link.clone();

            for link in &graph.stations[tovisit.station].links {
                match link {
                    Link::Rail(rlink) => {
                        let chngtime = if lastlink.ischange(&link) {
                            graph.stations[tovisit.station].transfer_time + self.contingency
                        } else {
                            0
                        };

                        let waittime = if tovisit.station == self.origin && curtime.timetil(&rlink.depart) < self.flexi_depart {
                            // Origin station, person can arrive on time for train
                            0
                        } else {
                            // Normal situation, person must wait for train
                            chngtime + curtime.add(chngtime).timetil(&rlink.depart)
                        };
                        let dsttime = tovisit.time + waittime + rlink.time;
                        
                        if dsttime < self.nodes[rlink.dst].time {
                            // Update best
                            self.update_best(rlink.dst, dsttime, rlink.depart.add(rlink.time), tovisit.station, link.clone());

                            // Done visiting
                            self.visitq.insert(tovisit);
                            return;
                        }
                    },
                    Link::Fixed(flink) => {
                        let dsttime = tovisit.time + flink.time;

                        if dsttime < self.nodes[flink.dst].time {
                            // Update best
                            self.update_best(flink.dst, dsttime, curtime.add(flink.time), tovisit.station, link.clone());

                            // Done visiting
                            self.visitq.insert(tovisit);
                            return;
                        }
                    },
                    _ => { }
                }
            }
        }

        fn update_best(&mut self, station: StationId, time: u32, depart: RailTime, last: StationId, link: Link) {
            let mut best = &mut self.nodes[station];
            best.time = time;
            best.depart = depart;
            best.last_station = last;
            best.last_link = link;

            self.visitq.insert(ToVisit {
                time: time,
                station: station
            });
        }

        pub fn best_journey(&self, destination: StationId) -> Journey {
            // Create a journey by backtracking
            let mut links = Vec::new();

            let mut best = self.nodes[destination].clone();
            let mut depart = best.depart.clone();
            let time = best.time;
            while best.last_link != Link::Dummy {
                if let (Some(Link::Rail(rlast)), Link::Rail(rnext)) = (links.last_mut(), &best.last_link) {
                    if rlast.service == rnext.service {
                        // Same service, update rlast with rnext assuming departure from new station
                        rlast.depart = rnext.depart;
                        rlast.time += rnext.time;
                    } else {
                        // New service, add link
                        links.push(best.last_link.clone());    
                    }
                } else {
                    // New service, add link
                    links.push(best.last_link.clone());
                }

                match &best.last_link {
                    Link::Rail(rl) => { 
                        depart = rl.depart;
                    }
                    Link::Fixed(fl) => {
                        depart = depart.sub(fl.time)
                    }
                    _ => {}
                }

                best = self.nodes[best.last_station].clone();
            }

            links.reverse();

            Journey {
                origin: best.last_station, // Start station stores last_station=start_station
                depart: depart,
                time: time,
                links: links
            }
        }
    }

    pub fn print_plantuml(graph: &TravelGraph, paths: &TimeDijkstras) {
        println!("@startuml");
        for id in 0..graph.stations.len() {
            println!("[{} ({})] as d{}", id, paths.nodes[id].time / 60, id);
        }

        for (id, node) in graph.stations.iter().enumerate() {
            for link in &node.links {
                match link {
                    Link::Rail(rlink) => {
                        print!("d{} --> d{} : ", id, rlink.dst);
                        println!("R({}, {}, {})", rlink.service, rlink.depart.to_24h(), rlink.time/60);
                    },
                    Link::Fixed(flink) => {
                        print!("d{} --> d{} : ", id, flink.dst);
                        println!("F({}, {:?})", flink.time/60, flink.kind);
                    }
                    _ => {}
                }
            }
        }
        println!("@enduml");
    }

    pub fn print_journey(journey: &Journey) {
        print!("{}@{}", journey.origin, journey.depart.to_24h());

        for link in &journey.links {
            match link {
                Link::Rail(rl) => {
                    print!(" -[{}@{}]-> {}", rl.service, rl.depart.to_24h(), rl.dst);
                }
                _ => {
                    print!(" -?-> ?");
                }
            }
        }

        println!(" (total={})", journey.time/60);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::stations::Station;
    use crate::timetable::{Service, Stop};

    #[test]
    fn test_simple_graph() {
        // Construct a simple two-way service
        let stations = StationList::new(vec![
            Station::simple("CAMBDGE", "Cambridge", "CBG"),
            Station::simple("KINGSX", "London Kings Cross", "KGX")
        ]);
        
        let fixedlinks = vec![
            fixed_links::FixedLink {
                a: 0,
                b: 1,
                time: 5*60,
                kind: FixedLinkKind::Bus
            }
        ];

        let timetable = Timetable {
            services: vec![
                Service {
                    id: 0,
                    train_uid: "OUTBOUND".to_string(),
                    stops: vec![
                        Stop::simple(0, "0000", "0000"),
                        Stop::simple(1, "0100", "0100")
                    ]
                },
                Service {
                    id: 1,
                    train_uid: "INBOUND".to_string(),
                    stops: vec![
                        Stop::simple(1, "0110", "0110"),
                        Stop::simple(0, "0215", "0215")
                    ]
                }
            ]
        };

        let graph = TravelGraph::new(&stations, &fixedlinks, &timetable);

        assert_eq!(graph, TravelGraph {
            stations: vec![
                TGNode {
                    links: vec![
                        Link::simple_fixed(1, 5*60, FixedLinkKind::Bus),
                        Link::simple_rail(1, 0, "0000", 60*60)
                    ],
                    transfer_time: 0
                },
                TGNode {
                    links: vec![
                        Link::simple_fixed(0, 5*60, FixedLinkKind::Bus),
                        Link::simple_rail(0, 1, "0110", 65*60)
                    ],
                    transfer_time: 0
                }
            ]
        });
    }

    #[test]
    fn test_time_dijkstras() {
        // This simple graph example consists of 3 stations in a row, 0,1,2
        // Links:
        //  0 -> 2 : 0000 -> 0100 s=0
        //  0 -> 1 : 0130 -> 0205 s=1
        //  1 -> 2 : 0030 -> 0105 s=2
        //  1 -> 2 : 0130 -> 0205 s=4
        //  2 -> 1 : 0110 -> 0130 s=3
        //  1 -> 0 : 0130 -> 0145 s=3
        let graph = TravelGraph {
            stations: vec![
                TGNode {
                    links: vec![
                        Link::simple_rail(2, 0, "0000", 60*60),
                        Link::simple_rail(1, 1, "0130", 35*60)
                    ],
                    transfer_time: 0
                },
                TGNode {
                    links: vec![
                        Link::simple_rail(2, 2, "0030", 35*60),
                        Link::simple_rail(2, 4, "0130", 35*60),
                        Link::simple_rail(0, 3, "0130", 15*60)
                    ],
                    transfer_time: 0
                },
                TGNode {
                    links: vec![
                        Link::simple_rail(1, 3, "0110", 20*60)
                    ],
                    transfer_time: 0
                }
            ]
        };

        let mut paths = dijkstras::TimeDijkstras::new(3, 0);
        paths.perform(&graph, 0, RailTime::new(0, 0), 0);

        let j1 = paths.best_journey(1);

        assert_eq!(j1.time, 90*60);
        let j2 = paths.best_journey(2);
        assert_eq!(j2.time, 60*60);

        // Try it from 2
        let journeys = graph.compute_journeys(RailTime::new(1, 0), 2, vec![0, 1], 0, 0);
        assert_eq!(journeys[1].time, 30*60);
        assert_eq!(journeys[0].time, 45*60);
    }

    #[test]
    fn test_dijkstras_transfer() {
        // Transfer times test, three stations 0,1,2, with services:
        //  0 -> 1 : 0000 -> 0030 (~0)
        //  0 -> 2 : 0030 -> 0110 (~1)
        //  1 -> 2 : 0035 -> 0100 (~2)
        //  1 -> 2 : 0105 -> 0130 (~3)
        let graph = TravelGraph {
            stations: vec![
                TGNode {
                    links: vec![
                        Link::simple_rail(1, 0, "0000", 30*60),
                        Link::simple_rail(2, 1, "0030", 40*60)
                    ],
                    transfer_time: 2*60
                },
                TGNode {
                    links: vec![
                        Link::simple_rail(2, 2, "0035", 25*60),
                        Link::simple_rail(2, 3, "0105", 25*60)
                    ],
                    transfer_time: 2*60
                },
                TGNode {
                    links: vec![],
                    transfer_time: 2*60
                }
            ]
        };

        let journeys = graph.compute_journeys(RailTime::new(23, 50), 0, vec![1, 2], 0, 0);
        assert_eq!(journeys[0].time, 40*60);
        assert_eq!(journeys[1].time, 70*60);
        assert_eq!(journeys[1].links.len(), 2);

        let journeys = graph.compute_journeys(RailTime::new(23, 50), 0, vec![1, 2], 4*60, 0);
        assert_eq!(journeys[0].time, 40*60);
        assert_eq!(journeys[1].time, 80*60);
        assert_eq!(journeys[1].links.len(), 1);
            
        // Test that for unreachable nodes, we get u32::MAX
        // AND test that with a origin_time we allow flexi_depart we only count the time from departure
        let journeys = graph.compute_journeys(RailTime::new(0, 0), 1, vec![0, 2], 4*60, 60*60);
        assert_eq!(journeys[0].time, std::u32::MAX);
        assert_eq!(journeys[1].time, 25*60);
        assert_eq!(journeys[1].depart, RailTime::new(0, 35));
    }

    #[test]
    fn test_fixed_link_graph() {
        // Transfer times test, three stations 0,1,2 with services:
        // 0 -> 2 : 0000 -> 0100 (~0)
        // 1 -> 2 : 0020 -> 0040 (~1)
        // 2 -> 1 : 0100 -> 0120 (~2)
        // And a walk between 0 and 1 of 10 mins
        let graph = TravelGraph {
            stations: vec![
                TGNode {
                    links: vec![
                        Link::simple_rail(2, 0, "0000", 60*60),
                        Link::simple_fixed(1, 10*60, FixedLinkKind::Walk)
                    ],
                    transfer_time: 2*60
                },
                TGNode {
                    links: vec![
                        Link::simple_rail(2, 1, "0020", 20*60),
                        Link::simple_fixed(0, 10*60, FixedLinkKind::Walk)
                    ],
                    transfer_time: 2*60
                },
                TGNode {
                    links: vec![Link::simple_rail(1, 2, "0100", 20*60)],
                    transfer_time: 2*60
                }
            ]
        };

        // From station 0
        let journeys = graph.compute_journeys(RailTime::new(0, 0), 0, vec![1, 2], 0, 0);
        assert_eq!(journeys[0].time, 10*60);
        assert_eq!(journeys[0].links, vec![Link::simple_fixed(1, 10*60, FixedLinkKind::Walk)]);
        assert_eq!(journeys[1].time, 40*60);
        assert_eq!(journeys[1].links, vec![
            Link::simple_fixed(1, 10*60, FixedLinkKind::Walk),
            Link::simple_rail(2, 1, "0020", 20*60)
        ]);

        // From station 2
        let journeys = graph.compute_journeys(RailTime::new(0, 0), 2, vec![0, 1], 0, 0);
        assert_eq!(journeys[0].time, 90*60);
        assert_eq!(journeys[0].links, vec![
            Link::simple_rail(1, 2, "0100", 20*60),
            Link::simple_fixed(0, 10*60, FixedLinkKind::Walk)
        ]);
        assert_eq!(journeys[1].time, 80*60);
        assert_eq!(journeys[1].links, vec![Link::simple_rail(1, 2, "0100", 20*60)]);
    }
}
