import {Component, html, Backend, Model} from 'veda-client';
import ProcessCluster from './ProcessCluster.js';

export default class ProcessClusterList extends Component(HTMLElement) {
  static tag = 'bpa-process-cluster-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllProcessClusters';
    const {id: clusterIds} = await Backend.stored_query(params);
    const clusters = await Promise.all(clusterIds.map((id) => new Model(id)));
    this.clusters = clusters;
  }
  
  async render() {
    return html`
      <div class="sheet">
        <h3>Бизнес-процессы</h3>
        <hr>
        ${this.clusters.map(cluster => html`
          <${ProcessCluster} about=${cluster.id}></${ProcessCluster}>
        `).join('')}
      </div>
    `;
  }
}

customElements.define(ProcessClusterList.tag, ProcessClusterList);
