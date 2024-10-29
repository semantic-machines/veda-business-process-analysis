import {Router, Model} from 'veda-client';
import Label from './Label.js';

const router = new Router;

router.add('#/:id', async (id) => {
  const model = new Model(id);
  await model.load();
  const main = document.querySelector('#main');
  const label = document.createElement(`${Label}`);
  label.model = model;
  main.innerHTML = '';
  main.appendChild(label);
});
