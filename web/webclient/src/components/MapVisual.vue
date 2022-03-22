<template>
  <div class="map-visual">
    <svg width="100%" height="100%" ref="mapvis"></svg>
  </div>
</template>

<script>
import geojson from "@/assets/uk.json";
import * as d3 from "d3";
import { StationInfo, ComputeJourneyRequest, computeJourneys } from "@/api";
import { default as Vector } from "immutable-vector2d";

function formatTime(time) {
  const mins = time % 60;
  const hrs = (time - mins) / 60;
  return hrs > 0 ? `${hrs}h${mins}` : `${mins}m`;
}

export default {
  name: 'MapVisual',
  props: {
    stations: Array
  },
  data() {
    return {
      geoPath: Object,
      geoProjection: Object,
      stationGroup: Object,
      currentScale: Number,
      m: Object,
      selectedStation: StationInfo
    };
  },
  components: {
  },
  mounted() {
    this.selectedStation = null;

    let svg = d3.select(".map-visual svg");
    let m = svg.append("g");

    this.geoProjection = d3.geoMercator()
      .scale(5000)
      .center([-5, 53]);

    this.geoPath = d3.geoPath()
      .projection(this.geoProjection);

    var u = m.selectAll('path').data(geojson.features);
    u.enter()
      .append('path')
      .attr('d', this.geoPath)
      .style("fill", "none")
      .style("stroke", "black");

    this.currentScale = 1.0;
    this.stationGroup = m.append("g").attr("class", "stations");
    this.updateStations();

    // Zoom must apply to inner group(<g>) but by called off the outter element (<svg>)
    // so that it doesnt jump all over the place.
    let zoom = d3.zoom().on("zoom", this.onZoom);
    svg.call(zoom);
    this.m = m;
  },
  watch: {
    "stations": function() {
      this.updateStations();
    }
  },
  methods: {
    onZoom(event) {
      this.currentScale = event.transform.k;
      this.m.attr("transform", event.transform);
      this.rescaleElements();
    },
    updateStations() {
      this.stationGroup.selectAll("*").remove();

      if (this.selectedStation) {
        this.drawJourneyLines();
      }

      for (const s of this.stations) {
        const ll = s.getLatLon();
        const ss = this.geoProjection(ll);
        const selclass = this.selectedStation == s ? "selected" : "deselected";

        this.stationGroup.append("circle")
          .attr("class", selclass)
          .attr("cx", ss[0])
          .attr("cy", ss[1])
          .attr("fill", "blue")
          .on("click", this.clickStation.bind(this, s));
        this.stationGroup.append("text")
          .attr("class", selclass)
          .attr("x", ss[0])
          .attr("y", ss[1])
          .attr("font-family", "monospace")
          .attr("font-weight", "bold")
          .attr("text-anchor", "middle")
          .attr("fill", "yellow")
          .attr("dominant-baseline", "central")
          .text(s.crs)
          .on("click", this.clickStation.bind(this, s));
      }

      this.rescaleElements();
    },
    rescaleElements() {
      this.stationGroup
        .selectAll("circle.deselected")
        .attr("r", 12/this.currentScale);
      this.stationGroup
        .selectAll("text.deselected")
        .attr("font-size", (12/this.currentScale) + "px");
      this.stationGroup
        .selectAll("circle.selected")
        .attr("r", 16/this.currentScale);
      this.stationGroup
        .selectAll("text.selected")
        .attr("font-size", (16/this.currentScale) + "px");
      this.stationGroup
        .selectAll("line")
        .attr("stroke-width", 5/this.currentScale + "");
      this.stationGroup
        .selectAll("text.journey-time")
        .attr("font-size", (16/this.currentScale) + "px");
    },
    clickStation(station) {
      this.selectedStation = station;
      this.updateStations();
    },
    drawJourneyLines() {
      const startll = this.selectedStation.getLatLon();
      const startss = this.geoProjection(startll);

      for (const s of this.stations) {
        if (s == this.selectedStation)
          continue;
        
        const ll = s.getLatLon();
        const ss = this.geoProjection(ll);

        this.stationGroup.append("line")
          .attr("x1", startss[0])
          .attr("y1", startss[1])
          .attr("x2", ss[0])
          .attr("y2", ss[1])
          .attr("stroke", "gray")
      }

      this.updateJourneyTimes();
    },
    async updateJourneyTimes() {
      const statcrs = this.selectedStation.crs;
      let jreq = new ComputeJourneyRequest("0800", statcrs, 15*60, 2*60*60);
      jreq.dests = this.stations.map((v) => v.crs).filter((v) => v != statcrs);
      let jtimes = await computeJourneys(jreq);
      
      for (const [d_crs, time] of Object.entries(jtimes)) {
        let dst = this.getStationByCrs(d_crs);
        const linestart = this.geoProjection(this.selectedStation.getLatLon());
        const lineend = this.geoProjection(dst.getLatLon());
        const linetext = formatTime(time);
        this.addLineText(linestart, lineend, linetext);

        this.rescaleElements();
      }
    },
    getStationByCrs(crs) {
      return this.stations.filter((v) => v.crs == crs)[0];
    },
    addLineText(linestart, lineend, text) {
      console.log(Vector);
      const start = Vector.fromArray(linestart);
      const end = Vector.fromArray(lineend);

      const dir = end.subtract(start);
      const halfway = start.add(dir.multiply(0.5));

      this.stationGroup.append("text")
        .attr("class", "journey-time")
        .attr("x", halfway.x)
        .attr("y", halfway.y)
        .attr("font-family", "sans-serif")
        .attr("text-anchor", "middle")
        .attr("fill", "black")
        .attr("dominant-baseline", "central")
        .text(text)
    }
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped>
.map-visual {
  width: 100%;
  height: 100%;
}
</style>
