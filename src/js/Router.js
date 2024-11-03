import {Model, Component, Router as VedaRouter} from 'veda-client';
import BusinessProcessOverview from './BusinessProcessOverview.js';
import BusinessProcessList from './BusinessProcessList.js';
import ClusterList from './ClusterList.js';
import BusinessProcessView from './BusinessProcessView.js';
import ClusterView from './ClusterView.js';

export default class Router extends Component(HTMLElement) {
  static tag = 'bpa-router';

  router = new VedaRouter;

  pre() {
    this.router.add('/', () => this.router.go('#/BusinessProcessOverview/false'));
    this.router.add('#/BusinessProcessList', () => this.replaceChildren(document.createElement(`${BusinessProcessList}`)));
    this.router.add('#/ProcessClusterList', () => this.replaceChildren(document.createElement(`${ClusterList}`)));
    this.router.add('#/BusinessProcessOverview', () => this.replaceChildren(document.createElement(`${BusinessProcessOverview}`)));
    this.router.add('#/BusinessProcessView/:id', (id) => {
      const component = document.createElement(`${BusinessProcessView}`);
      component.model = new Model(id);
      this.replaceChildren(component);
    });
    this.router.add('#/ClusterView/:id', (id) => {
      const component = document.createElement(`${ClusterView}`);
      component.model = new Model(id);
      this.replaceChildren(component);
    });
  }

  post() {
    this.router.go(location.hash || '#/BusinessProcessOverview/false');
  }
}

customElements.define(Router.tag, Router);
