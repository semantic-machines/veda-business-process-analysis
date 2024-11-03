import {Model, Component, Router as VedaRouter} from 'veda-client';
import ProcessOverview from './ProcessOverview.js';
import ProcessList from './ProcessList.js';
import ClusterList from './ClusterList.js';
import ProcessView from './ProcessView.js';
import ClusterView from './ClusterView.js';

export default class Router extends Component(HTMLElement) {
  static tag = 'bpa-router';

  router = new VedaRouter;

  pre() {
    this.router.add('/', () => this.router.go('#/ProcessOverview'));
    this.router.add('#/ProcessList', () => this.replaceChildren(document.createElement(`${ProcessList}`)));
    this.router.add('#/ClusterList', () => this.replaceChildren(document.createElement(`${ClusterList}`)));
    this.router.add('#/ProcessOverview', () => this.replaceChildren(document.createElement(`${ProcessOverview}`)));
    this.router.add('#/ProcessView/:id', (id) => {
      const component = document.createElement(`${ProcessView}`);
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
    this.router.go(location.hash || '#/ProcessOverview');
  }
}

customElements.define(Router.tag, Router);
