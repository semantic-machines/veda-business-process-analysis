import {Component, html} from 'veda-client';
import ProcessCard from './ProcessCard';

export default class ClusterView extends Component(HTMLElement) {
  static tag = 'bpa-cluster-view';
  
  render() {
    return html`
      <div class="sheet">
        <div class="d-flex justify-content-between align-items-center">
          <div>
            <p class="mb-0 text-muted" about="v-bpa:ProcessCluster" property="rdfs:label"></p>
            <h3 class="mb-0">
              <i class="bi bi-collection me-2"></i>
              <span property="rdfs:label"></span> 
            </h3>
          </div>
        </div>
        <hr>
        <div class="row">
          <div class="col-12 col-md-9">
            <p class="mb-0 text-muted" about="v-bpa:proposedClusterName" property="rdfs:label"></p>
            <p class="fw-bold" property="v-bpa:proposedClusterName"></p>

            <p class="mb-0 text-muted" about="v-bpa:proposedClusterDescription" property="rdfs:label"></p>
            <p class="fw-bold" property="v-bpa:proposedClusterDescription"></p>

            <p class="mb-0 text-muted" about="v-bpa:proposedDepartment" property="rdfs:label"></p>
            <p class="fs-6 badge bg-secondary" property="v-bpa:proposedDepartment"></p>

            <p class="mb-0 text-muted" about="v-bpa:proposedFrequency" property="rdfs:label"></p>
            <p class="fs-6 badge text-bg-light border border-secondary text-muted">
              <i class="bi bi-arrow-repeat me-1"></i>
              <span property="v-bpa:proposedFrequency"></span>&nbsp;
              <span about="v-bpa:TimesPerYear" property="rdfs:label"></span>
            </p>

            <p class="mb-0 text-muted" about="v-bpa:estimatedLaborCost" property="rdfs:label"></p>
            <p class="fw-bold">
              <span property="v-bpa:estimatedLaborCost"></span>&nbsp;
              <span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </p>
          </div>
          <div class="col-12 col-md-3 border-start border-secondary border-opacity-25">
            <p class="mb-0 text-muted" about="v-bpa:proposedParticipants" property="rdfs:label"></p>
            <p class="fw-bold" property="v-bpa:proposedParticipants"></p>
          </div>
        </div>
      </div>
      <div class="sheet">
        <h4>
          <span about="v-bpa:Clustered" property="rdfs:label"></span>&nbsp;
          <span class="badge text-bg-success">
            ${this.model['v-bpa:hasProcess']?.length ?? 0}
          </span>
        </h4>
        <hr>
        <div class="row px-3">
          <div class="col-12 col-md-6">
            <p class="mb-0 text-muted" about="v-bpa:clusterSimilarities" property="rdfs:label"></p>
            <p class="fw-bold mb-0" property="v-bpa:clusterSimilarities"></p>
          </div>
          <div class="col-12 col-md-6">
            <p class="mb-0 text-muted" about="v-bpa:clusterDifferences" property="rdfs:label"></p>
            <p class="fw-bold mb-0" property="v-bpa:clusterDifferences"></p>
          </div>
        </div>
        <div rel="v-bpa:hasProcess" class="mt-3 d-flex flex-column gap-3">
          <${ProcessCard} about={{this.model.id}}></${ProcessCard}>
        </div>
      </div>
    `;
  }
}

customElements.define(ClusterView.tag, ClusterView);
