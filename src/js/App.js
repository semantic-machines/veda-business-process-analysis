import {Component, html, Router, timeout} from 'veda-client';
import Auth from './Auth.js';
import Header from './Header.js';
import BpaRouter from './BpaRouter.js';
import Footer from './Footer.js';

export default class App extends Component(HTMLElement) {
  static toString() {
    return 'bpa-app';
  }

  static get observedAttributes() {
    return ['authenticated'];
  }

  async attributeChangedCallback(name, oldValue, newValue) {
    await this.update();
  }

  async render() {
    if (!this.hasAttribute('authenticated')) {
      return html`<${Auth}></${Auth}>`;
    }
    return html`
      <div class="container p-0">
        <${Header}></${Header}>
        <${BpaRouter}></${BpaRouter}>
        <${Footer}></${Footer}>
      </div>
    `;
  }
}


customElements.define(App.toString(), App);
