import {Model, Component, Router as VedaRouter} from 'veda-client';
import NotFound from './NotFound.js';
import Error from './Error.js';

export default class Router extends Component(HTMLElement) {
  static tag = 'bpa-router';

  router = new VedaRouter;

  handleNotFound = (error) => {
    console.error(error);
    const notFound = document.createElement(`${NotFound}`);
    this.replaceChildren(notFound);
  }

  handleError = (error) => {
    console.error(error);
    const errorElement = document.createElement(`${Error}`);
    errorElement.error = error;
    this.replaceChildren(errorElement);
  }

  post() {
    this.router.add('#/:component', async (component) => {
      try {
        component = await import(`./${component}.js`);
      } catch (error) {
        return this.handleNotFound(error);
      }
      component = document.createElement(`${component.default}`);
      this.replaceChildren(component);
    });

    this.router.add('#/:component/:id', async (component, id) => {
      try {
        component = await import(`./${component}.js`);
      } catch (error) {
        return this.handleNotFound(error);
      }
      component = document.createElement(`${component.default}`);
      component.model = new Model(id);
      this.replaceChildren(component);
    });

    this.router.go(location.hash || '#/ProcessOverview');
  }
}

customElements.define(Router.tag, Router);
