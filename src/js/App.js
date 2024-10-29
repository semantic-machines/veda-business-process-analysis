import {Component, html, Router} from 'veda-client';
import Auth from './Auth.js';
import Header from './Header.js';
import Breadcrumb from './Breadcrumb.js';
import Footer from './Footer.js';

// Основной компонент приложения
export default class App extends Component(HTMLElement) {
  static toString() {
    return 'bpa-app';
  }

  static get observedAttributes() {
    return ['authenticated'];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (this.hasAttribute('authenticated')) {
      const router = new Router;
      router.go(location.hash || '#/v-bpa:BusinessProcessAnalysisApplication');
    }
    this.update();
  }

  async render() {
    if (!this.hasAttribute('authenticated')) {
      return html`<${Auth}></${Auth}>`;
    }
    return html`
      <div class="container p-0">
        <${Header}></${Header}>
        <${Breadcrumb}></${Breadcrumb}>
        <div id="main" class="pb-5"></div>
        <${Footer}></${Footer}>
      </div>
    `;
  }
}

customElements.define(App.toString(), App);
