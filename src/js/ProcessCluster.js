import {Component, html} from 'veda-client';
import BusinessProcessCard from './BusinessProcessCard.js';

export default class ProcessCluster extends Component(HTMLElement) {
  static tag = 'bpa-process-cluster';
  async render() {
    return html`
      <h4 property="rdfs:label"></h4>
      <div rel="v-bpa:hasProcess" style="margin-left:2em">
        <${BusinessProcessCard} about={{this.model.id}}></${BusinessProcessCard}>
      </div>
    `;
  }
}

customElements.define(ProcessCluster.tag, ProcessCluster);
