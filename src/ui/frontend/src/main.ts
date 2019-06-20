import Vue from 'vue';
import App from './App.vue';
import router from './router';
import store from './store';

Vue.config.productionTip = false;

const tornadoApp = new Vue({
  router,
  store,
  render: (h) => h(App),
}).$mount('#tornado-app');