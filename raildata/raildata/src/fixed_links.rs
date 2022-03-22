/** Copyright James Lomax 2020 */

use std::io;
use std::io::BufRead;
use regex::Regex;
use crate::stations::{StationId, StationList};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FixedLinkKind {
    Walk,
    Tube,
    Metro,
    Bus,
    Ferry,
    Transfer
}

#[derive(Debug, PartialEq, Clone)]
pub struct FixedLink {
    pub a: StationId,
    pub b: StationId,
    pub time: u32,
    pub kind: FixedLinkKind
}

fn station_or_err(stations: &StationList, crs: &str, line: usize) -> io::Result<StationId> {
    if let Some(stat) = stations.get_by_crs(crs) {
        Ok(stat.id)
    } else {
        let msg = format!("On line {}: Reference to non-existent station CRS {}", line, crs);
        Err(io::Error::new(io::ErrorKind::InvalidData, msg))
    }
}

pub fn parse_fixed_links(stations: &StationList, reader: &mut dyn BufRead) -> io::Result<Vec<FixedLink>> {
    let pattern = Regex::new("^ADDITIONAL LINK: (WALK|TUBE|METRO|BUS|FERRY|TRANSFER) BETWEEN ([A-Z]{3}) AND ([A-Z]{3}) IN +([0-9]+) MINUTES *$").unwrap();

    let mut links = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_num = index + 1;

        if let Some(caps) = pattern.captures(&line?) {
            assert_eq!(caps.len(), 5);

            let kind = match caps.get(1).unwrap().as_str() {
                "WALK" => FixedLinkKind::Walk,
                "TUBE" => FixedLinkKind::Tube,
                "METRO" => FixedLinkKind::Metro,
                "BUS" => FixedLinkKind::Bus,
                "FERRY" => FixedLinkKind::Ferry,
                "TRANSFER" => FixedLinkKind::Transfer,
                other => panic!("Unrecognised fixed link kind {}", other)
            };

            let a = station_or_err(stations, caps.get(2).unwrap().as_str(), line_num)?;
            let b = station_or_err(stations, caps.get(3).unwrap().as_str(), line_num)?;

            let mins = caps.get(4).unwrap().as_str().parse::<u32>()
                        .expect("Fixed link time parse fails despite matching [0-9]+ regex!!?");
            
            links.push(FixedLink {
                a: a,
                b: b,
                time: mins*60,
                kind: kind
            });
        }
    }

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stations::Station;

    #[test]
    fn test_fixed_links() {
        let example = "/!! Begin
ADDITIONAL LINK: FERRY BETWEEN ABC AND DEF IN  25 MINUTES  
ADDITIONAL LINK: TUBE BETWEEN DEF AND XYZ IN  45 MINUTES    ";

        let stations = StationList::new(vec![
            Station::simple("CAMBDGE", "Cambridge", "ABC"),
            Station::simple("KINGSX", "London Kings Cross", "DEF"),
            Station::simple("FOO", "FooBar", "XYZ")
        ]);

        let mut reader = io::Cursor::new(&example);
        let links = parse_fixed_links(&stations, &mut reader).unwrap();

        assert_eq!(links, vec![
            FixedLink {
                a: 0,
                b: 1,
                time: 25*60,
                kind: FixedLinkKind::Ferry
            },
            FixedLink {
                a: 1,
                b: 2,
                time: 45*60,
                kind: FixedLinkKind::Tube
            },
        ]);
    }
}
