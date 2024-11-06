import {Component, html, Model} from 'veda-client';
import ProcessList from './ProcessList.js';
import ClusterList from './ClusterList.js';

export default class ProcessOverview extends Component(HTMLElement) {
  static tag = 'bpa-process-overview';

  showClusters = localStorage.getItem('ProcessOverview_showClusters') === 'true';

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

  async addProcess() {
    const newProcess = new Model();
    newProcess['rdf:type'] = 'v-bpa:BusinessProcess';
    location.hash = `#/ProcessEdit/${newProcess.id}`;
  }

  render() {
    return html`
      <div class="mb-2 ms-3 d-flex justify-content-between">
        <ul class="nav nav-underline">
          <li class="nav-item">
            <button @click="toggleView" class="nav-link ${!this.showClusters ? 'active disabled' : 'text-secondary-emphasis'}">
              <span about="v-bpa:ShowProcesses" property="rdfs:label"></span>
            </button>
          </li>
          <li class="nav-item">
            <button @click="toggleView" class="nav-link ${this.showClusters ? 'active disabled' : 'text-secondary-emphasis'}">
              <span about="v-bpa:ShowClusters" property="rdfs:label"></span>
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
