import {Component, html, Model, Backend} from 'veda-client';
import ProcessList from './ProcessList.js';
import ClusterList from './ClusterList.js';

export default class ProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  showClusters = localStorage.getItem('ProcessOverview_showClusters') === 'true';
  
  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:OverallCounts';
    params['v-s:resultFormat'] = 'cols';
    const {clusters: [clustersCount], processes: [processesCount]} = await Backend.stored_query(params);
    this.clustersCount = clustersCount;
    this.processesCount = processesCount;
  }
  
  toggleView() {
    this.showClusters = !this.showClusters;
    localStorage.setItem('ProcessOverview_showClusters', this.showClusters);
    this.update();
  }

  async updateClusters() {
    const clusterizationAttempt = new Model();
    clusterizationAttempt['rdf:type'] = 'v-bpa:ClusterizationAttempt';
    await clusterizationAttempt.save();
  }

  addProcess() {
    location.hash = `#/ProcessQuickCreate`;
  }

  render() {
    return html`
      <div class="mb-2 ms-3 d-flex justify-content-between">
        <ul class="nav nav-underline">
          <li class="nav-item">
            <button @click="toggleView" class="nav-link ${!this.showClusters ? 'active disabled' : 'text-secondary-emphasis'}">
              <span class="me-2" about="v-bpa:ShowProcesses" property="rdfs:label"></span>
              <span class="align-top badge rounded-pill bg-secondary">${this.processesCount}</span>
            </button>
          </li>
          <li class="nav-item">
            <button @click="toggleView" class="nav-link ${this.showClusters ? 'active disabled' : 'text-secondary-emphasis'}">
              <span class="me-2" about="v-bpa:ShowClusters" property="rdfs:label"></span>
              <span class="align-top badge rounded-pill bg-secondary">${this.clustersCount}</span>
            </button>
          </li>
        </ul>
        ${this.showClusters 
          ? html`<button class="btn text-dark" @click="updateClusters"><i class="bi bi-arrow-repeat"></i>&nbsp;<span about="v-bpa:UpdateClusters" property="rdfs:label"></span></button>`
          : html`<button class="btn text-dark" @click="addProcess"><i class="bi bi-plus"></i>&nbsp;<span about="v-bpa:AddProcess" property="rdfs:label"></span></button>`
        }
      </div>
      ${this.showClusters 
        ? html`<${ClusterList}></${ClusterList}>` 
        : html`<${ProcessList}></${ProcessList}>`
      }
    `;
  }
}

customElements.define(ProcessOverview.tag, ProcessOverview);
