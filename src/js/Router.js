import {Component, Router as VedaRouter} from 'veda-client';
import BusinessProcessList from './BusinessProcessList.js';

function isSubclassOf(child, parent) {
  let current = child;
  while (current) {
      if (current === parent) {
          return true;
      }
      current = Object.getPrototypeOf(current);
  }
  return false;
}

export default class Router extends Component(HTMLElement) {
  static tag = 'bpa-router';

  router = new VedaRouter;

  registerRoute(path, fn) {
    if (isSubclassOf(fn, HTMLElement)) {
      this.router.add(path, () => {
        this.replaceChildren(document.createElement(`${fn}`));
      });
    } else if (typeof fn === 'function') {
      this.router.add(path, fn);
    }
  }

  pre() {
    this.registerRoute('#/', BusinessProcessList);
    this.registerRoute('#/BusinessProcessList', BusinessProcessList);
  }

  post() {
    this.router.go(location.hash || '#/BusinessProcessList');
  }
}

customElements.define(Router.tag, Router);
