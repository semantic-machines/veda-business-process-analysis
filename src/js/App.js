import {Component, html} from 'veda-client';
import Auth from './Auth.js';
import Header from './Header.js';
import Router from './Router.js';
import Footer from './Footer.js';

export default class App extends Component(HTMLElement) {
  static tag = 'bpa-application';
    
  static get observedAttributes() {
    return ['authenticated'];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    this.update();
  }

  async render() {
    if (!this.hasAttribute('authenticated')) {
      return html`<${Auth}></${Auth}>`;
    }
    return html`
      <div class="container p-0 mb-5">
        <${Header}></${Header}>
        <${Router}></${Router}>
        <${Footer}></${Footer}>
      </div>
    `;
  }
}
customElements.define(App.tag, App);
