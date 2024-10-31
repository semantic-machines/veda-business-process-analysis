import {Component, html, Backend, Model} from 'veda-client';
import ProcessCluster from './ProcessCluster.js';

let queryResult;

export default class ProcessClusterList extends Component(HTMLElement) {
  static tag = 'bpa-process-cluster-list';

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
        <div class="d-flex justify-content-between align-items-center">
          <div>
            <h3 about="v-bpa:BusinessProcesses" property="rdfs:label"></h3>
          </div>
          <div class="text-end"> 
            <strong about="v-bpa:TotalTimeEffort" property="rdfs:label"></strong>
            <p class="text-muted mb-0">
              ${this.clusters.reduce((acc, [,totalTime]) => acc + totalTime, 0)}&nbsp;<span about="v-bpa:HoursPerYear" property="rdfs:label"></span>
            </p>
          </div>
        </div>
        <hr>
        ${this.clusters.map(([clusterId, totalTime]) => html`
          <${ProcessCluster} about=${clusterId} data-total-time=${totalTime}></${ProcessCluster}>
        `).join('')}
      </div>
    `;
  }
}

customElements.define(ProcessClusterList.tag, ProcessClusterList);
