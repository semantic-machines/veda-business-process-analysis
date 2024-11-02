import '@ungap/custom-elements';
import './install_sw.js';
import '../scss/app.scss';
import '../../node_modules/bootstrap/dist/js/bootstrap.min.js';
import App from './App.js';
import {Backend, Subscription} from 'veda-client';

Backend.init();
Subscription.init();

const app = document.createElement(`${App}`);
document.body.appendChild(app);
