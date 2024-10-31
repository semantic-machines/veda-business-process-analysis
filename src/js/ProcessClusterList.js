import {Component, html, Backend, Model} from 'veda-client';
import ProcessCluster from './ProcessCluster.js';

export default class ProcessClusterList extends Component(HTMLElement) {
  static tag = 'bpa-process-cluster-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessClusters';
    const {rows: clusters} = await Backend.stored_query(params);
    this.clusters = clusters;
  }
  
  async render() {
    return html`
      <div class="sheet">
        <h3>Бизнес-процессы</h3>
        <hr>
        ${this.clusters.map(([clusterId, totalTime]) => html`
          <${ProcessCluster} about=${clusterId} data-total-time=${totalTime}></${ProcessCluster}>
        `).join('')}
      </div>
    `;
  }
}

customElements.define(ProcessClusterList.tag, ProcessClusterList);
