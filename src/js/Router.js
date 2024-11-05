import {Model, Component, Router as VedaRouter} from 'veda-client';

export default class Router extends Component(HTMLElement) {
  static tag = 'bpa-router';

  router = new VedaRouter;

  key = `Router_lastHash`;

  pre() {
    this.router.add('#/:component', async (component) => {
      localStorage.setItem(this.key, location.hash);
      component = await import(`./${component}.js`);
      this.replaceChildren(document.createElement(`${component.default}`));
    });

    this.router.add('#/:component/:id', async (component, id) => {
      localStorage.setItem(this.key, location.hash);
      component = await import(`./${component}.js`);
      component = document.createElement(`${component.default}`);
      component.model = new Model(id);
      this.replaceChildren(component);
    });
  }

  post() {
    const lastHash = localStorage.getItem(this.key) || '#/ProcessOverview';
    this.router.go(location.hash || lastHash);
  }
}

customElements.define(Router.tag, Router);
