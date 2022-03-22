/** Copyright James Lomax 2020 */

use std::collections::HashMap;
use std::collections::HashSet;
use std::io;

use std::io::BufRead;
use crate::utils::append_err_context;

use crate::record_parsing::parse_or_invalid;

pub type StationId = usize;

#[derive(std::fmt::Debug)]
pub struct Station {
    pub id: StationId,
    pub tiplocs: Vec<String>,
    pub crs_code: String,
    pub names: Vec<String>,
    pub min_change_time: u32,
    pub gref_east: i32,
    pub gref_north: i32
}

make_record_type!(
    MsnStationRecord,
    (name, 5, 26),
    (cate_interchange, 35, 1),
    (tiploc, 36, 7),
    (crs_ref, 43, 3),
    (crs, 49, 3),
    (os_gref_east, 53, 4), // For some reason, east has a leading 1, north a leading 6??
    (os_gref_north, 59, 4),
    (min_change_time, 63, 2)
);

make_record_type!(
    MsnAliasRecord,
    (name, 5, 26),
    (alias, 36, 26)
);

impl Station {
    pub fn simple(tiploc: &str, name: &str, crs: &str) -> Self {
        Self {
            id: 0,
            tiplocs: vec![tiploc.to_string()],
            crs_code: crs.to_string(),
            names: vec![name.to_string()],
            min_change_time: 0,
            gref_east: 0,
            gref_north: 0
        }
    }

    pub fn from_msn_a_record(line: String) -> io::Result<Self> {
        let record = MsnStationRecord::read(&line)?;
        
        return Ok(Self {
            id: 0,
            tiplocs: vec![record.tiploc.to_string()],
            crs_code: record.crs.to_string(),
            names: vec![record.name.to_string()],
            min_change_time: parse_or_invalid(record.min_change_time, "min_change_time")?,
            gref_east: parse_or_invalid(record.os_gref_east, "os_gref_east")?,
            gref_north: parse_or_invalid(record.os_gref_north, "os_gref_north")?
        });
    }

    pub fn update_from_other(&mut self, other: &Self) {
        // They should be essentially the same if they have the same CRS
        assert_eq!(self.crs_code, other.crs_code);
        // Some details are discarded

        // Union of names
        for name in &other.names {
            if !self.names.contains(name) {
                self.names.push(name.clone());
            }
        }

        // But the TIPLOC will be different
        self.tiplocs.append(&mut other.tiplocs.clone());
    }
}

pub struct StationList {
    // Map of stations by TIPLOC
    stations: Vec<Station>,
    
    // Map of IDs by TIPLOC
    by_tiploc: HashMap<String, StationId>,

    // Map of ID by name (including any Aliases)
    by_name: HashMap<String, StationId>,

    // Map of IDs by CRS code
    by_crs: HashMap<String, StationId>
}

fn insert_for(map: &mut HashMap<String, StationId>, names: &Vec<String>, station: StationId) {
    for name in names {
        map.insert(name.clone(), station);
    }
}

impl StationList {
    pub fn new(statlist: Vec<Station>) -> Self {
        let mut stations = Self {
            stations: statlist,
            by_tiploc: HashMap::new(),
            by_name: HashMap::new(),
            by_crs: HashMap::new()
        };
        
        // Populate the lookup tables
        for (i, station) in stations.stations.iter_mut().enumerate() {
            insert_for(&mut stations.by_tiploc, &station.tiplocs, i);
            insert_for(&mut stations.by_name, &station.names, i);
            stations.by_crs.insert(station.crs_code.clone(), i as StationId);
            station.id = i as StationId;
        }

        return stations;
    }

    pub fn read_msn_file(reader: &mut dyn BufRead) -> io::Result<Self> {
        let mut stations = Self {
            stations: Vec::new(),
            by_tiploc: HashMap::new(),
            by_name: HashMap::new(),
            by_crs: HashMap::new()
        };

        // Iterate over file and populate stations map
        let mut a_rec_head = true;
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            let line_num = index + 1;

            match line.chars().nth(0) {
                Some('A') => {
                    // Station record
                    if a_rec_head {
                        // Skip first line
                        a_rec_head = false;
                    } else {
                        let rs = Station::from_msn_a_record(line);
                        let rs = append_err_context(rs, format!("On line {}", line_num));
                        let mut s = rs?;
                        let crs = s.crs_code.clone();
                        let tiplocs = s.tiplocs.clone();

                        // Check if we've already got a station with this CRS
                        if let Some(station) = stations.by_crs.get_mut(&crs).cloned() {
                            stations.stations[station].update_from_other(&s);
                            insert_for(&mut stations.by_tiploc, &tiplocs, station);
                            insert_for(&mut stations.by_name, &s.names, station);
                        } else {
                            // New ID is the current length (next index)
                            let next_id = stations.stations.len() as StationId;
                            s.id = next_id;
                            let names = s.names.clone();
                            stations.stations.push(s);
                            stations.by_crs.insert(crs, next_id);
                            insert_for(&mut stations.by_name, &names, next_id);
                            insert_for(&mut stations.by_tiploc, &tiplocs, next_id);
                        }
                    }
                }
                Some('L') => {
                    // Alias record
                    let rs = MsnAliasRecord::read(&line);
                    let rs = append_err_context(rs, format!("On line {}", line_num));
                    let r = rs?;
                    
                    let maybe_id = stations.by_name.get(r.name).cloned();
                    if let Some(id) = maybe_id {
                        stations.by_name.insert(r.alias.to_string(), id);
                        stations.stations[id as usize].names.push(r.alias.to_string());
                    } else {
                        let msg = format!("On line {}: Reference to non-existent station {}", index, r.name);
                        return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
                    }
                }
                _ => {}
            }
        }

