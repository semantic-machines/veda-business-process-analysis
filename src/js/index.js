import {Backend, Subscription} from 'veda-client';
import App from './App.js';

Backend.init('http://localhost');
Subscription.init('ws://localhost/ccus');

const app = document.createElement(`${App}`);
document.body.appendChild(app);
