/** Copyright James Lomax 2020 */

use std::io;
use std::io::BufRead;

use regex::Regex;

use crate::stations::{StationId, StationList};

pub type ServiceId = u32;

// RailTime is represented by seconds since 00:00am. (TODO: 3am?)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailTime {
    secs: u32
}

lazy_static! {
    static ref TIME_FORMAT_REGEX: Regex = Regex::new(r"^\d{2}\d{2}.?$").unwrap();
}

impl RailTime {
    pub fn new(hours: u32, mins: u32) -> Self {
        Self {
            secs: (hours*60*60 + mins*60) % (24*60*60)
        }
    }

    pub fn from_24h(timestr: &str) -> Option<Self> {
        if TIME_FORMAT_REGEX.is_match(timestr) {
            let hrs = timestr[0..2].parse::<u32>().unwrap();
            let mns = timestr[2..4].parse::<u32>().unwrap();

            let secs = hrs*60*60 + mns*60;

            Some(Self {
                secs: secs
            })
        } else {
            None
        }
    }

    pub fn to_24h(&self) -> String {
        let a = self.secs % (60*60);
        let hrs = (self.secs - a) / (60*60);
        let b = a % 60;
        let mins = (a - b) / 60;
        return format!("{:02}{:02}", hrs, mins);
    }

    /**
     * Returns the number of seconds until the $other time,
     * if it is in the past, then it will wrap around, assuming its
     * in the next day
     */
    pub fn timetil(&self, other: &RailTime) -> u32 {
        if self.secs > other.secs {
            return other.secs + 24*60*60 - self.secs;
        } else {
            return other.secs - self.secs;
        }
    }

    pub fn add(&self, secs: u32) -> Self {
        Self {
            secs: (self.secs + secs) % (24*60*60)
        }
    }

    pub fn sub(&self, secs: u32) -> Self {
        let s = if secs > self.secs {
            self.secs + 24*60*60 - secs
        } else {
            self.secs - secs
        };

        Self {
            secs: s
        }
    }
}

#[derive(Debug)]
pub struct Stop {
    pub station: StationId,
    // Arrival and departure time are "public" if the record exists, scheduled otherwise.
    // First/last stops use the same time for arrival and departure
    pub arrival: RailTime,
    pub departure: RailTime
}

impl Stop {
    pub fn simple(station: StationId, arrival: &str, departure: &str) -> Self {
        Self {
            station: station,
            arrival: RailTime::from_24h(arrival).unwrap(),
            departure: RailTime::from_24h(departure).unwrap()
        }
    }
}

#[derive(Debug)]
pub struct Service {
    pub id: ServiceId,
    pub train_uid: String,
    pub stops: Vec<Stop>
}

// There's more but these are the ones I'm probably interested in...
make_record_type!(
    McaScheduleRecord,
    (transaction_type, 2, 1),
    (train_uid, 3, 6),
    (days_run, 21, 7),
    (bank_holiday_running, 28, 1),
    (power_type, 50, 3)
);

make_record_type!(
    McaOriginStationRecord,
    (tiploc, 2, 7),
    (sched_departure, 10, 5),
    (public_departure, 15, 4),
    (platform, 19, 3)
);

make_record_type!(
    McaIntermediateStationRecord,
    (tiploc, 2, 7),
    (tiploc_suffix, 9, 1),
    (scheduled_arrival, 10, 5),
    (scheduled_departure, 15, 5),
    (scheduled_pass, 20, 5),
    (public_arrival, 25, 4),
    (public_departure, 29, 4),
    (platform, 33, 3)
);

make_record_type!(
    McaTerminalStationRecord,
    (tiploc, 2, 7),
    (tiploc_suffix, 9, 1),
    (scheduled_arrival, 10, 5),
    (public_arrival, 15, 4),
    (platform, 19, 3)
);

impl Service {
    pub fn read_service_entry(stations: &StationList, reader: &mut dyn BufRead) -> io::Result<Option<Service>> {
        let mut service = Service {
            id: 0,
            train_uid: String::new(),
            stops: Vec::new()
        };

        let mut has_record = false;

        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? > 2 {
                match &line[0..2] {
                    "BS" => {
                        let r = McaScheduleRecord::read(&line)?;
                        service.train_uid = r.train_uid.to_string();
                        has_record = true;
                    }
                    "LO" => {
                        let r = McaOriginStationRecord::read(&line)?;
                        if let Some(station) = stations.get_by_tiploc(r.tiploc) {
                            let dep_time = RailTime::from_24h(r.public_departure).unwrap();
                            let stop = Stop {
                                station: station.id,
                                arrival: dep_time,
                                departure: dep_time
                            };
                            service.stops.push(stop);
                        }
                    }
                    "LI" => {
                        let r = McaIntermediateStationRecord::read(&line)?;
                        if let Some(station) = stations.get_by_tiploc(r.tiploc) {
                            let station_id = station.id;

                            let pass_time = RailTime::from_24h(r.scheduled_pass);
                            let arr_time = RailTime::from_24h(r.public_arrival);
                            let dep_time = RailTime::from_24h(r.public_departure);

                            if let Some(_passtime) = pass_time {
                                // Skip, we dont record passes
                            } else {
                                service.stops.push(Stop {
                                    station: station_id,
                                    arrival: arr_time.unwrap(),
                                    departure: dep_time.unwrap()
                                });
                            }
                        } else {
                            //println!("Skipping missing station {}", tiploc);
                        }
                    }
                    "LT" => {
                        let r = McaTerminalStationRecord::read(&line)?;
                        if let Some(station) = stations.get_by_tiploc(r.tiploc) {
                            let arr_time = RailTime::from_24h(r.public_arrival).unwrap();
                            let stop = Stop {
                                station: station.id,
                                arrival: arr_time,
                                departure: arr_time
                            };
                            service.stops.push(stop);
                        }

                        return Ok(Some(service));
                    }
                    _ => {}
                }
            } else {
                if has_record {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF/short line while reading service..."));
                } else {
                    return Ok(None);
                }
            }
        }
    }
}


