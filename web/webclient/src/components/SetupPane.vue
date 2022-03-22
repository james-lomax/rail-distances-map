<template>
  <div class="stations-select">
    <autocomplete
      class="station-search"
      ref="stationSearcher" 
      :search="searchStations" 
      :get-result-value="getStationDescriptor"
      @submit="addStation"
      placeholder="e.g. 'Cambridge' or 'CBG'" 
      aria-label="Station search"
      auto-select/>

    <md-list>
      <md-list-item 
          v-for="station in stations" 
          :key="station.crs"
          @click="deleteStation(station)">
        {{station.names[0]}} ({{station.crs}})
      </md-list-item>
    </md-list>
    <span class="footnote">Click the station to remove</span>
  </div>
</template>

<script>
// See: https://autocomplete.trevoreyre.com/#/vue-component
import Autocomplete from '@trevoreyre/autocomplete-vue'
import * as api from '@/api';

export default {
  name: 'SetupPane',
  props: {
    stations: Array
  },
  components: {
    Autocomplete
  },
  methods: {
    async searchStations(input) {
      if (input.length >= 3) {
        return await api.stationSearch(input);
      } else {
        return [];
      }
    },

    getStationDescriptor(station) {
      return `${station.names[0]} (${station.crs})`;
    },

    addStation(station) {
      this.$refs.stationSearcher.value = "";
      if (!this.stations.find((s) => s.crs == station.crs)) {
        this.stations.splice(0, 0, station);
      }
    },

    deleteStation(station) {
      const idx = this.stations.indexOf(station);
      this.stations.splice(idx, 1);
    }
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped>
/* This doesnt work to set the result list index.. So instead we reduce other z-index...
.autocomplete-result-list {
  z-index: 99;
}
*/

.stations-select {
  width: 400px;
  max-width: 100%;
  margin-top: 10px;
  margin-left: 10px;
}

.md-list {
  width: 100%;
  max-width: 100%;
  margin-top: 10px;
  display: inline-block;
  vertical-align: top;
  z-index: 0;
  border: 1px solid rgba(#000, .12);
}

span.footnote {
  font-size: 8pt;
  font-style: italic;
  color: #bbb;
}
</style>
