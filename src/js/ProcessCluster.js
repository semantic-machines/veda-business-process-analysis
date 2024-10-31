import {Component, html} from 'veda-client';
import BusinessProcessCard from './BusinessProcessCard.js';

export default class ProcessCluster extends Component(HTMLElement) {
  static tag = 'bpa-process-cluster';
  async render() {
    return html`
      <div class="d-flex align-items-center justify-content-between mb-2">
        <h4 property="rdfs:label" class="mb-0"></h4>
        <strong>${this.dataset.totalTime} ч/год</strong>
      </div>
      <div rel="v-bpa:hasProcess" style="margin-left:2em">
        <${BusinessProcessCard} about={{this.model.id}}></${BusinessProcessCard}>
      </div>
    `;
  }
}

customElements.define(ProcessCluster.tag, ProcessCluster);
