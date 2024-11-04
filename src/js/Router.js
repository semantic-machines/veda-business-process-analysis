import {Model, Component, Router as VedaRouter} from 'veda-client';

export default class Router extends Component(HTMLElement) {
  static tag = 'bpa-router';

  router = new VedaRouter;

  pre() {
    this.router.add('/', () => this.router.go('#/ProcessOverview'));

    this.router.add('#/:component', async (component) => {
      component = await import(`./${component}.js`);
      this.replaceChildren(document.createElement(`${component.default}`));
    });

    this.router.add('#/:component/:id', async (component, id) => {
      component = await import(`./${component}.js`);
      component = document.createElement(`${component.default}`);
      component.model = new Model(id);
      this.replaceChildren(component);
    });
  }

  post() {
    this.router.go(location.hash || '#/ProcessOverview');
  }
}

customElements.define(Router.tag, Router);
