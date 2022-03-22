<template>
  <div id="app">
    <div class="ui-overlay">
    <SetupPane :stations="stations"/>
    </div>

    <div class="map-vis">
    <MapVisual :stations="stations"/>
    </div>
  </div>
</template>

<script>
import Vue from 'vue'
import Autocomplete from '@trevoreyre/autocomplete-vue'
import '@trevoreyre/autocomplete-vue/dist/style.css'
import { MdList } from 'vue-material/dist/components'
import 'vue-material/dist/vue-material.min.css'
import 'vue-material/dist/theme/default.css'

import SetupPane from './components/SetupPane.vue'
import MapVisual from './components/MapVisual.vue'

Vue.use(MdList);
Vue.use(Autocomplete);

export default {
  name: 'App',
  components: {
    SetupPane,
    MapVisual
  },
  data() {
    return {
      stations: []
    };
  }
}
</script>

<style>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: left;
  color: #2c3e50;
}

/* TODO: Overlay scrolling, hiding, and 'card' appearance */
.ui-overlay {
  width: 400px;
  height: 400px;
  position: absolute;
  top: 0;
  left: 0;
}

.map-vis {
  width: 100%;
  height: 100vh;
}
</style>
