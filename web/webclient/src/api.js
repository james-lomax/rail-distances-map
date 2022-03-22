import * as geotools from "@/thirdparty/geotools2.js";

const API = "http://localhost:8000";

export class StationInfo {
    constructor(jsobj) {
        this.crs = jsobj.crs;
        this.names = jsobj.names;
        this.gref_east = jsobj.gref_east;
        this.gref_north = jsobj.gref_north;
    }

    getLatLon() {
        let wgs84 = geotools.osgb2wgs84(this.gref_east*100, this.gref_north*100);
        console.log("osgb east=" + this.gref_east + " north=" + this.gref_north);
        console.log("wgs lat=" + wgs84.latitude + " and long=" + wgs84.longitude);
        return [wgs84.longitude, wgs84.latitude];
    }
}

export async function stationSearch(search) {
    let rs = await fetch(`${API}/lookup/${search}`);
    let r = await rs.json();
    return r.map((v) => new StationInfo(v));
}

export class ComputeJourneyRequest {
    constructor(start_time, origin, contingency, flexi_depart) {
        this.start = start_time;
        this.origin = origin;
        this.dests = [];
        this.contingency = contingency;
        this.flexi_depart = flexi_depart;
    }
}

export async function computeJourneys(journeyReq) {
    let rs = await fetch(`${API}/computejourneys`, {
        method: "POST",
        body: JSON.stringify(journeyReq)
    });
    let journeys = await rs.json();
    let dsts = {};
    for (const journey of journeys) {
        const dst = journey.links[journey.links.length-1].dst;
        const t = journey.time / 60;
        dsts[dst] = t;
    }
    return dsts;
}
