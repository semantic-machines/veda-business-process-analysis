import {Component, html} from 'veda-client';
import BusinessProcessCard from './BusinessProcessCard.js';

export default class ProcessCluster extends Component(HTMLElement) {
  static tag = 'bpa-process-cluster';
  async render() {
    return html`
      <div class="d-flex align-items-center justify-content-between mb-2">
        <div>
          <h4 property="rdfs:label" class="mb-0"></h4>
          <div class="text-muted">
            <small property="v-bpa:clusterSimilarities"></small>
          </div>
        </div>
        <div class="text-end">
          <strong>${this.dataset.totalTime}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span></strong>
        </div>
      </div>
      <div class="mt-2 d-flex justify-content-between align-items-center mb-3">
        <div>
          <span class="badge text-bg-secondary border border-secondary me-2" property="v-bpa:clusterResponsibleDepartment"></span>
          <span class="badge text-bg-light border border-secondary text-muted">
            <i class="bi bi-arrow-repeat me-1"></i>
            <span property="v-bpa:aggregatedFrequency"></span>
            &nbsp;<span about="v-bpa:TimesPerYear" property="rdfs:label"></span>
          </span>
        </div>
        <div>
          <small class="text-muted" property="v-bpa:proposedParticipants"></small>
        </div>
      </div>
      <div rel="v-bpa:hasProcess" class="ms-4">
        <${BusinessProcessCard} about={{this.model.id}}></${BusinessProcessCard}>
      </div>
    `;
  }
}

customElements.define(ProcessCluster.tag, ProcessCluster);