pub struct Timetable {
    pub services: Vec<Service>
}

impl Timetable {
    pub fn read_mca_file(stations: &StationList, reader: &mut dyn BufRead) -> io::Result<Self> {
        let mut timetable = Timetable {
            services: Vec::new()
        };

        while let Some(mut service) = Service::read_service_entry(stations, reader)? {
            let next_id = timetable.services.len() as ServiceId;
            service.id = next_id;
            timetable.services.push(service);
        }

        return Ok(timetable);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_railtime() {
        assert_eq!(RailTime::from_24h("0025"), Some(RailTime { secs: 25*60 }));
        assert_eq!(RailTime::from_24h("2359"), Some(RailTime { secs: 23*60*60+59*60 }));

        let t1 = RailTime::from_24h("1325").unwrap();
        let t2 = RailTime::from_24h("1412").unwrap();
        assert_eq!(t1.timetil(&t2), 47*60);
        
        let t1 = RailTime::from_24h("2355").unwrap();
        let t2 = RailTime::from_24h("0020").unwrap();
        assert_eq!(t1.timetil(&t2), 25*60);
    }

    #[test]
    fn test_service_parse() {
        let mca_file = "/!! Comment line!
BSNL221082005232012120000010 PXX1T25    121725000 EMU365 100D     B            P
BX         GNYGN161701                                                          
LOKLYNN   1045 10451         TB                                                 
LIWATLGTN 1052 1052H     105210521        T                                     
CRCAMBDGE XX1T25    121725000 EMU365 100D     B                    GN161703     
LICAMBDGE 1136H1144H     113711448        T -U                                  
LISTEVNGE           1211H000000002                      1                       
LTKNGX    1235 12356     TF                                                     
";
        let msn_file = "/!! Start of file
A                             FILE-SPEC=05 1.00 25/08/20 18.05.31   748           
A    KINGS LYNN                    1KLYNN  KLN   KLN15623 63201 5                 
A    WATLINGTON                    0WATLGTNWTG   WTG15612 63110 5                 
A    CAMBRIDGE                     2CAMBDGECBG   CBG15462 62573 5                 
A    STEVENAGE                     2STEVNGESVG   SVG15235 62238 4                 
A    LONDON KINGS CROSS            3KNGX   KGX   KGX15303 6183015                 
";
        
        let mut msn_read = io::Cursor::new(&msn_file);
        let stations = StationList::read_msn_file(&mut msn_read).unwrap();

        let mut mca_read = io::Cursor::new(&mca_file);
        
        let service = Service::read_service_entry(&stations, &mut mca_read).unwrap().unwrap();
        println!("service: {:?}", service);
        assert_eq!(service.train_uid, "L22108");
        assert_eq!(service.stops.len(), 4);
        assert_eq!(service.stops.get(2).unwrap().station, stations.get_by_name("CAMBRIDGE").unwrap().id);
        assert_eq!(service.stops.get(2).unwrap().departure.to_24h(), "1144");
    }

    #[test]
    fn test_timetable() {
        let mca_file = "/!! Comment line!
BSNL221082005232012120000010 PXX1T25    121725000 EMU365 100D     B            P
BX         GNYGN161701                                                          
LOKLYNN   1045 10451         TB                                                 
LTKNGX    1235 12356     TF                                                     
BSNL221192005232012120000010 PXX1T30    121725000 EMU365 100D     B            P
BX         GNYGN162200                                                          
LOKNGX    1242 12429  B      TB                                                 
LTKLYNN   1431 14311     TF                                                     
";
        let msn_file = "/!! Start of file
A                             FILE-SPEC=05 1.00 25/08/20 18.05.31   748           
A    KINGS LYNN                    1KLYNN  KLN   KLN15623 63201 5                 
A    LONDON KINGS CROSS            3KNGX   KGX   KGX15303 6183015                 
";

        let mut msn_read = io::Cursor::new(&msn_file);
        let stations = StationList::read_msn_file(&mut msn_read).unwrap();

        let mut mca_read = io::Cursor::new(&mca_file);

        let timetable = Timetable::read_mca_file(&stations, &mut mca_read).unwrap();
        assert_eq!(timetable.services.len(), 2);
        assert_eq!(timetable.services[1].train_uid, "L22119");
        assert_eq!(timetable.services[1].stops.len(), 2);
    }
}
