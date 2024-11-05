import {Component, html, Backend, Model} from 'veda-client';
import ClusterCard from './ClusterCard.js';

let queryResult;

export default class ClusterList extends Component(HTMLElement) {
  static tag = 'bpa-cluster-list';

  async added() {
    if (!queryResult) {
      const params = new Model;
      params['rdf:type'] = 'v-s:QueryParams';
      params['v-s:storedQuery'] = 'v-bpa:AllProcessClusters';
      const {rows: clusters} = await Backend.stored_query(params);
      queryResult = clusters;
    }
    this.clusters = queryResult;
  }
  
  async render() {
    return html`
      <div class="sheet">
        <div class="d-flex justify-content-between align-items-center mb-3">
          <div class="d-flex justify-content-start align-items-center">
            <div class="me-3 fs-1">
              <i class="bi bi-collection"></i>
            </div>
            <div>
              <h3 about="v-bpa:ProcessClusters" property="rdfs:label" class="mb-1"></h3>
              <h5 class="mb-0">
                <span class="align-bottom" about="v-bpa:Clustered" property="rdfs:label"></span>&nbsp;
                <span class="badge bg-success">${this.clusters.reduce((acc, [,,clustered]) => acc + clustered, 0)}</span>
              </h5>
            </div>
          </div>
          <div class="text-end"> 
            <strong about="v-bpa:TotalTimeEffort" property="rdfs:label"></strong>
            <p class="text-muted mb-0 mt-1">
              ${this.clusters.reduce((acc, [,totalTime]) => acc + totalTime, 0)}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </p>
          </div>
        </div>
        ${this.clusters.map(([clusterId, totalTime]) => html`
          <hr class="my-0">
          <${ClusterCard} about=${clusterId} data-total-time=${totalTime}></${ClusterCard}>
        `).join('')}
      </div>
    `;
  }
}

customElements.define(ClusterList.tag, ClusterList);