        return Ok(stations);
    }

    pub fn iter(&self) -> std::slice::Iter<Station> {
        self.stations.iter()
    }

    pub fn get(&self, id: StationId) -> Option<&Station> {
        self.stations.get(id as usize)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Station> {
        match self.by_name.get(name).cloned() {
            Some(id) => self.get(id),
            None => None
        }
    }

    pub fn get_by_tiploc(&self, name: &str) -> Option<&Station> {
        match self.by_tiploc.get(name).cloned() {
            Some(id) => self.get(id),
            None => None
        }
    }

    pub fn get_by_crs(&self, name: &str) -> Option<&Station> {
        match self.by_crs.get(name).cloned() {
            Some(id) => self.get(id),
            None => None
        }
    }

    pub fn name_search(&self, name: &str) -> HashSet<StationId> {
        let mut rs = HashSet::new();
        for (key, id) in self.by_name.iter() {
            if key.contains(name) {
                rs.insert(*id);
            }
        }
        rs
    }

    pub fn count(&self) -> usize {
        self.stations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_station_msn_parse() {
        let rec1 = "A    ABBEY WOOD MTR                9ABWDXR ABX   ABW15473 61790 4";
        let s = Station::from_msn_a_record(rec1.to_string()).unwrap();
        assert_eq!(s.tiplocs, vec!["ABWDXR"]);
        assert_eq!(s.crs_code, "ABW");
        assert_eq!(s.names, vec!["ABBEY WOOD MTR"]);
        assert_eq!(s.min_change_time, 4);
        assert_eq!(s.gref_east, 5473);
        assert_eq!(s.gref_north, 1790);

        let rec2 = "A    ABBEY WOOD MTR                9ABWDXR ABX   ABW15473 617";
        let s = Station::from_msn_a_record(rec2.to_string());
        s.expect_err("Record too short!");
    }

    #[test]
    fn test_stations_read() {
        let msn = "/!! Start of file...
A                             FILE-SPEC=05 1.00 25/08/20 18.05.31   748           
A    ABBEY WOOD                    0ABWD   ABW   ABW15473 61790 4                         
A    ABERDARE                      0ABDARE ABA   ABA13004 62027 3                 
A    ABERDEEN                      2ABRDEENABD   ABD13942 68058 5                         
A    CAMBRIDGE NORTH               2CAMBNTHCMB   CMB15475 62607 5                 
A    CAMBRIDGE NORTH Stand         9CMBNTSTCMB   CMB15475 62607 5                 
L    ABERDARE                       ABAHDAR                                       
";
        
        let mut msn_read = io::Cursor::new(&msn);
        let stations = StationList::read_msn_file(&mut msn_read).unwrap();

        let abdare1 = stations.get_by_tiploc("ABDARE")
            .expect("Expected station with TIPLOC ABDARE");
        assert_eq!(abdare1.names, vec!["ABERDARE", "ABAHDAR"]);
        assert_eq!(abdare1.gref_north, 2027);

        let abdare2 = stations.get_by_name("ABAHDAR")
            .expect("Expected station with name ABAHDAR");
        assert_eq!(abdare1.names, vec!["ABERDARE", "ABAHDAR"]);
        assert_eq!(abdare2.gref_north, 2027);

        let abdare3 = stations.get_by_name("ABERDARE")
            .expect("Expected station with name ABERDARE");
        assert_eq!(abdare1.names, vec!["ABERDARE", "ABAHDAR"]);
        assert_eq!(abdare3.gref_north, 2027);

        let camnorth_id = stations.get_by_crs("CMB")
            .expect("Expected station with name CAMBRIDGE NORTH")
            .id;
        let camnorth = stations.get(camnorth_id).unwrap();
        assert_eq!(camnorth.names, vec!["CAMBRIDGE NORTH", "CAMBRIDGE NORTH Stand"]);
        assert_eq!(camnorth.tiplocs, vec!["CAMBNTH", "CMBNTST"]);
        assert_eq!(camnorth.crs_code, "CMB");
    }
}
