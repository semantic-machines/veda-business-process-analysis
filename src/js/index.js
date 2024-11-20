import '@ungap/custom-elements';
import './install_sw.js';
import '../scss/app.scss';
import 'bootstrap';
import App from './App.js';
import {Backend, Subscription} from 'veda-client';
import './Raw.js';
import './Spinner.js';

Backend.init();
Subscription.init();

const app = document.createElement(`${App}`);
document.body.appendChild(app);
