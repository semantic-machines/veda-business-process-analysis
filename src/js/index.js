import './install_sw.js';
import '../scss/app.scss';
import '../../node_modules/bootstrap/dist/js/bootstrap.min.js';
import App from './App.js';
import {Backend, Subscription} from 'veda-client';

import ('./options.js').then((options) => {
  Backend.init(options.base);
  Subscription.init(options.ccus);
  
  const app = document.createElement(`${App}`);
  document.body.appendChild(app);
});
